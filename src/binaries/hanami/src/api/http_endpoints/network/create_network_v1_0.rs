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
use uuid::Uuid;
use validator::Validate;

use crate::config;
use crate::database::network_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::quota::get_quota;

#[api_operation(
    tag = "network",
    summary = "Create new network",
    description = r###"Create new network.

The traffic between the virtual machines of the network, which run on different hosts, is
encrypted with IPsec, whose keys the gateways exchange over a MLS-group of the network. With
`disable_encryption` set, the network gets neither the encryption nor the MLS-group. The flag can't
be changed after the creation."###,
    error_code = 400,
    error_code = 401,
    error_code = 409,
    error_code = 500
)]
pub async fn create_network(
    body: Json<NetworkCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<NetworkResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    check_quota(&context).await?;

    let network_uuid = Uuid::new_v4();

    // add new network to database
    network_table::add_new_network(
        &network_uuid,
        &body.name,
        &body.subnet,
        body.disable_encryption,
        &context,
    )
    .map_err(|e| {
        map_db_write_error(
            &format!("add network with UUID '{network_uuid}' to database"),
            e,
        )
    })?;

    // get new created network from database to get additional information
    let network = network_table::get_network(&network_uuid, &context)
        .map_err(|e| map_db_uuid_get_after_add_error("project", &network_uuid, e))?;

    let resp = NetworkResp {
        uuid: network_uuid,
        name: network.name,
        subnet: network.subnet,
        disable_encryption: network.disable_encryption,
        created_by: network.created_by,
        created_at: network.created_at,
        updated_by: network.updated_by,
        updated_at: network.updated_at,
    };

    Ok(CreatedJson(resp))
}

/// Asynchronously checks if the project's current number of networks is within its quota limit.
///
/// This function performs two main operations:
/// 1. Counts the current number of networks for the project of the context
/// 2. Retrieves the project's quota from the Miko endpoint and verifies if the quota is exceeded
///
/// # Arguments
///
/// * `context` - A reference to the UserContext containing authentication and user information
///
/// # Returns
///
/// * `Ok(())` - If the quota check passes (project is within its limit)
/// * `Err(ErrorResponse)` - If there's an error during the check or if the quota is exceeded
///
/// # Errors
///
/// This function will return an error in the following cases:
/// - Database error when counting networks
/// - Network error when communicating with the Miko endpoint
/// - If the project has exceeded its network quota limit
async fn check_quota(context: &UserContext) -> Result<(), ErrorResponse> {
    // Get the current number of networks of the whole project from the database
    // This count is used to compare against the project's quota limit
    let current_number_of_networks = network_table::count_networks_of_project(&context.project_id)
        .map_err(|e| {
            log::error!("Failed to count networks in database.: {e}");
            ErrorResponse::InternalError("Internal Error".to_string())
        })?;

    // Retrieve the project's quota information from the Miko endpoint
    // The miko_endpoint is configured in the application settings
    let miko_endpoint = &config::CONFIG.miko;
    let quota = get_quota(
        miko_endpoint,
        &context.token,
        &context.project_id,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    // Convert the quota's maximum network count to i64 for comparison
    let max_number_of_networks = quota.max_network as i64;

    // Check if the project has already exceeded its quota
    // If exceeded, return a Conflict error response
    if current_number_of_networks as i64 >= max_number_of_networks {
        return Err(ErrorResponse::Conflict(
            "Maximum number of networks exceeded.".to_string(),
        ));
    }

    // If all checks pass, return Ok indicating the quota is not exceeded
    Ok(())
}
