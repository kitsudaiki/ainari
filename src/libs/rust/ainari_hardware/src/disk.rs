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

use std::io;
use std::path::{Path, PathBuf};

use sysinfo::Disks;

/// Total and available space of a disk in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiskSpace {
    /// total size of the disk
    pub total: u64,
    /// space, which is still available on the disk
    pub available: u64,
}

/// Retrieves the total size of the disk, on which the given path is located.
///
/// The path doesn't have to exist. If it doesn't, the nearest existing parent-directory
/// is used to identify the disk. The disk is the one with the longest mount-point,
/// which is a prefix of the resolved path.
///
/// # Arguments
///
/// * `path` - Path to a file or directory on the disk
///
/// # Returns
///
/// * `Ok(u64)` - Total size of the disk in bytes
/// * `Err` - If the path could not be resolved or no disk was found for the path
pub fn get_total_disk_space(path: &Path) -> io::Result<u64> {
    get_disk_space(path).map(|space| space.total)
}

/// Retrieves the total and the available space of the disk, on which the given path is located.
///
/// The disk is identified like in `get_total_disk_space`.
///
/// # Arguments
///
/// * `path` - Path to a file or directory on the disk
///
/// # Returns
///
/// * `Ok(DiskSpace)` - Total and available space of the disk in bytes
/// * `Err` - If the path could not be resolved or no disk was found for the path
pub fn get_disk_space(path: &Path) -> io::Result<DiskSpace> {
    let resolved_path = resolve_existing_path(path)?;

    let disks = Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .filter(|disk| resolved_path.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().as_os_str().len())
        .map(|disk| DiskSpace {
            total: disk.total_space(),
            available: disk.available_space(),
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("No disk found for path '{}'", path.display()),
            )
        })
}

/// Resolves the given path to an absolute path without symlinks. If the path doesn't exist,
/// the nearest existing parent-directory is resolved instead.
///
/// # Arguments
///
/// * `path` - Path to resolve
///
/// # Returns
///
/// * `Ok(PathBuf)` - Canonicalized path of the path or its nearest existing parent
/// * `Err` - If no part of the path could be resolved
fn resolve_existing_path(path: &Path) -> io::Result<PathBuf> {
    let absolute_path = std::path::absolute(path)?;
    for ancestor in absolute_path.ancestors() {
        if let Ok(resolved) = ancestor.canonicalize() {
            return Ok(resolved);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("Failed to resolve path '{}'", path.display()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disk() {
        let total = get_total_disk_space(Path::new("/")).unwrap();
        assert!(total > 0);
        println!("Total disk space of '/': {total}");

        let non_existing = get_total_disk_space(Path::new("/tmp/ainari/does/not/exist")).unwrap();
        assert!(non_existing > 0);

        let space = get_disk_space(Path::new("/")).unwrap();
        assert_eq!(space.total, total);
        assert!(space.available <= space.total);
    }
}
