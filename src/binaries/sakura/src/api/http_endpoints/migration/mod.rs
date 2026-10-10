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

//! Internal endpoints for the cold migration of a virtual_machine to another sakura-host.
//!
//! They are only called by hanami, which orchestrates the migration, and by the target host,
//! which pulls the description and the files of the virtual_machine from the source host. See
//! `core::virtual_machine::cloud_hypervisor::migration` for the steps.

pub mod cancel_migration_internal_v1_0;
pub mod get_migration_file_internal_v1_0;
pub mod get_migration_internal_v1_0;
pub mod import_virtual_machine_internal_v1_0;
pub mod prepare_migration_internal_v1_0;
pub mod remove_migrated_virtual_machine_internal_v1_0;

use uuid::Uuid;

use crate::core::processing::tasks::{Task, TaskMeta, TaskVariant};
use crate::database::task_table;
use crate::database::virtual_machine_table;
use crate::database::virtual_machine_table::{VirtualMachineEntry, VirtualMachineState};

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;

/// Queues a task, which runs a step of the migration of a virtual_machine
///
/// The task is processed by the same worker-thread as all other tasks of the virtual_machine, so
/// the steps of the migration are serialized with them.
///
/// # Arguments
/// * `virtual_machine_uuid` - Unique identifier of the virtual_machine
/// * `task_type` - Type of the new task
/// * `description` - Human-readable description of the task
/// * `variant` - The step of the migration together with everything it needs
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(TaskResp)` with the new task on success
/// * `Err(ErrorResponse)` if the task could not be created
fn add_migration_task(
    virtual_machine_uuid: &Uuid,
    task_type: TaskType,
    description: String,
    variant: TaskVariant,
    context: &UserContext,
) -> Result<TaskResp, ErrorResponse> {
    let task_uuid = Uuid::new_v4();
    let task = Task {
        uuid: task_uuid,
        resouce_uuid: *virtual_machine_uuid,
        resource_type: TaskResourceType::VirtualMachine,
        description,
        info: variant,
        meta: TaskMeta::new(),
    };
    super::task::add_task(task, &task_type, context)
        .inspect_err(|e| log::error!("Creating a {task_type} failed with error: {e}"))?;

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

/// Reads a virtual_machine, which is frozen on this host for its migration
///
/// # Arguments
/// * `virtual_machine_uuid` - Unique identifier of the virtual_machine
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(VirtualMachineEntry)` with the prepared virtual_machine
/// * `Err(ErrorResponse)` with `Conflict`, if the virtual_machine is not prepared, otherwise an
///   appropriate error on failure
fn get_prepared_virtual_machine(
    virtual_machine_uuid: &Uuid,
    context: &UserContext,
) -> Result<VirtualMachineEntry, ErrorResponse> {
    let virtual_machine_data =
        virtual_machine_table::get_virtual_machine(virtual_machine_uuid, context).map_err(|e| {
            map_db_uuid_get_delete_error("virtual_machine", virtual_machine_uuid, e)
        })?;

    if virtual_machine_data.vm_state != VirtualMachineState::Migrating.as_str() {
        return Err(ErrorResponse::Conflict(format!(
            "Virtual_machine '{virtual_machine_uuid}' is not prepared for a migration."
        )));
    }
    Ok(virtual_machine_data)
}
