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
    description = r###"Create new network."###,
    error_code = 400,
    error_code = 401,
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

    // add new network to datbase
    network_table::add_new_network(&network_uuid, &body.name, &body.subnet, &context).map_err(
        |e| {
            log::error!("Failed to add network with UUID '{network_uuid}' to database.: {e}");
            ErrorResponse::InternalError("Internal Error".to_string())
        },
    )?;

    // get new created network from database to get addtional information
    let network = network_table::get_network(&network_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("project", &network_uuid, e))?;

    let resp = NetworkResp {
        uuid: network_uuid,
        name: network.name,
        subnet: network.subnet,
        created_by: network.created_by,
        created_at: network.created_at,
        updated_by: network.updated_by,
        updated_at: network.updated_at,
    };

    Ok(CreatedJson(resp))
}

/// Asynchronously checks if the user's current number of networks is within their quota limit.
///
/// This function performs two main operations:
/// 1. Counts the current number of networks for the given user
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
/// - Database error when counting networks
/// - Network error when communicating with the Miko endpoint
/// - If the user has exceeded their network quota limit
async fn check_quota(context: &UserContext) -> Result<(), ErrorResponse> {
    // Get the current number of networks for the user from the database
    // This count is used to compare against the user's quota limit
    let current_number_of_networks = network_table::count_networks(context).map_err(|e| {
        log::error!("Failed to count networks in database.: {e}");
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

    // Convert the quota's maximum network count to i64 for comparison
    let max_number_of_networks = quota.max_network as i64;

    // Check if the user has already exceeded their quota
    // If exceeded, return a Conflict error response
    if current_number_of_networks as i64 >= max_number_of_networks {
        return Err(ErrorResponse::Conflict(
            "Maximum number of networks exceeded.".to_string(),
        ));
    }

    // If all checks pass, return Ok indicating the quota is not exceeded
    Ok(())
}
