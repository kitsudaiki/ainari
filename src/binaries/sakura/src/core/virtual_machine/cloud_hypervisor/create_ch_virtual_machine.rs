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

use std::fs;
use std::net::Ipv4Addr;
use std::process::Command;
use std::time::Duration;

use cloud_hypervisor_client::apis::DefaultApi;
use cloud_hypervisor_client::models::{
    ConsoleConfig, ConsoleMode, CpusConfig, DiskConfig, MemoryConfig, NetConfig, PayloadConfig,
    SerialConfig, VmConfig,
};
use cloud_hypervisor_client::socket_based_api_client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use ainari_api::common_functions::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::get_endpoints;
use ainari_clients::image::*;
use ainari_clients::onsen_file_transfer::*;
use ainari_clients::public_key::get_public_key;
use ainari_clients::secret::get_secret_payload;
use ainari_common::config::Endpoints;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;
use ainari_files::file_encryption::decrypt_file;

use super::{
    mark_error_on_failure, set_vm_state, vm_directory, vm_serial_log_path, vm_socket_path,
    vm_temp_directory,
};
use crate::config;
use crate::database::virtual_machine_table;
use crate::database::virtual_machine_table::VirtualMachineState;

/// Handle of a running cloud-hypervisor virtual_machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmHandle {
    /// Name of the TAP-device, which is attached to the virtual_machine
    pub tap_name: String,
    /// Path of the API-socket of the cloud-hypervisor process
    pub socket_path: String,
    /// Process-id of the cloud-hypervisor process
    pub pid: u32,
}

/// Creates and boots a new cloud-hypervisor virtual_machine based on its database-entry
///
/// This downloads and converts the boot-image, creates the cloud-init seed-image, starts a new
/// cloud-hypervisor process and creates and boots the virtual_machine via its API-socket. The
/// virtual_machine is marked as running afterwards, or as error, if the creation failed, because
/// it can not be started in this case.
///
/// # Arguments
/// * `uuid` - Unique identifier of the virtual_machine to create
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(VmHandle)` with the handle of the running virtual_machine on success
/// * `Err(AinariError)` with an appropriate error on failure
pub async fn create_ch_virtual_machine(
    uuid: &Uuid,
    context: &UserContext,
) -> Result<VmHandle, AinariError> {
    let result = create_and_boot_vm(uuid, context).await;
    mark_error_on_failure(uuid, context, result)
}

/// Prepares all files of the virtual_machine and boots it in a new cloud-hypervisor process
///
/// # Arguments
/// * `uuid` - Unique identifier of the virtual_machine to create
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(VmHandle)` with the handle of the running virtual_machine on success
/// * `Err(AinariError)` with an appropriate error on failure
async fn create_and_boot_vm(uuid: &Uuid, context: &UserContext) -> Result<VmHandle, AinariError> {
    let virtual_machine_data = virtual_machine_table::get_virtual_machine(uuid, context)
        .map_err(|e| map_db_uuid_get_delete_ainari_error("virtual_machine", uuid, e))?;
    let endpoints =
        get_endpoints(&config::CONFIG.miko, config::CONFIG.skip_tls_verification).await?;

    // get public-key, which is injected into the virtual_machine
    let public_key_resp = get_public_key(
        &endpoints.omamori,
        &context.token,
        &config::INTERNAL_API_KEY,
        &virtual_machine_data.public_key_uuid,
        config::CONFIG.skip_tls_verification,
    )
    .await?;

    // prepare disks of the virtual_machine
    let (vm_dir, temp_dir) = prepare_directories(&virtual_machine_data.uuid).await?;
    let root_disk_path = download_and_convert_image(
        &endpoints,
        &virtual_machine_data.image_uuid,
        &temp_dir,
        &vm_dir,
        virtual_machine_data.disk_size,
        context,
    )
    .await?;
    let seed_path = create_seed_file(
        &vm_dir,
        &public_key_resp.public_key,
        &virtual_machine_data.internal_ip,
        &virtual_machine_data.mac_address,
    )?;

    log::info!("Start creation of VM {uuid}");

    // start cloud-hypervisor process, which is controlled via its API-socket
    let socket_path = vm_socket_path(uuid);
    let _ = fs::remove_file(&socket_path);
    let mut child = Command::new(&config::CONFIG.hypervisor.binary_path)
        .arg("--api-socket")
        .arg(&socket_path)
        .spawn()
        .map_err(|e| AinariError::InternalError(format!("Failed to spawn VMM: {e}")))?;

    // give the process time to create the API-socket
    tokio::time::sleep(Duration::from_millis(500)).await;
    let client = socket_based_api_client(&socket_path);

    let vm_config = VmConfig {
        payload: PayloadConfig {
            firmware: Some(config::CONFIG.hypervisor.firmware_path.clone()),
            ..Default::default()
        },
        cpus: Some(CpusConfig {
            boot_vcpus: virtual_machine_data.number_of_cores,
            max_vcpus: virtual_machine_data.number_of_cores,
            ..Default::default()
        }),
        memory: Some(MemoryConfig {
            size: virtual_machine_data.memory_size,
            ..Default::default()
        }),
        // write the output of the serial console into a log-file
        console: Some(ConsoleConfig {
            mode: ConsoleMode::Null,
            ..Default::default()
        }),
        serial: Some(SerialConfig {
            mode: ConsoleMode::File,
            file: Some(vm_serial_log_path(uuid)),
            ..Default::default()
        }),
        // No checksum offloading: the gateways rewrite addresses with
        // incremental checksum updates, which are only correct on complete
        // checksums. With offloading the VM hands over a partial one, and on
        // a path where nobody finishes it (e.g. VM -> floating IP of another
        // VM through the host) the receiver drops the packet. TSO and UFO
        // depend on checksum offloading, so they go as well.
        net: Some(vec![NetConfig {
            tap: Some(virtual_machine_data.tap_name.clone()),
            mac: Some(virtual_machine_data.mac_address.clone()),
            offload_csum: Some(false),
            offload_tso: Some(false),
            offload_ufo: Some(false),
            ..Default::default()
        }]),
        // writable root-disk and read-only cloud-init seed-image
        disks: Some(vec![
            DiskConfig {
                path: Some(root_disk_path.clone()),
                readonly: Some(false),
                ..Default::default()
            },
            DiskConfig {
                path: Some(seed_path.clone()),
                readonly: Some(true),
                ..Default::default()
            },
        ]),
        ..Default::default()
    };

    log::info!(
        "Creating VM {uuid} attached to {}",
        virtual_machine_data.tap_name
    );
    client
        .create_vm(vm_config)
        .await
        .map_err(|e| AinariError::InternalError(format!("Create VM {uuid} failed: {e:?}")))?;

    log::info!(
        "Booting VM {uuid} attached to {}",
        virtual_machine_data.tap_name
    );
    client
        .boot_vm()
        .await
        .map_err(|e| AinariError::InternalError(format!("Boot VM {uuid} failed: {e:?}")))?;

    // Capture the PID before moving `child` into the reaper-thread. `wait` blocks, so it runs in
    // a plain thread instead of an async task to not block the async runtime.
    let vm_pid = child.id();
    std::thread::spawn(move || {
        if let Err(e) = child.wait() {
            log::error!("Failed to wait for VMM-process {vm_pid}: {e}");
        }
    });

    // the virtual_machine runs now, so its disks are stored and it is marked as running
    virtual_machine_table::update_virtual_machine(
        uuid,
        &virtual_machine_data.image_uuid,
        &virtual_machine_data.public_key_uuid,
        &seed_path,
        Some(root_disk_path),
        context,
    )
    .map_err(|e| map_db_uuid_get_delete_ainari_error("virtual_machine", uuid, e))?;
    set_vm_state(uuid, VirtualMachineState::Running, context)?;

    log::info!("New VM {uuid} started");

    Ok(VmHandle {
        tap_name: virtual_machine_data.tap_name,
        socket_path,
        pid: vm_pid,
    })
}

/// Creates the directories for the files of a virtual_machine
///
/// # Arguments
/// * `vm_uuid` - Unique identifier of the virtual_machine
///
/// # Returns
/// * `Ok((String, String))` with the paths of the vm-directory and the temp-directory on success
/// * `Err(AinariError)` with an appropriate error on failure
async fn prepare_directories(vm_uuid: &Uuid) -> Result<(String, String), AinariError> {
    let vm_dir = vm_directory(vm_uuid);
    create_directory(&vm_dir).await?;

    let temp_dir = vm_temp_directory(vm_uuid);
    create_directory(&temp_dir).await?;

    Ok((vm_dir, temp_dir))
}

/// Creates the cloud-init seed-image with the network-config and user-data of a virtual_machine
///
/// # Arguments
/// * `vm_dir` - Directory of the virtual_machine, where the seed-image is written to
/// * `public_key` - Public ssh-key, which is added to the authorized keys of the virtual_machine
/// * `internal_ip` - Internal address of the virtual_machine
/// * `mac_address` - MAC-address of the network-interface of the virtual_machine
///
/// # Returns
/// * `Ok(String)` with the path of the seed-image on success
/// * `Err(AinariError)` with an appropriate error on failure
fn create_seed_file(
    vm_dir: &str,
    public_key: &str,
    internal_ip: &Ipv4Addr,
    mac_address: &str,
) -> Result<String, AinariError> {
    let network_config_path = format!("{vm_dir}/network-config");
    let user_data_path = format!("{vm_dir}/user-data");
    let seed_path = format!("{vm_dir}/seed.iso");

    // the gateway is the first address of the /24 network of the virtual_machine
    let [a, b, c, _] = internal_ip.octets();
    let gateway = Ipv4Addr::new(a, b, c, 1);

    // json-strings are valid double-quoted yaml-strings, so this escapes the values properly
    let quoted_mac_address = serde_json::to_string(mac_address)
        .map_err(|e| AinariError::InternalError(format!("Failed to quote mac-address: {e}")))?;
    let quoted_public_key = serde_json::to_string(public_key.trim())
        .map_err(|e| AinariError::InternalError(format!("Failed to quote public-key: {e}")))?;

    let network_config = format!(
        "version: 2
ethernets:
  eth0:
    match:
      macaddress: {quoted_mac_address}
    addresses:
      - {internal_ip}/24
    gateway4: {gateway}
    nameservers:
      addresses: [8.8.8.8]
"
    );

    let user_data = format!(
        "#cloud-config
password: ubuntu
chpasswd: {{ expire: False }}
ssh_pwauth: True
ssh_authorized_keys:
  - {quoted_public_key}
"
    );

    fs::write(&network_config_path, network_config).map_err(|e| {
        AinariError::InternalError(format!("Failed to write {network_config_path}: {e}"))
    })?;
    fs::write(&user_data_path, user_data).map_err(|e| {
        AinariError::InternalError(format!("Failed to write {user_data_path}: {e}"))
    })?;

    // combine network-config and user-data into the seed-image
    if let Err(e) = run_command(
        "cloud-localds",
        &["-N", &network_config_path, &seed_path, &user_data_path],
    ) {
        let _ = fs::remove_file(&seed_path);
        return Err(e);
    }

    Ok(seed_path)
}

/// Gets the secret of an image, which is required to decrypt the image-file
///
/// # Arguments
/// * `endpoints` - Endpoints of the other components
/// * `secret_uuid` - Unique identifier of the secret
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(Secret)` with the secret on success
/// * `Err(AinariError)` with an appropriate error on failure
pub(super) async fn get_secret(
    endpoints: &Endpoints,
    secret_uuid: &Uuid,
    context: &UserContext,
) -> Result<Secret, AinariError> {
    let secret_payload = get_secret_payload(
        &endpoints.omamori,
        &context.token,
        secret_uuid,
        config::CONFIG.skip_tls_verification,
    )
    .await?;

    Ok(Secret::from(secret_payload.secret_payload))
}

/// Downloads an image from onsen, decrypts it and converts it into a raw boot-disk
///
/// # Arguments
/// * `endpoints` - Endpoints of the other components
/// * `image_uuid` - Unique identifier of the image to download
/// * `temp_dir` - Directory for the intermediate encrypted and decrypted image-files
/// * `target_dir` - Directory where the converted boot-disk is written to
/// * `disk_size` - Size in GiB, by which the boot-disk is increased
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(String)` with the path of the converted boot-disk on success
/// * `Err(AinariError)` with an appropriate error on failure
async fn download_and_convert_image(
    endpoints: &Endpoints,
    image_uuid: &Uuid,
    temp_dir: &str,
    target_dir: &str,
    disk_size: i64,
    context: &UserContext,
) -> Result<String, AinariError> {
    // get image information
    let image_resp = get_image(
        &endpoints.ryokan,
        &context.token,
        &config::INTERNAL_API_KEY,
        image_uuid,
        config::CONFIG.skip_tls_verification,
    )
    .await?;

    // create file-paths
    let local_file_path = format!("{temp_dir}/{}", image_resp.uuid);
    let local_encrypted_file_path = format!("{local_file_path}_encrypted");
    let local_converted_file_path = format!("{target_dir}/boot_disk");

    // download encrypted image
    download_file(
        &image_resp.onsen_address,
        &image_resp.file_path,
        &local_encrypted_file_path,
    )
    .await
    .map_err(|e| {
        let _ = fs::remove_file(&local_encrypted_file_path);
        AinariError::InternalError(format!("Failed to download image-file from onsen: {e}"))
    })?;

    // decrypt image
    let decrypt_result = match get_secret(endpoints, &image_resp.secret_uuid, context).await {
        Ok(secret) => decrypt_file(&local_encrypted_file_path, &local_file_path, &secret).await,
        Err(e) => Err(e),
    };

    // delete encrypted file again
    let _ = fs::remove_file(&local_encrypted_file_path);
    if let Err(e) = decrypt_result {
        let _ = fs::remove_file(&local_file_path);
        return Err(e);
    }

    // convert image into raw boot-disk and delete decrypted file again
    let convert_result = convert_image(&local_file_path, &local_converted_file_path, disk_size);
    let _ = fs::remove_file(&local_file_path);
    if let Err(e) = convert_result {
        let _ = fs::remove_file(&local_converted_file_path);
        return Err(e);
    }

    Ok(local_converted_file_path)
}

/// Converts a qcow2-image into a raw-image and increases its size by the given disk-size
///
/// # Arguments
/// * `input_path` - Path of the qcow2-image
/// * `output_path` - Path of the new raw-image
/// * `disk_size` - Size in GiB, by which the raw-image is increased
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(AinariError)` with an appropriate error on failure
fn convert_image(input_path: &str, output_path: &str, disk_size: i64) -> Result<(), AinariError> {
    run_command(
        "qemu-img",
        &[
            "convert",
            "-f",
            "qcow2",
            "-O",
            "raw",
            input_path,
            output_path,
        ],
    )?;
    run_command(
        "qemu-img",
        &["resize", output_path, &format!("+{disk_size}G")],
    )
}

/// Runs an external command and waits until it is finished
///
/// # Arguments
/// * `program` - Name or path of the program to run
/// * `args` - Arguments for the program
///
/// # Returns
/// * `Ok(())` if the command was successful
/// * `Err(AinariError)` if the command could not be started or failed
pub(super) fn run_command(program: &str, args: &[&str]) -> Result<(), AinariError> {
    let status = Command::new(program).args(args).status().map_err(|e| {
        AinariError::InternalError(format!("Failed to execute {program} process: {e}"))
    })?;

    if status.success() {
        Ok(())
    } else {
        Err(AinariError::InternalError(format!(
            "{program} failed with exit status: {status}"
        )))
    }
}
