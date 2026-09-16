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

use std::time::{Duration, Instant};
use uuid::Uuid;

use ainari_api_structs::task_structs::*;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::database::task_table;

use crate::core::virtual_machine::cloud_hypervisor::create_ch_virtual_machine::create_ch_virtual_machine;

#[derive(Debug)]
#[allow(dead_code)]
pub struct CloudHypervisorVirtualMachineCreateInfo {
    pub vm_uuid: Uuid,
    pub number_of_cores: i32,
    pub memory_size: i64,
    pub public_key: Secret,
}

/// An enumeration of different task variants that a Task can have.
/// Each variant contains different information relevant to that type of task.
#[derive(Debug)]
pub enum TaskVariant {
    CloudHypervisorVirtualMachineCreate(CloudHypervisorVirtualMachineCreateInfo),
}

/// Metadata for tracking the progress and state of a task.
/// Includes counters for cycles and epochs, timestamps, and completion status.
#[derive(Debug)]
#[allow(dead_code)]
pub struct TaskMeta {
    pub number_of_cycles: u64,
    pub number_of_epochs: u64,
    pub number_of_finished_cycles: u64,
    pub number_of_finished_epochs: u64,
    pub time_length: u64,
    pub forecast_length: u64,

    pub task_cycle_counter: u64,

    pub is_finished: bool,
    pub prev_timestamp: std::time::Instant,
}

impl TaskMeta {
    /// Creates a new TaskMeta virtual_machine with the given parameters.
    ///
    /// # Arguments
    ///
    /// * `number_of_cycler_per_epoch` - Total number of cycles per epoch.
    /// * `number_of_epochs` - Total number of epochs.
    /// * `time_length` - Time length for the task in seconds.
    ///
    /// # Returns
    ///
    /// A new TaskMeta virtual_machine initialized with the given parameters.
    pub fn new(
        number_of_cycler_per_epoch: u64,
        number_of_epochs: u64,
        time_length: u64,
        forecast_length: u64,
    ) -> Self {
        Self {
            number_of_cycles: number_of_cycler_per_epoch,
            number_of_epochs,
            number_of_finished_cycles: 0,
            number_of_finished_epochs: 0,
            time_length,
            forecast_length,

            task_cycle_counter: 0,

            is_finished: false,
            prev_timestamp: std::time::Instant::now(),
        }
    }
}

/// Represents a task that can be executed by the system.
/// Contains a unique identifier, virtual_machine identifier, task information, and metadata.
#[derive(Debug)]
pub struct Task {
    pub uuid: Uuid,
    pub resouce_uuid: Uuid,
    pub resource_type: TaskResourceType,
    #[allow(dead_code)]
    pub name: String,

    pub info: TaskVariant,
    pub meta: TaskMeta,
}

/// Processes a worker task according to its type.
///
/// This function handles the execution of the task based on its type (Train, Process, or Backpropagate).
/// It performs the appropriate operation on the task's block and then finalizes the task.
/// If the task requires updating a finish counter, it does so after finalization.
///
/// # Arguments
/// * `task` - A reference to the worker task to be processed
///
/// # Returns
/// * `Result<(), AinariError>` - Ok(()) if processing succeeds, Err(AinariError) if an error occurs
pub async fn process_task(task: &mut Task) -> Result<(), AinariError> {
    task.start_task().await?;

    task.finalize_task().await?;

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

        self.meta.prev_timestamp = Instant::now();
        let _ = task_table::update_task_state(&self.uuid, &TaskState::Active);

        match &mut self.info {
            TaskVariant::CloudHypervisorVirtualMachineCreate(task_info) => {
                handle_vm_creation(&self.uuid, &self.resouce_uuid, &mut self.meta, task_info).await;
                Ok(())
            }
        }
    }

    /// Finalizes the task, performing cleanup and updating the task state.
    /// For request tasks, it encrypts and uploads the results.
    /// For training tasks, it cleans up temporary files.
    pub async fn finalize_task(&mut self) -> Result<(), AinariError> {
        // if let TaskVariant::Request(task_info) = &mut self.info {
        //     let rt = Builder::new_current_thread()
        //         .enable_all() // I/O & timers
        //         .build()
        //         .expect("failed to build runtime");

        //     // LocalSet allows spawn_local to work
        //     let local = LocalSet::new();
        //     let upload_resp = local.block_on(&rt, async {
        //         encrypt_file(
        //             &task_info.results.link.local_file_path,
        //             &task_info.results.link.local_encrypted_file_path,
        //             &task_info.output_secret,
        //         )
        //         .await?;
        //         upload_file(
        //             &task_info.results.link.onsen_address,
        //             &task_info.results.link.remote_file_path,
        //             &task_info.results.link.local_encrypted_file_path,
        //         )
        //         .await
        //     });

        //     // delete temp-files
        //     remove_dir_all(&task_info.temp_dir);

        //     // handle result
        //     match upload_resp {
        //         Ok(()) => {}
        //         Err(_) => {
        //             let _ = task_table::update_task_state(&self.uuid, &TaskState::Error);
        //             let _ = task_table::update_task_progress(
        //                 &self.uuid,
        //                 &(self.meta.number_of_epochs as i64),
        //                 &(self.meta.number_of_cycles as i64),
        //             );
        //             return;
        //         }
        //     }
        // }

        let _ = task_table::update_task_state(&self.uuid, &TaskState::Finished);

        Ok(())
    }

    /// Finishes the current cycle of the task and prepares for the next cycle.
    /// Updates progress in the database and checks for task completion.
    #[allow(dead_code)]
    pub fn finish_cycle(&mut self) {
        // update current state in database at least after 1 second
        let now = Instant::now();
        if now.duration_since(self.meta.prev_timestamp) >= Duration::from_secs(1) {
            self.meta.prev_timestamp = now;
            let _ = task_table::update_task_progress(
                &self.uuid,
                &(self.meta.number_of_finished_epochs as i64),
                &(self.meta.number_of_finished_cycles as i64),
            );
            if task_table::is_aborted(&self.uuid) {
                self.meta.is_finished = true;
                return;
            }
        }

        // update and check cycle- and epoch-counter
        self.meta.number_of_finished_cycles += 1;
        if self.meta.number_of_finished_cycles >= self.meta.number_of_cycles {
            self.meta.number_of_finished_epochs += 1;
            if self.meta.number_of_finished_epochs == self.meta.number_of_epochs {
                self.meta.is_finished = true;
                return;
            } else {
                self.meta.number_of_finished_cycles = 0;
            }
        }
        self.meta.task_cycle_counter += 1;
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

async fn handle_vm_creation(
    _task_uuid: &Uuid,
    virtual_machine_uuid: &Uuid,
    _: &mut TaskMeta,
    task_info: &mut CloudHypervisorVirtualMachineCreateInfo,
) {
    let root_disk_path = Some("/tmp/ubuntu-24.04.raw".to_string());
    let seed_disk_path = "/tmp/seed.iso".to_string();
    let tap_device_name = "tap-vm".to_string();
    let mac_address = "02:00:00:00:00:42".to_string();

    match create_ch_virtual_machine(
        virtual_machine_uuid,
        task_info.number_of_cores,
        task_info.memory_size,
        root_disk_path,
        &seed_disk_path,
        &tap_device_name,
        &mac_address,
    )
    .await
    {
        Ok(_) => (),
        Err(e) => log::error!("fail: {:?}", e),
    }
}

// /// Handles the task of saving a virtual_machine checkpoint.
// ///
// /// This function creates a checkpoint of the virtual_machine, encrypts it, and uploads it to the specified
// /// storage location. It manages temporary files and updates the task state in the database.
// ///
// /// # Arguments
// ///
// /// * `task_uuid` - Unique identifier for the task
// /// * `virtual_machine_uuid` - Unique identifier for the virtual_machine
// /// * `_` - Unused TaskMeta parameter (kept for interface consistency)
// /// * `task_info` - Mutable reference to checkpoint save information containing storage details
// fn handle_checkpoint_save_task(
//     task_uuid: &Uuid,
//     virtual_machine_uuid: &Uuid,
//     _: &mut TaskMeta,
//     task_info: &mut CheckpointSaveInfo,
// ) {
//     // create file-paths for temporary files
//     let local_temp_file_path = format!(
//         "{}/{}",
//         config::CONFIG.storage.tempfile_location,
//         virtual_machine_uuid
//     );
//     let local_encrypted_temp_file_path = format!("{local_temp_file_path}_encrypted");

//     {
//         // let virtual_machine_handler = MODEL_HANDLER.read().expect("mutex poisoned");
//         // match virtual_machine_handler.create_checkpoint(virtual_machine_uuid, &local_temp_file_path) {
//         //     Ok(()) => {}
//         //     Err(_) => {
//         //         let _ = fs::remove_file(&local_temp_file_path);
//         //         let _ = task_table::update_task_state(task_uuid, &TaskState::Error);
//         //         let _ = task_table::update_task_progress(task_uuid, &1, &1);
//         //         return;
//         //     }
//         // }

//         // Create a single-threaded runtime
//         let rt = Builder::new_current_thread()
//             .enable_all() // I/O & timers
//             .build()
//             .expect("failed to build runtime");

//         // LocalSet allows spawn_local to work
//         let local = LocalSet::new();
//         let upload_resp = local.block_on(&rt, async {
//             encrypt_file(
//                 &local_temp_file_path,
//                 &local_encrypted_temp_file_path,
//                 &task_info.secret,
//             )
//             .await?;
//             upload_file(
//                 &task_info.onsen_address,
//                 &task_info.file_path,
//                 &local_encrypted_temp_file_path,
//             )
//             .await
//         });

//         match upload_resp {
//             Ok(()) => {}
//             Err(_) => {
//                 let _ = task_table::update_task_state(task_uuid, &TaskState::Error);
//                 let _ = task_table::update_task_progress(task_uuid, &1, &1);
//                 return;
//             }
//         }

//         let _ = task_table::update_task_state(task_uuid, &TaskState::Finished);
//         let _ = task_table::update_task_progress(task_uuid, &1, &1);
//     }

//     let _ = fs::remove_file(&local_temp_file_path);
//     let _ = fs::remove_file(&local_encrypted_temp_file_path);
// }

// /// Handles the task of restoring a virtual_machine from a checkpoint.
// ///
// /// This function downloads an encrypted checkpoint file, decrypts it, and restores the virtual_machine from
// /// the checkpoint. It manages temporary files and updates the task state in the database.
// ///
// /// # Arguments
// ///
// /// * `task_uuid` - Unique identifier for the task
// /// * `virtual_machine_uuid` - Unique identifier for the virtual_machine
// /// * `_` - Unused TaskMeta parameter (kept for interface consistency)
// /// * `task_info` - Mutable reference to checkpoint restore information containing storage details
// fn handle_checkpoint_restore_task(
//     task_uuid: &Uuid,
//     _: &mut TaskMeta,
//     task_info: &mut CheckpointRestoreInfo,
// ) {
//     // create file-paths for temporary files
//     let local_temp_file_path = format!(
//         "{}/{}",
//         config::CONFIG.storage.tempfile_location,
//         Uuid::new_v4()
//     );
//     let local_encrypted_temp_file_path = format!("{local_temp_file_path}_encrypted");

//     {
//         // Create a single-threaded runtime
//         let rt = Builder::new_current_thread()
//             .enable_all() // I/O & timers
//             .build()
//             .expect("failed to build runtime");

//         // LocalSet allows spawn_local to work
//         let local = LocalSet::new();
//         let download_resp = local.block_on(&rt, async {
//             let resp = download_file(
//                 &task_info.onsen_address,
//                 &task_info.file_path,
//                 &local_encrypted_temp_file_path,
//             )
//             .await;
//             decrypt_file(
//                 &local_encrypted_temp_file_path,
//                 &local_temp_file_path,
//                 &task_info.secret,
//             )
//             .await?;

//             resp
//         });

//         match download_resp {
//             Ok(()) => {}
//             Err(e) => {
//                 log::error!("Error in checkpoint-restore-task: {e}");
//                 let _ = task_table::update_task_state(task_uuid, &TaskState::Error);
//                 let _ = task_table::update_task_progress(task_uuid, &1, &1);
//                 return;
//             }
//         }

//         // // restore virtual_machine from the downloaded and decrypted checkpoint-file
//         // let mut virtual_machine_handler = MODEL_HANDLER.write().expect("mutex poisoned");
//         // match virtual_machine_handler.restore_checkpoint(virtual_machine_uuid, &local_temp_file_path) {
//         //     Ok(()) => {}
//         //     Err(_) => {
//         //         let _ = task_table::update_task_state(task_uuid, &TaskState::Error);
//         //         let _ = task_table::update_task_progress(task_uuid, &1, &1);
//         //         return;
//         //     }
//         // }

//         // delete temporary checkpoint-file
//         let _ = task_table::update_task_state(task_uuid, &TaskState::Finished);
//         let _ = task_table::update_task_progress(task_uuid, &1, &1);
//     }

//     // cleanup temp-files
//     let _ = fs::remove_file(&local_temp_file_path);
//     let _ = fs::remove_file(&local_encrypted_temp_file_path);
// }

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
