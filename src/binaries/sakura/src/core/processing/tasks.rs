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

use uuid::Uuid;

use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::error::AinariError;

use crate::core::virtual_machine::cloud_hypervisor::create_ch_virtual_machine::create_ch_virtual_machine;
use crate::core::virtual_machine::cloud_hypervisor::delete_ch_virtual_machine::delete_ch_virtual_machine;
use crate::core::virtual_machine::cloud_hypervisor::restore_ch_virtual_machine::restore_ch_virtual_machine;
use crate::core::virtual_machine::cloud_hypervisor::save_ch_virtual_machine::save_ch_virtual_machine;
use crate::database::task_table;

#[derive(Debug)]
pub struct CloudHypervisorVirtualMachineCreateInfo {
    #[allow(dead_code)]
    pub vm_uuid: Uuid,
    #[allow(dead_code)]
    pub description: String,
    pub context: UserContext,
}

#[derive(Debug)]
pub struct CloudHypervisorVirtualMachineDeleteInfo {
    #[allow(dead_code)]
    pub vm_uuid: Uuid,
    #[allow(dead_code)]
    pub description: String,
    pub context: UserContext,
}

#[derive(Debug)]
pub struct CloudHypervisorVirtualMachineSnapshotInfo {
    #[allow(dead_code)]
    pub vm_uuid: Uuid,
    /// Image, which was already registered in ryokan as snapshot and gets the root-disk as content
    pub image_uuid: Uuid,
    #[allow(dead_code)]
    pub description: String,
    pub context: UserContext,
}

#[derive(Debug)]
pub struct CloudHypervisorVirtualMachineRestoreInfo {
    #[allow(dead_code)]
    pub vm_uuid: Uuid,
    /// Image, which is a snapshot and replaces the root-disk of the virtual_machine
    pub image_uuid: Uuid,
    #[allow(dead_code)]
    pub description: String,
    pub context: UserContext,
}

/// An enumeration of different task variants that a Task can have.
/// Each variant contains different information relevant to that type of task.
#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum TaskVariant {
    CloudHypervisorVirtualMachineCreate(CloudHypervisorVirtualMachineCreateInfo),
    CloudHypervisorVirtualMachineDelete(CloudHypervisorVirtualMachineDeleteInfo),
    CloudHypervisorVirtualMachineSnapshot(CloudHypervisorVirtualMachineSnapshotInfo),
    CloudHypervisorVirtualMachineRestore(CloudHypervisorVirtualMachineRestoreInfo),
}

/// Metadata for tracking the state of a task.
#[derive(Debug)]
pub struct TaskMeta {
    /// True, as soon as the task was processed to its end.
    pub is_finished: bool,
}

impl TaskMeta {
    /// Creates a new TaskMeta for a task, which was not started yet.
    ///
    /// # Returns
    ///
    /// A new TaskMeta, which is marked as unfinished.
    pub fn new() -> Self {
        Self { is_finished: false }
    }
}

/// Represents a task that can be executed by the system.
#[derive(Debug)]
pub struct Task {
    /// Unique identifier of the task itself.
    pub uuid: Uuid,
    /// Identifier of the resource, which the task acts on. It also decides, which worker-thread
    /// processes the task, so all tasks of the same resource are serialized.
    pub resouce_uuid: Uuid,
    /// Type of the resource, which the task acts on.
    pub resource_type: TaskResourceType,
    /// Human-readable description of the task.
    #[allow(dead_code)]
    pub description: String,

    /// The concrete work of the task together with everything it needs for it.
    pub info: TaskVariant,
    /// State of the task while it is processed.
    pub meta: TaskMeta,
}

/// Processes a worker task.
///
/// Runs the task and finalizes it afterwards, which updates its state in the database. The
/// concrete work depends on the variant of the task. A failed task is marked as error and gets
/// the error as message, so the failure is visible to the user. It doesn't stop the processing of
/// the following tasks.
///
/// # Arguments
/// * `task` - A reference to the worker task to be processed
///
/// # Returns
/// * `Result<(), AinariError>` - Ok(()) if processing succeeds, Err(AinariError) if an error occurs
pub async fn process_task(task: &mut Task) -> Result<(), AinariError> {
    match task.start_task().await {
        Ok(()) => {
            let _ = task_table::update_task_state(&task.uuid, &TaskState::Finished);
        }
        Err(e) => {
            log::error!("Task {} ({}) failed: {e}", task.uuid, task.description);
            let _ = task_table::add_message_to_task(&task.uuid, &e.to_string());
            let _ = task_table::update_task_state(&task.uuid, &TaskState::Error);
        }
    }

    Ok(())
}

impl Task {
    // ==================================================================================================

    /// Starts the execution of the task.
    ///
    /// # Returns
    ///
    /// `true` if the task should continue execution, `false` if it should pause or stop.
    pub async fn start_task(&mut self) -> Result<(), AinariError> {
        // check if task was aborted
        if task_table::is_aborted(&self.uuid) {
            return Ok(());
        }

        let _ = task_table::update_task_state(&self.uuid, &TaskState::Active);

        match &mut self.info {
            TaskVariant::CloudHypervisorVirtualMachineCreate(task_info) => {
                handle_vm_creation(&self.uuid, &self.resouce_uuid, &mut self.meta, task_info).await
            }
            TaskVariant::CloudHypervisorVirtualMachineDelete(task_info) => {
                handle_vm_deletion(&self.uuid, &self.resouce_uuid, &mut self.meta, task_info).await
            }
            TaskVariant::CloudHypervisorVirtualMachineSnapshot(task_info) => {
                handle_vm_snapshot(&self.uuid, &self.resouce_uuid, &mut self.meta, task_info).await
            }
            TaskVariant::CloudHypervisorVirtualMachineRestore(task_info) => {
                handle_vm_restore(&self.uuid, &self.resouce_uuid, &mut self.meta, task_info).await
            }
        }
    }

    /// Checks if the task has been completed.
    ///
    /// # Returns
    ///
    /// `true` if the task is finished, `false` otherwise.
    #[allow(dead_code)]
    pub fn is_task_finished(&self) -> bool {
        self.meta.is_finished
    }
}

/// Handles the task, which creates the virtual machine on this host.
///
/// A failure is returned, so the task is marked as failed.
///
/// # Arguments
///
/// * `_task_uuid` - Unique identifier for the task
/// * `virtual_machine_uuid` - Unique identifier for the virtual machine to create
/// * `_` - Unused TaskMeta parameter (kept for interface consistency)
/// * `task_info` - Information, which is needed to create the virtual machine
async fn handle_vm_creation(
    _task_uuid: &Uuid,
    virtual_machine_uuid: &Uuid,
    _: &mut TaskMeta,
    task_info: &mut CloudHypervisorVirtualMachineCreateInfo,
) -> Result<(), AinariError> {
    create_ch_virtual_machine(virtual_machine_uuid, &task_info.context)
        .await
        .map(|_| ())
}

/// Handles the task, which deletes the virtual machine completely from this host.
///
/// A failure is returned, so the task is marked as failed.
///
/// # Arguments
///
/// * `_task_uuid` - Unique identifier for the task
/// * `virtual_machine_uuid` - Unique identifier for the virtual machine to delete
/// * `_` - Unused TaskMeta parameter (kept for interface consistency)
/// * `task_info` - Information, which is needed to delete the virtual machine
async fn handle_vm_deletion(
    _task_uuid: &Uuid,
    virtual_machine_uuid: &Uuid,
    _: &mut TaskMeta,
    task_info: &mut CloudHypervisorVirtualMachineDeleteInfo,
) -> Result<(), AinariError> {
    delete_ch_virtual_machine(virtual_machine_uuid, &task_info.context).await
}

/// Handles the task, which saves the root-disk of the virtual_machine as snapshot.
///
/// A failure is returned, so the task is marked as failed.
///
/// # Arguments
///
/// * `_task_uuid` - Unique identifier for the task
/// * `virtual_machine_uuid` - Unique identifier for the virtual machine to save
/// * `_` - Unused TaskMeta parameter (kept for interface consistency)
/// * `task_info` - Information, which is needed to create the snapshot
async fn handle_vm_snapshot(
    _task_uuid: &Uuid,
    virtual_machine_uuid: &Uuid,
    _: &mut TaskMeta,
    task_info: &mut CloudHypervisorVirtualMachineSnapshotInfo,
) -> Result<(), AinariError> {
    save_ch_virtual_machine(
        virtual_machine_uuid,
        &task_info.image_uuid,
        &task_info.context,
    )
    .await
}

/// Handles the task, which resets the root-disk of the virtual_machine to a snapshot.
///
/// A failure is returned, so the task is marked as failed.
///
/// # Arguments
///
/// * `_task_uuid` - Unique identifier for the task
/// * `virtual_machine_uuid` - Unique identifier for the virtual machine to reset
/// * `_` - Unused TaskMeta parameter (kept for interface consistency)
/// * `task_info` - Information, which is needed to restore the snapshot
async fn handle_vm_restore(
    _task_uuid: &Uuid,
    virtual_machine_uuid: &Uuid,
    _: &mut TaskMeta,
    task_info: &mut CloudHypervisorVirtualMachineRestoreInfo,
) -> Result<(), AinariError> {
    restore_ch_virtual_machine(
        virtual_machine_uuid,
        &task_info.image_uuid,
        &task_info.context,
    )
    .await
}

/// Removes a directory and all its contents from the filesystem.
///
/// This function attempts to delete a directory and all files within it. If the operation fails,
/// it logs an error message but does not propagate the error.
///
/// # Arguments
///
/// * `target_dir_path` - Path to the directory to be removed
#[allow(dead_code)]
fn remove_dir_all(target_dir_path: &String) {
    // delete all temporary files
    let _ = std::fs::remove_dir_all(target_dir_path).map_err(|e| {
        log::error!("Failed to delete temp-dir {target_dir_path} from disk with error {e}.");
    });
}
