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

use std::net::Ipv4Addr;

use actix_web::web::Json;
use apistos::actix::CreatedJson;
use apistos::api_operation;
use rand::prelude::IndexedRandom;
use uuid::Uuid;
use validator::Validate;

use crate::config;
use crate::database::host_table;
use crate::database::host_table::HostEntry;
use crate::database::meta_virtual_machine_table;
use crate::database::address_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_api_structs::virtual_machine_structs::*;
use ainari_clients::endpoints::*;
use ainari_clients::network_interface::*;
use ainari_clients::proxy as proxy_clients;
use ainari_clients::quota::get_quota;
use ainari_clients::virtual_machine as virtual_machine_clients;

#[api_operation(
    tag = "virtual_machine",
    summary = "Create new virtual_machine",
    description = r###"Create new virtual_machine."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn create_virtual_machine(
    body: Json<VirtualMachineCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<VirtualMachineResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    check_quota(&context).await?;

    let selected_host = select_host(&context)?;
    let (virtual_machine_resp, proxy_uuid) =
        prepare_selected_host(&selected_host, &body, &context).await?;

    // parse uuid-string
    let sakura_uuid = convert_uuid(&selected_host.uuid)?;

    // add new virtual_machine to database
    let virtual_machine_uuid = virtual_machine_resp.uuid;
    meta_virtual_machine_table::add_new_meta_virtual_machine(
        &virtual_machine_uuid,
        &body.name,
        &sakura_uuid,
        &proxy_uuid,
        &context,
    )
    .map_err(|e| {
        log::error!(
            "Failed to add virtual_machine with UUID '{virtual_machine_uuid}' to database with error: {e}."
        );
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    Ok(CreatedJson(virtual_machine_resp))
}

fn select_host(context: &UserContext) -> Result<HostEntry, ErrorResponse> {
    // list all avaialble hosts
    let hosts = host_table::list_hosts(&context).map_err(|e| {
        log::error!("Failed to get list of hosts form database: '{e}'");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    // check that there is at least one host
    if hosts.is_empty() {
        log::error!("No hosts to schedule new virtual_machine.");
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

    Ok(selected_host.clone())
}

async fn prepare_selected_host(
    selected_host: &HostEntry,
    body: &Json<VirtualMachineCreateReq>,
    context: &UserContext,
) -> Result<(VirtualMachineResp, Uuid), ErrorResponse> {
    let root_disk_path = Some("/tmp/ubuntu-24.04.raw".to_string());
    let seed_path = "/tmp/seed.iso".to_string();
    let internal_ip = Ipv4Addr::new(192, 168, 100, 2);
    let tap_name = "tap-vm".to_string();
    let mac_address = "02:00:00:00:00:42".to_string();

    //address_table::reserve_new_address()

    // send request to the selected sakura-host to create a virtual_machine
    let mut virtual_machine_resp = virtual_machine_clients::create_virtual_machine(
        &selected_host.address,
        &context.token,
        &config::INTERNAL_API_KEY,
        &body.name,
        body.number_of_cores,
        body.memory_size,
        root_disk_path,
        &seed_path,
        &internal_ip,
        &tap_name,
        &mac_address,
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
        &virtual_machine_resp.uuid,
        &selected_host.address,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    virtual_machine_resp.torii_port = proxy_resp.port;

    Ok((virtual_machine_resp, proxy_resp.uuid))
}

/// Asynchronously checks if the user's current number of meta_virtual_machines is within their quota limit.
///
/// This function performs two main operations:
/// 1. Counts the current number of meta_virtual_machines for the given user
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
/// - Database error when counting meta_virtual_machines
/// - Network error when communicating with the Miko endpoint
/// - If the user has exceeded their meta_virtual_machine quota limit
async fn check_quota(context: &UserContext) -> Result<(), ErrorResponse> {
    // Get the current number of meta_virtual_machines for the user from the database
    // This count is used to compare against the user's quota limit
    let current_number_of_meta_virtual_machines =
        meta_virtual_machine_table::count_meta_virtual_machines(context).map_err(|e| {
            log::error!("Failed to count meta_virtual_machines in database.: {e}");
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

    // Convert the quota's maximum virtual_machine count to i64 for comparison
    let max_number_of_meta_virtual_machines = quota.max_virtual_machine as i64;

    // Check if the user has already exceeded their quota
    // If exceeded, return a Conflict error response
    if current_number_of_meta_virtual_machines as i64 >= max_number_of_meta_virtual_machines {
        return Err(ErrorResponse::Conflict(
            "Maximum number of meta_virtual_machines exceeded.".to_string(),
        ));
    }

    // If all checks pass, return Ok indicating the quota is not exceeded
    Ok(())
}
