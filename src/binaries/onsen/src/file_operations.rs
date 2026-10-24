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

//! Operations on directories and files within the storage-location.
//!
//! All functions get the storage-location as argument instead of reading it from the config, so
//! they can be tested with temporary directories.

use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{ErrorKind, Read};
use std::path::{Component, Path, PathBuf};
use tonic::Status;

use crate::server::data::{DirectoryEntry, FileInfoResponse};
use ainari_common::constants::CHUNK_SIZE;
use ainari_common::functions::is_safe_subpath;

/// Builds the absolute path of a remote path within the storage-location.
///
/// # Arguments
///
/// * `root` - The storage-location
/// * `remote_path` - Relative path within the storage-location
///
/// # Returns
///
/// The absolute path, or `INVALID_ARGUMENT`, if the remote path would leave the storage-location.
fn resolve_path(root: &Path, remote_path: &str) -> Result<PathBuf, Status> {
    let remote_path = Path::new(remote_path);
    if !is_safe_subpath(remote_path) {
        let msg = format!("provided remote-path is invalid: {:?}", remote_path);
        log::error!("{}", msg);
        return Err(Status::invalid_argument(msg));
    }
    Ok(root.join(remote_path))
}

/// Lists the direct entries of a directory, sorted by name.
///
/// Symbolic links are not followed, so they are listed as files with the size of the link.
///
/// # Arguments
///
/// * `root` - The storage-location
/// * `remote_dir_path` - Relative path of the directory, an empty path is the storage-location
///
/// # Returns
///
/// The entries, `NOT_FOUND`, if the directory doesn't exist, or `INVALID_ARGUMENT`, if the path is
/// invalid or not a directory.
pub async fn list_directory(
    root: &Path,
    remote_dir_path: &str,
) -> Result<Vec<DirectoryEntry>, Status> {
    let dir_path = resolve_path(root, remote_dir_path)?;

    let mut read_dir = tokio::fs::read_dir(&dir_path)
        .await
        .map_err(|e| match e.kind() {
            ErrorKind::NotFound => {
                Status::not_found(format!("directory not found: {:?}", remote_dir_path))
            }
            ErrorKind::NotADirectory => {
                Status::invalid_argument(format!("not a directory: {:?}", remote_dir_path))
            }
            _ => Status::internal(format!("failed to read directory: {e}")),
        })?;

    let mut entries = Vec::new();
    while let Some(entry) = read_dir
        .next_entry()
        .await
        .map_err(|e| Status::internal(format!("failed to read directory: {e}")))?
    {
        // `DirEntry::metadata` doesn't follow symbolic links
        let metadata = entry
            .metadata()
            .await
            .map_err(|e| Status::internal(format!("failed to read metadata: {e}")))?;
        let is_dir = metadata.is_dir();
        entries.push(DirectoryEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            is_dir,
            size: if is_dir { 0 } else { metadata.len() },
        });
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(entries)
}

/// Deletes a directory with all of its content.
///
/// Symbolic links within the directory are removed, but not followed. Deleting a not existing
/// directory is no error, so a failed deletion can simply be repeated.
///
/// # Arguments
///
/// * `root` - The storage-location
/// * `remote_dir_path` - Relative path of the directory
///
/// # Returns
///
/// Ok, or `INVALID_ARGUMENT`, if the path is invalid, the storage-location itself or not a
/// directory.
pub async fn delete_directory(root: &Path, remote_dir_path: &str) -> Result<(), Status> {
    // prevent, that the whole storage-location is deleted by an empty path or by "."
    let has_normal_component = Path::new(remote_dir_path)
        .components()
        .any(|comp| matches!(comp, Component::Normal(_)));
    if !has_normal_component {
        return Err(Status::invalid_argument(
            "the storage-location itself can not be deleted",
        ));
    }
    let dir_path = resolve_path(root, remote_dir_path)?;

    match tokio::fs::symlink_metadata(&dir_path).await {
        Ok(metadata) if !metadata.is_dir() => {
            return Err(Status::invalid_argument(format!(
                "not a directory: {:?}",
                remote_dir_path
            )));
        }
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(Status::internal(format!("failed to read metadata: {e}"))),
    }

    tokio::fs::remove_dir_all(&dir_path).await.map_err(|e| {
        let msg = format!("failed to delete directory {:?}: {e}", dir_path);
        log::error!("{}", msg);
        Status::internal(msg)
    })?;
    log::info!("Deleted directory: {:?}", dir_path);

    Ok(())
}

/// Returns the size and the sha256-checksum of a file.
///
/// The file is hashed on the blocking thread-pool, because it can be a disk-image with many
/// gigabytes, which would otherwise block the grpc-server.
///
/// # Arguments
///
/// * `root` - The storage-location
/// * `remote_file_path` - Relative path of the file
///
/// # Returns
///
/// The info, `NOT_FOUND`, if the file doesn't exist, or `INVALID_ARGUMENT`, if the path is
/// invalid or not a file.
pub async fn get_file_info(
    root: &Path,
    remote_file_path: &str,
) -> Result<FileInfoResponse, Status> {
    let file_path = resolve_path(root, remote_file_path)?;
    let remote_file_path = remote_file_path.to_owned();

    tokio::task::spawn_blocking(move || {
        let not_found = || Status::not_found(format!("file not found: {:?}", remote_file_path));

        // symbolic links are not followed, so no file outside of the storage-location is read
        let metadata = std::fs::symlink_metadata(&file_path).map_err(|e| match e.kind() {
            ErrorKind::NotFound => not_found(),
            _ => Status::internal(format!("failed to read metadata: {e}")),
        })?;
        if !metadata.is_file() {
            return Err(Status::invalid_argument(format!(
                "not a file: {:?}",
                remote_file_path
            )));
        }

        let mut file = File::open(&file_path).map_err(|e| match e.kind() {
            ErrorKind::NotFound => not_found(),
            _ => Status::internal(format!("failed to open file: {e}")),
        })?;

        // the size is counted while hashing, so it matches the hashed content, even if the file
        // was changed in between
        let mut hasher = Sha256::new();
        let mut buf = vec![0u8; CHUNK_SIZE];
        let mut size: u64 = 0;
        loop {
            let n = file
                .read(&mut buf)
                .map_err(|e| Status::internal(format!("read error: {e}")))?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            size += n as u64;
        }

        Ok(FileInfoResponse {
            size,
            sha256: hex::encode(hasher.finalize()),
        })
    })
    .await
    .map_err(|e| Status::internal(format!("failed to hash file: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::Code;

    /// Temporary storage-location, which is deleted at the end of the test.
    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("onsen_test_{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&path).unwrap();
            TestRoot(path)
        }

        fn write(&self, remote_path: &str, content: &[u8]) {
            let path = self.0.join(remote_path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[tokio::test]
    async fn test_list_directory() {
        let root = TestRoot::new();
        root.write("backups/b2/manifest.json", b"{}");
        root.write("backups/b1/disks/vm.enc", b"disk");
        root.write("backups/file.txt", b"12345");

        let entries = list_directory(&root.0, "backups").await.unwrap();
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["b1", "b2", "file.txt"]);
        assert!(entries[0].is_dir);
        assert_eq!(entries[0].size, 0);
        assert!(!entries[2].is_dir);
        assert_eq!(entries[2].size, 5);

        // the storage-location itself
        let entries = list_directory(&root.0, "").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "backups");

        // empty directory
        std::fs::create_dir_all(root.0.join("empty")).unwrap();
        assert!(list_directory(&root.0, "empty").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_list_directory_errors() {
        let root = TestRoot::new();
        root.write("file.txt", b"content");

        let err = list_directory(&root.0, "missing").await.unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
        let err = list_directory(&root.0, "file.txt").await.unwrap_err();
        assert_eq!(err.code(), Code::InvalidArgument);
        let err = list_directory(&root.0, "../").await.unwrap_err();
        assert_eq!(err.code(), Code::InvalidArgument);
        let err = list_directory(&root.0, "/etc").await.unwrap_err();
        assert_eq!(err.code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_delete_directory() {
        let root = TestRoot::new();
        root.write("backups/b1/disks/vm.enc", b"disk");
        root.write("backups/b1/manifest.json", b"{}");
        root.write("backups/b2/manifest.json", b"{}");

        delete_directory(&root.0, "backups/b1").await.unwrap();
        assert!(!root.0.join("backups/b1").exists());
        assert!(root.0.join("backups/b2/manifest.json").exists());

        // deleting again is no error
        delete_directory(&root.0, "backups/b1").await.unwrap();
    }

    #[tokio::test]
    async fn test_delete_directory_errors() {
        let root = TestRoot::new();
        root.write("file.txt", b"content");

        for path in ["", ".", "./"] {
            let err = delete_directory(&root.0, path).await.unwrap_err();
            assert_eq!(err.code(), Code::InvalidArgument, "path: {path:?}");
        }
        assert!(root.0.join("file.txt").exists());

        let err = delete_directory(&root.0, "file.txt").await.unwrap_err();
        assert_eq!(err.code(), Code::InvalidArgument);
        assert!(root.0.join("file.txt").exists());

        let err = delete_directory(&root.0, "../other").await.unwrap_err();
        assert_eq!(err.code(), Code::InvalidArgument);
        let err = delete_directory(&root.0, "/tmp").await.unwrap_err();
        assert_eq!(err.code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_delete_directory_does_not_follow_symlinks() {
        let root = TestRoot::new();
        let outside = TestRoot::new();
        outside.write("keep.txt", b"keep");
        std::fs::create_dir_all(root.0.join("backups/b1")).unwrap();
        std::os::unix::fs::symlink(&outside.0, root.0.join("backups/b1/link")).unwrap();

        delete_directory(&root.0, "backups/b1").await.unwrap();
        assert!(!root.0.join("backups/b1").exists());
        assert!(outside.0.join("keep.txt").exists());

        // a symlink as target itself is not a directory
        std::os::unix::fs::symlink(&outside.0, root.0.join("link")).unwrap();
        let err = delete_directory(&root.0, "link").await.unwrap_err();
        assert_eq!(err.code(), Code::InvalidArgument);
        assert!(outside.0.join("keep.txt").exists());
    }

    #[tokio::test]
    async fn test_get_file_info() {
        let root = TestRoot::new();
        root.write("backups/b1/manifest.json", b"abc");
        root.write("empty.bin", b"");

        // known sha256 of "abc" and of the empty input
        let info = get_file_info(&root.0, "backups/b1/manifest.json")
            .await
            .unwrap();
        assert_eq!(info.size, 3);
        assert_eq!(
            info.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let info = get_file_info(&root.0, "empty.bin").await.unwrap();
        assert_eq!(info.size, 0);
        assert_eq!(
            info.sha256,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );

        // file bigger than one chunk
        let content = vec![7u8; CHUNK_SIZE * 2 + 1];
        root.write("big.bin", &content);
        let info = get_file_info(&root.0, "big.bin").await.unwrap();
        assert_eq!(info.size, content.len() as u64);
        assert_eq!(info.sha256, hex::encode(Sha256::digest(&content)));
    }

    #[tokio::test]
    async fn test_get_file_info_errors() {
        let root = TestRoot::new();
        root.write("dir/file.txt", b"content");

        let err = get_file_info(&root.0, "missing.txt").await.unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
        let err = get_file_info(&root.0, "dir").await.unwrap_err();
        assert_eq!(err.code(), Code::InvalidArgument);
        let err = get_file_info(&root.0, "../file.txt").await.unwrap_err();
        assert_eq!(err.code(), Code::InvalidArgument);

        let outside = TestRoot::new();
        outside.write("secret.txt", b"secret");
        std::os::unix::fs::symlink(outside.0.join("secret.txt"), root.0.join("link")).unwrap();
        let err = get_file_info(&root.0, "link").await.unwrap_err();
        assert_eq!(err.code(), Code::InvalidArgument);
    }
}
