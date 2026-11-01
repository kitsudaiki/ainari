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

use awc::http::StatusCode;
use uuid::Uuid;

use ainari_api_structs::task_structs::{
    TaskBackupRestoreReq, TaskBackupSaveReq, TaskResp, VirtualMachineCreateTaskReq,
};
use ainari_api_structs::virtual_machine_structs::*;
use ainari_common::config as ainari_config;
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
/// * `network_uuid` - Uuid of the network, which the virtual_machine is attached to.
/// * `number_of_cores` - Number of cpu-cores to assign to the virtual_machine.
/// * `memory_size` - Amount of memory in bytes to assign to the virtual_machine.
/// * `disk_size` - Size of the disk in GiB of the virtual_machine.
/// * `internal_ip` - Internal address, which is assigned to the virtual_machine.
/// * `tap_name` - Name of the TAP-device, which is attached to the virtual_machine.
/// * `mac_address` - MAC-address of the network-interface of the virtual_machine.
/// * `uuid` - Optional UUID of the virtual_machine, otherwise sakura generates one
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
    disk_size: i64,
    internal_ip: &Ipv4Addr,
    tap_name: &String,
    mac_address: &str,
    uuid: Option<&Uuid>,
    insecure_client: bool,
) -> Result<VirtualMachineResp, AinariError> {
    let client = prepare_client(sakura_address, insecure_client);
    let url = format!("{sakura_address}/v1alpha/virtual_machine/internal");

    let body = VirtualMachineInternalCreateReq {
        name: name.to_owned(),
        network_uuid: *network_uuid,
        number_of_cores,
        memory_size,
        disk_size,
        internal_ip: *internal_ip,
        tap_name: tap_name.to_owned(),
        mac_address: mac_address.to_owned(),
        uuid: uuid.copied(),
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

/// Checks, if a specific virtual_machine was deleted on its sakura-host.
///
/// A virtual_machine counts as deleted, as soon as the sakura-host doesn't know it anymore and
/// answers with `404 Not Found`.
///
/// # Arguments
///
/// * `sakura_address` - The base URL of the Ainari Sakura service.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `virtual_machine_uuid` - UUID of the virtual_machine to check.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// A `Result` containing `true` if the virtual_machine is deleted and `false` if it still
/// exists, or an `AinariError` on failure.
pub async fn is_virtual_machine_deleted(
    sakura_address: &String,
    token: &String,
    internal_api_key: &Secret,
    virtual_machine_uuid: &Uuid,
    insecure_client: bool,
) -> Result<bool, AinariError> {
    let client = prepare_client(sakura_address, insecure_client);
    let url = format!("{sakura_address}/v1alpha/virtual_machine/{virtual_machine_uuid}/internal");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await
        .map_err(|e| {
            AinariError::internal(format!(
                "Error while getting virtual_machine with uuid '{virtual_machine_uuid}' : {e}"
            ))
        })?;

    match response.status() {
        StatusCode::NOT_FOUND => Ok(true),
        StatusCode::OK => Ok(false),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            Err(AinariError::Unauthorized("Invalid token".to_string()))
        }
        code => Err(AinariError::internal(format!(
            "Error while getting virtual_machine with uuid '{virtual_machine_uuid}'. \
             Got response-code: {code}"
        ))),
    }
}

/// Reserves a new virtual_machine in hanami for another component, for example to restore it from
/// a backup with its original UUID and addresses.
///
/// # Arguments
///
/// * `hanami_endpoint` - The endpoint configuration for the hanami service
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key for accessing protected endpoints
/// * `request` - Values of the new virtual_machine
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing the reserved virtual_machine, or an `AinariError`. A UUID or an address,
/// which is already used, is rejected with `Conflict`.
pub async fn reserve_virtual_machine_internal(
    hanami_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    request: &VirtualMachineReserveInternalReq,
    insecure_client: bool,
) -> Result<VirtualMachineResp, AinariError> {
    let address = hanami_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/virtual_machine/internal");

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send_json(request)
        .await;

    handle_response(response, "virtual_machine", "").await
}

/// Creates a task on the sakura-host, which saves the root-disk of a virtual_machine into a
/// backup.
///
/// # Arguments
///
/// * `sakura_address` - Internal address of the sakura-host, which runs the virtual_machine
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key for accessing protected endpoints
/// * `virtual_machine_uuid` - The virtual_machine, whose root-disk is saved
/// * `request` - The backup, the secret for the encryption and if the virtual_machine is stopped
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing the new task, which can be polled, or an `AinariError`.
pub async fn backup_virtual_machine(
    sakura_address: &str,
    token: &String,
    internal_api_key: &Secret,
    virtual_machine_uuid: &Uuid,
    request: &TaskBackupSaveReq,
    insecure_client: bool,
) -> Result<TaskResp, AinariError> {
    let client = prepare_client(sakura_address, insecure_client);
    let url =
        format!("{sakura_address}/v1alpha/virtual_machine/{virtual_machine_uuid}/backup/internal");

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send_json(request)
        .await;

    handle_response(
        response,
        "virtual_machine",
        &virtual_machine_uuid.to_string(),
    )
    .await
}

/// Gets the sakura-host, which runs a virtual_machine, from hanami.
///
/// # Arguments
///
/// * `hanami_endpoint` - The endpoint configuration for the hanami service
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key for accessing protected endpoints
/// * `virtual_machine_uuid` - The virtual_machine
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing the host with its internal address, or an `AinariError`.
pub async fn get_virtual_machine_host(
    hanami_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    virtual_machine_uuid: &Uuid,
    insecure_client: bool,
) -> Result<VirtualMachineHostResp, AinariError> {
    let address = hanami_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/virtual_machine/{virtual_machine_uuid}/host/internal");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_response(
        response,
        "virtual_machine",
        &virtual_machine_uuid.to_string(),
    )
    .await
}

/// Creates a task on the sakura-host, which creates a reserved virtual_machine from an image.
///
/// The internal API key is sent as well, so the creation also works, while the project is locked
/// by a restore.
///
/// # Arguments
///
/// * `sakura_address` - Internal address of the sakura-host, which got the virtual_machine
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key of the calling component
/// * `virtual_machine_uuid` - The reserved virtual_machine
/// * `image_uuid` - Image or snapshot, which the virtual_machine boots from
/// * `public_key_uuid` - Public-key, which is injected into the virtual_machine
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing the new task, which can be polled, or an `AinariError`.
pub async fn create_virtual_machine_task(
    sakura_address: &str,
    token: &String,
    internal_api_key: &Secret,
    virtual_machine_uuid: &Uuid,
    image_uuid: &Uuid,
    public_key_uuid: &Uuid,
    insecure_client: bool,
) -> Result<TaskResp, AinariError> {
    let client = prepare_client(sakura_address, insecure_client);
    let url = format!("{sakura_address}/v1alpha/virtual_machine/{virtual_machine_uuid}");

    let body = VirtualMachineCreateTaskReq {
        vm_uuid: *virtual_machine_uuid,
        image_uuid: *image_uuid,
        public_key_uuid: *public_key_uuid,
    };

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send_json(&body)
        .await;

    handle_response(
        response,
        "virtual_machine",
        &virtual_machine_uuid.to_string(),
    )
    .await
}

/// Creates a task on the sakura-host, which replaces the root-disk of a virtual_machine with the
/// root-disk of a backup, which was registered as snapshot-image before.
///
/// # Arguments
///
/// * `sakura_address` - Internal address of the sakura-host, which runs the virtual_machine
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key for accessing protected endpoints
/// * `virtual_machine_uuid` - The virtual_machine, whose root-disk is replaced
/// * `request` - The backup and the snapshot-image with its root-disk
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing the new task, which can be polled, or an `AinariError`.
pub async fn restore_virtual_machine_from_backup(
    sakura_address: &str,
    token: &String,
    internal_api_key: &Secret,
    virtual_machine_uuid: &Uuid,
    request: &TaskBackupRestoreReq,
    insecure_client: bool,
) -> Result<TaskResp, AinariError> {
    let client = prepare_client(sakura_address, insecure_client);
    let url = format!(
        "{sakura_address}/v1alpha/virtual_machine/{virtual_machine_uuid}/backup_restore/internal"
    );

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send_json(request)
        .await;

    handle_response(
        response,
        "virtual_machine",
        &virtual_machine_uuid.to_string(),
    )
    .await
}

/// Creates a task on the sakura-host, which shuts a virtual_machine down.
///
/// The internal API key is sent as well, so the virtual_machine can also be stopped, while the
/// project is locked by a restore.
///
/// # Arguments
///
/// * `sakura_address` - Internal address of the sakura-host, which runs the virtual_machine
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key of the calling component
/// * `virtual_machine_uuid` - The virtual_machine to stop
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing the new task, which can be polled, or an `AinariError`.
pub async fn stop_virtual_machine_task(
    sakura_address: &str,
    token: &String,
    internal_api_key: &Secret,
    virtual_machine_uuid: &Uuid,
    insecure_client: bool,
) -> Result<TaskResp, AinariError> {
    let client = prepare_client(sakura_address, insecure_client);
    let url = format!("{sakura_address}/v1alpha/virtual_machine/{virtual_machine_uuid}/stop");

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_response(
        response,
        "virtual_machine",
        &virtual_machine_uuid.to_string(),
    )
    .await
}

/// Deletes a virtual_machine over hanami, which also removes it from its sakura-host in the
/// background.
///
/// The internal API key is sent as well, so the resource can also be deleted, while the project is
/// locked by a restore, which prunes the project.
///
/// # Arguments
///
/// * `hanami_endpoint` - The endpoint configuration for the hanami service
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key of the calling component
/// * `virtual_machine_uuid` - The virtual_machine to delete
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` indicating success or an `AinariError`.
pub async fn delete_virtual_machine_in_hanami(
    hanami_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    virtual_machine_uuid: &Uuid,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let address = hanami_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/virtual_machine/{virtual_machine_uuid}");

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
