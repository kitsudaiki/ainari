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
use rand::prelude::IndexedRandom;
use validator::Validate;

use crate::config;
use crate::database::host_table;
use crate::database::meta_model_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::model_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::*;
use ainari_clients::model as model_clients;
use ainari_clients::proxy as proxy_clients;
use ainari_clients::quota::get_quota;

#[api_operation(
    tag = "model",
    summary = "Create new model",
    description = r###"Create new model based on a model-template."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn create_model(
    body: Json<ModelCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<ModelResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    check_quota(&context).await?;

    // list all avaialble hosts
    let hosts = host_table::list_hosts(&context).map_err(|e| {
        log::error!("Failed to get list of hosts form database: '{e}'");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    // check that there is at least one host
    if hosts.is_empty() {
        log::error!("No hosts to schedule new model.");
        return Err(ErrorResponse::InternalError("Internal Error".to_string()));
    }

    // select first host
    let mut rng = rand::rng();
    let selected_host = if let Some(host) = hosts.choose(&mut rng) {
        host
    } else {
        log::error!("No hosts with list-position 0 doesn't exist.");
        return Err(ErrorResponse::InternalError("Internal Error".to_string()));
    };

    // send request to the selected sakura-host to create a model
    let mut model_resp = model_clients::create_model(
        &selected_host.address,
        &context.token,
        &config::INTERNAL_API_KEY,
        &body.name,
        &body.template,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    // get endpoints from miko
    let miko_endpoint = &config::CONFIG.miko;
    let endpoints = get_endpoints(miko_endpoint, config::CONFIG.skip_tls_verification)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    // send request to torii to create a proxy
    let proxy_resp = proxy_clients::create_proxy(
        &endpoints.torii,
        &context.token,
        &config::INTERNAL_API_KEY,
        &model_resp.uuid,
        &selected_host.address,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    // set port-number for the response
    model_resp.torii_port = proxy_resp.port;

    // parse uuid-string
    let sakura_uuid = convert_uuid(&selected_host.uuid)?;

    // add new model to database
    let model_uuid = model_resp.uuid;
    meta_model_table::add_new_meta_model(
        &model_uuid,
        &body.name,
        &sakura_uuid,
        &proxy_resp.uuid,
        &context,
    )
    .map_err(|e| {
        log::error!("Failed to add model with UUID '{model_uuid}' to database with error: {e}.");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    Ok(CreatedJson(model_resp))
}

/// Asynchronously checks if the user's current number of meta_models is within their quota limit.
///
/// This function performs two main operations:
/// 1. Counts the current number of meta_models for the given user
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
/// - Database error when counting meta_models
/// - Network error when communicating with the Miko endpoint
/// - If the user has exceeded their meta_model quota limit
async fn check_quota(context: &UserContext) -> Result<(), ErrorResponse> {
    // Get the current number of meta_models for the user from the database
    // This count is used to compare against the user's quota limit
    let current_number_of_meta_models =
        meta_model_table::count_meta_models(context).map_err(|e| {
            log::error!("Failed to count meta_models in database.: {e}");
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

    // Convert the quota's maximum model count to i64 for comparison
    let max_number_of_meta_models = quota.max_model as i64;

    // Check if the user has already exceeded their quota
    // If exceeded, return a Conflict error response
    if current_number_of_meta_models as i64 >= max_number_of_meta_models {
        return Err(ErrorResponse::Conflict(
            "Maximum number of meta_models exceeded.".to_string(),
        ));
    }

    // If all checks pass, return Ok indicating the quota is not exceeded
    Ok(())
}
