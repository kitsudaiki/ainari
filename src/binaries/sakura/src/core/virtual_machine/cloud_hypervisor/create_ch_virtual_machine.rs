use cloud_hypervisor_client::apis::DefaultApi;
use cloud_hypervisor_client::models::{
    ConsoleConfig, ConsoleMode, CpusConfig, DiskConfig, MemoryConfig, NetConfig, PayloadConfig,
    SerialConfig, VmConfig,
};
use cloud_hypervisor_client::socket_based_api_client;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::Duration;
use uuid::Uuid;

use ainari_common::error::AinariError;

use ainari_clients::root_wrap::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmHandle {
    pub tap_name: String,
    pub socket_path: String,
    pub pid: u32,
}

async fn create_tap_device(name: &str, ip_cidr: Option<&str>) -> Result<(), AinariError> {
    let mut neko_client = init_neko_root_wrapper_client().await?;

    // create tap-device
    run_root_cmd(
        &mut neko_client,
        "ip",
        &["tuntap", "add", "mode", "tap", name],
    )
    .await
    .map_err(|e| AinariError::InternalError(format!("Failed to create TAP: {}", e)))?;

    // bring tap-device up
    run_root_cmd(&mut neko_client, "ip", &["link", "set", name, "up"])
        .await
        .map_err(|e| AinariError::InternalError(format!("Failed to bring TAP up: {}", e)))?;

    // disable offloading
    run_root_cmd(
        &mut neko_client,
        "ethtool",
        &["-K", name, "tx", "off", "rx", "off"],
    )
    .await
    .map_err(|e| AinariError::InternalError(format!("Failed to disable offloading: {}", e)))?;

    // assign IP CIDR to tap-device
    if let Some(ip) = ip_cidr {
        run_root_cmd(&mut neko_client, "ip", &["addr", "add", ip, "dev", name])
            .await
            .map_err(|e| AinariError::InternalError(format!("Failed to assign IP: {}", e)))?;
    }

    Ok(())
}

pub async fn create_ch_virtual_machine(
    uuid: &Uuid,
    number_of_cores: i32,
    memory_size: i64,
    root_disk_path: Option<String>,
    seed_path: &str,
    tap_name: &String,
    mac_address: &str,
) -> Result<VmHandle, AinariError> {
    create_tap_device(tap_name, Some("192.168.100.1/24")).await?;
    log::info!("Start creation of VM {uuid}");

    let socket_path = format!("/tmp/cloud-hypervisor-{}.sock", uuid);
    let _ = std::fs::remove_file(&socket_path);

    let vmm_process = Command::new("/tmp/cloud-hypervisor")
        .arg("--api-socket")
        .arg(&socket_path)
        .spawn();

    let mut child = match vmm_process {
        Ok(c) => c,
        Err(e) => {
            return Err(AinariError::InternalError(format!(
                "Failed to spawn VMM: {}",
                e
            )));
        }
    };

    tokio::time::sleep(Duration::from_millis(500)).await;

    let client = socket_based_api_client(&socket_path);

    let payload = PayloadConfig {
        firmware: Some(String::from("/tmp/CLOUDHV.fd")),
        ..Default::default()
    };

    let (disk_path, _has_root_disk) = if let Some(disk_path) = root_disk_path {
        (disk_path, true)
    } else {
        ("".to_string(), false)
    };

    let vm_config = VmConfig {
        payload,
        cpus: Some(CpusConfig {
            boot_vcpus: number_of_cores,
            max_vcpus: number_of_cores,
            ..Default::default()
        }),

        console: Some(ConsoleConfig {
            mode: ConsoleMode::Null,
            ..Default::default()
        }),
        serial: Some(SerialConfig {
            // Note: Depending on your ch-api version, this may be SerialConfig or ConsoleConfig
            mode: ConsoleMode::File,
            file: Some(format!("/tmp/{}-serial.log", uuid)),
            ..Default::default()
        }),

        net: Some(vec![NetConfig {
            tap: Some(tap_name.clone()),
            mac: Some(mac_address.to_string()),
            ..Default::default()
        }]),
        memory: Some(MemoryConfig {
            size: memory_size,
            ..Default::default()
        }),
        disks: Some(vec![
            DiskConfig {
                path: Some(disk_path.clone()),
                readonly: Some(false),
                ..Default::default()
            },
            DiskConfig {
                path: Some(seed_path.to_string()),
                readonly: Some(true),
                ..Default::default()
            },
        ]),
        ..Default::default()
    };

    log::info!("Creating VM {uuid} attached to {tap_name}");
    if let Err(e) = client.create_vm(vm_config).await {
        return Err(AinariError::InternalError(format!(
            "Create VM {} failed: {:?}",
            uuid, e
        )));
    }

    log::info!("Booting VM {uuid} attached to {tap_name}");
    if let Err(e) = client.boot_vm().await {
        return Err(AinariError::InternalError(format!(
            "Boot VM {} failed: {:?}",
            uuid, e
        )));
    }

    // Capture the PID before moving `child` into the spawned task
    let vm_pid = child.id();

    tokio::spawn(async move {
        let _ = child
            .wait()
            .map_err(|e| AinariError::InternalError(format!("Failed to spawn VM: {e}")))?;

        Ok::<(), AinariError>(())
    });

    // Create the handle
    let handle = VmHandle {
        tap_name: tap_name.clone(),
        socket_path: socket_path.clone(),
        pid: vm_pid,
    };

    log::info!("New VM {uuid} started");

    Ok(handle)
}

// pub async fn delete_vm(handle: &VmHandle) -> HttpResponse {
//     let client = socket_based_api_client(&handle.socket_path);

//     // 1. Attempt graceful shutdown via the cloud-hypervisor API
//     // Note: Method names depend on your specific OpenAPI client version.
//     // It might be `shutdown_vm()`, `power_button()`, or `delete_vm()`.
//     match client.shutdown_vm().await {
//         Ok(_) => {
//             println!("Gracefully shut down VM: {}", handle.tap_name);
//         }
//         Err(e) => {
//             eprintln!("API shutdown failed for {}: {:?}. Forcing kill...", handle.tap_name, e);

//             // 2. Fallback: Force kill the process if the API is unresponsive
//             if let Some(pid) = handle.pid {
//                 unsafe {
//                     // Requires `libc` crate: Sends SIGKILL to the process
//                     libc::kill(pid as i32, libc::SIGKILL);
//                 }
//             }
//         }
//     }

//     // 3. Clean up the socket file
//     let _ = std::fs::remove_file(&handle.socket_path);

//     // 4. (Optional) Clean up the serial log file
//     let serial_log = format!("/tmp/{}-serial.log", handle.tap_name);
//     let _ = std::fs::remove_file(&serial_log);

//     HttpResponse::Ok().body(format!("VM {} deleted", handle.tap_name))
// }
