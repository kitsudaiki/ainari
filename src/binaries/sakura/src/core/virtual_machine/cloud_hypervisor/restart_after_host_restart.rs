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

//! Starts the virtual_machines again, which ran before sakura was restarted.
//!
//! The cloud-hypervisor processes of the virtual_machines are children of sakura, so they end
//! together with sakura, for example when its container or pod is restarted. Their disks are on
//! the persistent storage of the host, so the virtual_machines continue with their data, when
//! they are booted again. Their network is restored by hanami, when the host registers itself.

use std::os::unix::net::UnixStream;
use std::path::Path;

use tokio::runtime::Builder;
use tokio::task::LocalSet;

use ainari_api_structs::user_context::UserContext;
use ainari_common::enums::ProjectRole;

use super::start_ch_virtual_machine::start_ch_virtual_machine;
use super::vm_socket_path;
use crate::database::virtual_machine_table;
use crate::database::virtual_machine_table::VirtualMachineState;

/// Starts the background-thread, which boots the virtual_machines again, which were running,
/// before sakura was restarted.
///
/// It runs in the background, so the api of sakura is available, while the virtual_machines
/// boot.
pub fn spawn_restart_of_virtual_machines() {
    std::thread::spawn(|| {
        let rt = Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build runtime");
        let local = LocalSet::new();
        local.block_on(&rt, restart_virtual_machines());
    });
}

/// Boots every virtual_machine again, which is marked as running, but has no cloud-hypervisor
/// process anymore.
async fn restart_virtual_machines() {
    let context = system_context();
    let virtual_machines = match virtual_machine_table::list_virtual_machines(&context) {
        Ok(virtual_machines) => virtual_machines,
        Err(e) => {
            log::error!("Failed to list the virtual_machines for their restart: {e}");
            return;
        }
    };

    for virtual_machine in virtual_machines {
        let uuid = virtual_machine.uuid;
        if !remove_stale_socket(&vm_socket_path(&uuid)) {
            // its cloud-hypervisor process survived the restart of sakura
            continue;
        }
        if virtual_machine.vm_state != VirtualMachineState::Running.as_str() {
            continue;
        }

        log::info!("Start VM {uuid} again, which was running before the restart of sakura");
        if let Err(e) = start_ch_virtual_machine(&uuid, &context).await {
            log::error!("Failed to start VM {uuid} again after the restart of sakura: {e}");
        }
    }
}

/// Removes the API-socket of a cloud-hypervisor process, which doesn't run anymore.
///
/// The socket lies in the directory of the virtual_machine on the persistent storage, so it is
/// still there, after its process ended with sakura. A process, which still runs, for example
/// when sakura runs directly on the host and only itself was restarted, still accepts
/// connections on it and is left alone.
///
/// # Arguments
/// * `socket_path` - Path of the API-socket
///
/// # Returns
/// `true` if no process listens on the socket anymore
fn remove_stale_socket(socket_path: &str) -> bool {
    if !Path::new(socket_path).exists() {
        return true;
    }
    if UnixStream::connect(socket_path).is_ok() {
        return false;
    }
    if let Err(e) = std::fs::remove_file(socket_path) {
        log::warn!("Failed to remove the stale socket '{socket_path}': {e}");
    }
    true
}

/// Context of the restart, which isn't bound to a user and sees every virtual_machine
fn system_context() -> UserContext {
    UserContext {
        token: String::new(),
        user_id: "sakura".to_string(),
        project_id: String::new(),
        is_admin: true.to_string(),
        project_role: ProjectRole::Admin.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    #[test]
    fn a_socket_without_process_is_removed() {
        let dir = std::env::temp_dir().join(format!("sakura_socket_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("stale.sock");
        drop(UnixListener::bind(&path).unwrap());
        assert!(path.exists());

        assert!(remove_stale_socket(path.to_str().unwrap()));
        assert!(!path.exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_socket_of_a_running_process_is_kept() {
        let dir = std::env::temp_dir().join(format!("sakura_socket_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("alive.sock");
        let _listener = UnixListener::bind(&path).unwrap();

        assert!(!remove_stale_socket(path.to_str().unwrap()));
        assert!(path.exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_missing_socket_needs_a_start() {
        assert!(remove_stale_socket(
            "/nonexistent/ainari/cloud-hypervisor.sock"
        ));
    }
}
