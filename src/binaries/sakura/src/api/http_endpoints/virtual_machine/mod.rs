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
pub mod reserve_virtual_machine_internal_v1_0;
pub mod snapshot_restore_v1_0;
pub mod snapshot_save_v1_0;

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
