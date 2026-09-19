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

use uuid::Uuid;

use ainari_api_structs::virtual_machine_structs::*;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::prepare_client;
use crate::{handle_empty_response, handle_response};

/// Creates a new virtual_machine in the Ainari system.
///
/// # Arguments
///
/// * `sakura_address` - The base URL of the Ainari Sakura service.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `name` - Name of the virtual_machine to be created.
/// * `number_of_cores` - Number of cpu-cores to assign to the virtual_machine.
/// * `memory_size` - Amount of memory in bytes to assign to the virtual_machine.
/// * `root_disk_path` - Optional path to the root-disk-image of the virtual_machine.
/// * `seed_path` - Path to the cloud-init seed-image of the virtual_machine.
/// * `internal_ip` - Internal address, which is assigned to the virtual_machine.
/// * `tap_name` - Name of the TAP-device, which is attached to the virtual_machine.
/// * `mac_address` - MAC-address of the network-interface of the virtual_machine.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// A `Result` containing the created `VirtualMachineResp` on success, or an `AinariError` on failure.
#[allow(clippy::too_many_arguments)]
pub async fn create_virtual_machine(
    sakura_address: &String,
    token: &String,
    internal_api_key: &Secret,
    name: &str,
    network_uuid: &Uuid,
    number_of_cores: i32,
    memory_size: i64,
    internal_ip: &Ipv4Addr,
    tap_name: &String,
    mac_address: &str,
    insecure_client: bool,
) -> Result<VirtualMachineResp, AinariError> {
    let client = prepare_client(sakura_address, insecure_client);
    let url = format!("{sakura_address}/v1alpha/virtual_machine/internal");

    let body = VirtualMachineInternalCreateReq {
        name: name.to_owned(),
        network_uuid: *network_uuid,
        number_of_cores,
        memory_size,
        internal_ip: *internal_ip,
        tap_name: tap_name.to_owned(),
        mac_address: mac_address.to_owned(),
    };
    let json_str = serde_json::to_string(&body).unwrap();

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    // Handle the response and return the result
    let resp: Result<VirtualMachineResp, AinariError> =
        handle_response(response, "virtual_machine", "").await;
    resp
}

/// Retrieves information about a specific virtual_machine from the Ainari system.
///
/// # Arguments
///
/// * `sakura_address` - The base URL of the Ainari Sakura service.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `virtual_machine_uuid` - UUID of the virtual_machine to retrieve.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// A `Result` containing the retrieved `VirtualMachineResp` on success, or an `AinariError` on failure.
pub async fn get_virtual_machine(
    sakura_address: &String,
    token: &String,
    internal_api_key: &Secret,
    virtual_machine_uuid: &Uuid,
    insecure_client: bool,
) -> Result<VirtualMachineResp, AinariError> {
    let client = prepare_client(sakura_address, insecure_client);
    let url = format!("{sakura_address}/v1alpha/virtual_machine/{virtual_machine_uuid}/internal");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    let resp: Result<VirtualMachineResp, AinariError> = handle_response(
        response,
        "virtual_machine",
        &virtual_machine_uuid.to_string(),
    )
    .await;
    resp
}

/// Lists all virtual_machines available in the Ainari system.
///
/// # Arguments
///
/// * `sakura_address` - The base URL of the Ainari Sakura service.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// A `Result` containing the list of virtual_machines as `VirtualMachineListResp` on success, or an `AinariError` on failure.
pub async fn list_virtual_machine(
    sakura_address: &String,
    token: &String,
    internal_api_key: &Secret,
    insecure_client: bool,
) -> Result<VirtualMachineListResp, AinariError> {
    let client = prepare_client(sakura_address, insecure_client);
    let url = format!("{sakura_address}/v1alpha/virtual_machine/internal");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    let resp: Result<VirtualMachineListResp, AinariError> =
        handle_response(response, "virtual_machine", "").await;
    resp
}

/// Deletes a specific virtual_machine from the Ainari system.
///
/// # Arguments
///
/// * `sakura_address` - The base URL of the Ainari Sakura service.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `virtual_machine_uuid` - UUID of the virtual_machine to delete.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// A `Result` indicating success or failure. On success, returns `Ok(())`.
pub async fn delete_virtual_machine(
    sakura_address: &String,
    token: &String,
    internal_api_key: &Secret,
    virtual_machine_uuid: &Uuid,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let client = prepare_client(sakura_address, insecure_client);
    let url = format!("{sakura_address}/v1alpha/virtual_machine/{virtual_machine_uuid}/internal");

    let response = client
        .delete(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_empty_response(
        response,
        "virtual_machine",
        &virtual_machine_uuid.to_string(),
    )
    .await
}
