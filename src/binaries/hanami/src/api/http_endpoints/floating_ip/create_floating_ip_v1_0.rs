// Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use actix_web::web::Json;
use apistos::actix::CreatedJson;
use apistos::api_operation;
use std::net::Ipv4Addr;
use validator::Validate;

use crate::config;
use crate::database::floating_ip_table;
use crate::database::floating_ip_table::FloatingIpEntry;
use crate::database::floating_ip_table::FloatingIpReserveError;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::floating_ip_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::get_endpoints;
use ainari_clients::floating_ip as floating_ip_clients;
use ainari_clients::quota::get_quota;

#[api_operation(
    tag = "floating_ip",
    summary = "Create new floating_ip",
    description = r###"Create new floating_ip. If no floating ip-address is requested, a free one is selected."###,
    error_code = 400,
    error_code = 401,
    error_code = 409,
    error_code = 500
)]
pub async fn create_floating_ip(
    body: Json<FloatingIpCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<FloatingIpResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    check_quota(&context).await?;

    // add new floating_ip to datbase and reserve the requested or a free floating ip-address for it
    let (floating_ip_uuid, _) = floating_ip_table::add_new_floating_ip(
        &body.network_uuid,
        &body.internal_ip,
        body.floating_ip.as_ref(),
        &config::CONFIG.network.floating_ip_cidr,
        &context,
    )
    .map_err(|e| map_reserve_error(e, body.floating_ip.as_ref()))?;

    // get new created floating_ip from database to get addtional information
    let floating_ip_entrry = floating_ip_table::get_floating_ip(&floating_ip_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("project", &floating_ip_uuid, e))?;

    // register the NAT of the floating ip-address in the torii, which is reachable from outside
    register_floating_ip(&floating_ip_entrry, &body.name, &context).await?;

    let resp = FloatingIpResp {
        uuid: floating_ip_uuid,
        network_uuid: floating_ip_entrry.network_uuid,
        floating_ip: floating_ip_entrry.floating_ip_addr,
        internal_ip: floating_ip_entrry.internal_ip_addr,
        created_by: floating_ip_entrry.created_by,
        created_at: floating_ip_entrry.created_at,
        updated_by: floating_ip_entrry.updated_by,
        updated_at: floating_ip_entrry.updated_at,
    };

    Ok(CreatedJson(resp))
}

/// Registers the NAT of a floating ip-address in the torii
///
/// The floating ip-address is translated by the torii, which is reachable from the outside,
/// because that is where the traffic of the virtual_machines enters and leaves the virtual
/// network. If the registration fails, the reserved address is released again, so it is not
/// blocked by an entry, which the torii doesn't know.
///
/// # Arguments
/// * `floating_ip_entry` - Reserved floating ip-address with its internal address
/// * `name` - Name of the floating ip-address
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` if the floating ip-address is registered in the torii
/// * `Err(ErrorResponse)` with an appropriate error on failure
async fn register_floating_ip(
    floating_ip_entry: &FloatingIpEntry,
    name: &str,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    let result = async {
        // get endpoints from miko
        let endpoints = get_endpoints(&config::CONFIG.miko, config::CONFIG.skip_tls_verification)
            .await
            .map_err(map_ainari_error_to_api_response)?;

        floating_ip_clients::create_floating_ip(
            &endpoints.torii,
            &context.token,
            &config::INTERNAL_API_KEY,
            name,
            &floating_ip_entry.network_uuid,
            &floating_ip_entry.floating_ip_addr,
            &floating_ip_entry.internal_ip_addr,
            config::CONFIG.skip_tls_verification,
        )
        .await
        .map_err(map_ainari_error_to_api_response)
    }
    .await;

    if let Err(e) = result {
        log::error!(
            "Failed to register floating ip '{}' in torii. Releasing it again.",
            floating_ip_entry.floating_ip_addr
        );
        let _ = floating_ip_table::force_delete_floating_ip(&floating_ip_entry.uuid);
        return Err(e);
    }

    Ok(())
}

/// Converts an error of the floating ip-address reservation into an error-response.
///
/// # Arguments
///
/// * `error` - The error of the reservation
/// * `requested_ip` - The floating ip-address, which was requested by the user, if any
///
/// # Returns
///
/// The error-response, which is sent back to the user
fn map_reserve_error(
    error: FloatingIpReserveError,
    requested_ip: Option<&Ipv4Addr>,
) -> ErrorResponse {
    let requested = requested_ip.map(|ip| ip.to_string()).unwrap_or_default();
    match error {
        FloatingIpReserveError::NotInRange => ErrorResponse::BadRequest(format!(
            "Floating ip '{requested}' is not within the range '{}'.",
            config::CONFIG.network.floating_ip_cidr
        )),
        FloatingIpReserveError::AlreadyUsed => {
            ErrorResponse::Conflict(format!("Floating ip '{requested}' is already used."))
        }
        FloatingIpReserveError::NoFreeAddress => {
            ErrorResponse::Conflict("No free floating ip left.".to_string())
        }
        FloatingIpReserveError::InvalidCidr | FloatingIpReserveError::InternalError => {
            log::error!("Failed to add new floating_ip to database.");
            ErrorResponse::InternalError("Internal Error".to_string())
        }
    }
}

/// Asynchronously checks if the user's current number of floating_ips is within their quota limit.
///
/// This function performs two main operations:
/// 1. Counts the current number of floating_ips for the given user
/// 2. Retrieves the user's quota from the Miko endpoint and verifies if the quota is exceeded
///
/// # Arguments
///
/// * `context` - A reference to the UserContext containing authentication and user information
///
/// # Returns
///
/// * `Ok(())` - If the quota check passes (user is within their limit)
/// * `Err(ErrorResponse)` - If there's an error during the check or if the quota is exceeded
///
/// # Errors
///
/// This function will return an error in the following cases:
/// - Database error when counting floating_ips
/// - FloatingIp error when communicating with the Miko endpoint
/// - If the user has exceeded their floating_ip quota limit
async fn check_quota(context: &UserContext) -> Result<(), ErrorResponse> {
    // Get the current number of floating_ips for the user from the database
    // This count is used to compare against the user's quota limit
    let current_number_of_floating_ips =
        floating_ip_table::count_floating_ips(context).map_err(|e| {
            log::error!("Failed to count floating_ips in database.: {e}");
            ErrorResponse::InternalError("Internal Error".to_string())
        })?;

    // Retrieve the user's quota information from the Miko endpoint
    // The miko_endpoint is configured in the application settings
    let miko_endpoint = &config::CONFIG.miko;
    let quota = get_quota(
        miko_endpoint,
        &context.token,
        &context.user_id,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    // Convert the quota's maximum floating_ip count to i64 for comparison
    let max_number_of_floating_ips = quota.max_floating_ip as i64;

    // Check if the user has already exceeded their quota
    // If exceeded, return a Conflict error response
    if current_number_of_floating_ips as i64 >= max_number_of_floating_ips {
        return Err(ErrorResponse::Conflict(
            "Maximum number of floating_ips exceeded.".to_string(),
        ));
    }

    // If all checks pass, return Ok indicating the quota is not exceeded
    Ok(())
}
