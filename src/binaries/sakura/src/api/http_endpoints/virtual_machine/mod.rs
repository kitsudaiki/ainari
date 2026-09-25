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

pub mod create_virtual_machine_v1_0;
pub mod delete_virtual_machine_internal_v1_0;
pub mod get_virtual_machine_internal_v1_0;
pub mod list_virtual_machine_internal_v1_0;
pub mod reboot_virtual_machine_v1_0;
pub mod reserve_virtual_machine_internal_v1_0;
pub mod snapshot_restore_v1_0;
pub mod snapshot_save_v1_0;
pub mod start_virtual_machine_v1_0;
pub mod stop_virtual_machine_v1_0;

use uuid::Uuid;

use crate::core::processing::tasks::{
    CloudHypervisorVirtualMachinePowerInfo, Task, TaskMeta, TaskVariant,
};
use crate::database::task_table;
use crate::database::virtual_machine_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;

/// Removes all files and directories in the specified target directory.
///
/// This function performs a complete cleanup of the specified directory,
/// removing all files and subdirectories within it.
///
/// # Arguments
/// * `target_dir_path` - The path to the directory to remove
#[allow(dead_code)]
fn remove_all(target_dir_path: &String) {
    // delete all temporary files
    let _ = std::fs::remove_dir_all(target_dir_path).map_err(|e| {
        log::error!("Failed to delete temp-dir {target_dir_path} from disk with error {e}.");
    });
}

/// Creates a new task, which changes the power-state of a virtual_machine
///
/// This is shared by the start-, stop- and reboot-endpoints, which only differ in the type of
/// the task and the operation, which the task runs.
///
/// # Arguments
/// * `virtual_machine_uuid` - Unique identifier of the virtual_machine
/// * `task_type` - Type of the new task
/// * `action` - Name of the operation, which is used for the description of the task
/// * `variant` - Wraps the task-info into the variant of the task
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(TaskResp)` with the new task on success
/// * `Err(ErrorResponse)` if the virtual_machine doesn't exist or the task could not be created
fn add_power_task(
    virtual_machine_uuid: &Uuid,
    task_type: TaskType,
    action: &str,
    variant: fn(CloudHypervisorVirtualMachinePowerInfo) -> TaskVariant,
    context: &UserContext,
) -> Result<TaskResp, ErrorResponse> {
    // check that the virtual_machine exists, before a task for it is created
    let virtual_machine_data =
        virtual_machine_table::get_virtual_machine(virtual_machine_uuid, context).map_err(|e| {
            map_db_uuid_get_delete_error("virtual_machine", virtual_machine_uuid, e)
        })?;

    let task_uuid = Uuid::new_v4();

    // prepare task-info
    let task_description = format!(
        "{action} virtual machine with UUID {}",
        virtual_machine_data.uuid
    );
    let info = CloudHypervisorVirtualMachinePowerInfo {
        vm_uuid: virtual_machine_data.uuid,
        description: task_description.clone(),
        context: context.clone(),
    };

    // create new task, which is processed by the same worker-thread as all other tasks of the
    // virtual_machine, so it can not overtake its creation
    let task = Task {
        uuid: task_uuid,
        resouce_uuid: *virtual_machine_uuid,
        resource_type: TaskResourceType::VirtualMachine,
        description: task_description,
        info: variant(info),
        meta: TaskMeta::new(),
    };
    super::task::add_task(task, &task_type, context)
        .inspect_err(|e| log::error!("Creating a {task_type} failed with error: {e}"))?;

    // get new created task from database to get additional information
    let task_data = task_table::get_task(&task_uuid, context)
        .map_err(|e| map_db_uuid_get_delete_error("task", &task_uuid, e))?;

    Ok(TaskResp {
        uuid: task_uuid,
        description: task_data.description,
        task_type: task_data.task_type,
        state: task_data.task_state,
        queued_at: task_data.queued_at,
        started_at: task_data.started_at,
        finished_at: task_data.finished_at,
        messages: task_data.messages,
        created_by: task_data.created_by,
    })
}
