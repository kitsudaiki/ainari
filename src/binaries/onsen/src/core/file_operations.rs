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

use actix_web::web::{Bytes, BytesMut};
use futures_util::{Stream, StreamExt, stream};
use sha2::{Digest, Sha256};
use std::fmt::Display;
use std::fs::File;
use std::io::{ErrorKind, Read};
use std::path::{Component, Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use ainari_api_structs::storage_structs::{StorageDirectoryEntry, StorageFileInfoResp};
use ainari_common::constants::CHUNK_SIZE;
use ainari_common::error::AinariError;
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
/// The absolute path, or `InvalidInput`, if the remote path would leave the storage-location.
fn resolve_path(root: &Path, remote_path: &str) -> Result<PathBuf, AinariError> {
    let remote_path = Path::new(remote_path);
    if !is_safe_subpath(remote_path) {
        return Err(AinariError::InvalidInput(format!(
            "provided remote-path is invalid: {:?}",
            remote_path
        )));
    }
    Ok(root.join(remote_path))
}

/// Builds the absolute path of a remote path within the storage-location, which must not be the
/// storage-location itself, like a file or a directory, which is deleted.
fn resolve_entry_path(root: &Path, remote_path: &str) -> Result<PathBuf, AinariError> {
    // prevent, that the storage-location itself is addressed by an empty path or by "."
    let has_normal_component = Path::new(remote_path)
        .components()
        .any(|comp| matches!(comp, Component::Normal(_)));
    if !has_normal_component {
        return Err(AinariError::InvalidInput(
            "the storage-location itself can not be addressed".to_string(),
        ));
    }
    resolve_path(root, remote_path)
}

/// Writes the content of an upload into a file. An existing file is overwritten.
///
/// The upload is written chunk by chunk, so also large files are never held in memory. If the
/// upload breaks off, the incomplete file is deleted again, so no part of a file is kept.
///
/// # Arguments
///
/// * `root` - The storage-location
/// * `remote_file_path` - Relative path of the file
/// * `upload` - The content of the file
///
/// # Returns
///
/// The size of the written file, `InvalidInput`, if the path is invalid or the upload broke off.
pub async fn write_file<S, E>(
    root: &Path,
    remote_file_path: &str,
    mut upload: S,
) -> Result<u64, AinariError>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
    E: Display,
{
    let file_path = resolve_entry_path(root, remote_file_path)?;

    // create directory of target-file, if not already exist
    if let Some(parent) = file_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            AinariError::internal(format!("failed to create directory {:?}: {e}", parent))
        })?;
    }
    let mut file = tokio::fs::File::create(&file_path)
        .await
        .map_err(|e| AinariError::internal(format!("failed to open file {:?}: {e}", file_path)))?;

    let result = async {
        let mut size: u64 = 0;
        while let Some(chunk) = upload.next().await {
            let chunk = chunk.map_err(|e| {
                AinariError::InvalidInput(format!(
                    "upload of file {:?} broke off: {e}",
                    remote_file_path
                ))
            })?;
            file.write_all(&chunk)
                .await
                .map_err(|e| AinariError::internal(format!("write error: {e}")))?;
            size += chunk.len() as u64;
        }
        file.flush()
            .await
            .map_err(|e| AinariError::internal(format!("flush error: {e}")))?;
        Ok(size)
    }
    .await;

    if result.is_err() {
        if let Err(e) = tokio::fs::remove_file(&file_path).await {
            log::error!("Failed to delete incomplete file {:?}: {e}", file_path);
        }
    }
    result
}

/// Opens a file for its download.
///
/// Symbolic links are not followed, so no file outside of the storage-location is read.
///
/// # Arguments
///
/// * `root` - The storage-location
/// * `remote_file_path` - Relative path of the file
///
/// # Returns
///
/// The size of the file and a stream of its content, `NotFound`, if the file doesn't exist, or
/// `InvalidInput`, if the path is invalid or not a file.
pub async fn read_file(
    root: &Path,
    remote_file_path: &str,
) -> Result<
    (
        u64,
        impl Stream<Item = Result<Bytes, std::io::Error>> + 'static,
    ),
    AinariError,
> {
    let file_path = resolve_entry_path(root, remote_file_path)?;
    let not_found = || AinariError::NotFound(format!("file not found: {:?}", remote_file_path));

    let metadata = tokio::fs::symlink_metadata(&file_path)
        .await
        .map_err(|e| match e.kind() {
            ErrorKind::NotFound => not_found(),
            _ => AinariError::internal(format!("failed to read metadata: {e}")),
        })?;
    if !metadata.is_file() {
        return Err(AinariError::InvalidInput(format!(
            "not a file: {:?}",
            remote_file_path
        )));
    }
    let file = tokio::fs::File::open(&file_path)
        .await
        .map_err(|e| match e.kind() {
            ErrorKind::NotFound => not_found(),
            _ => AinariError::internal(format!("failed to open file: {e}")),
        })?;

    let content = stream::try_unfold(file, |mut file| async move {
        let mut buf = BytesMut::zeroed(CHUNK_SIZE);
        let n = file.read(&mut buf).await?;
        if n == 0 {
            return Ok(None);
        }
        buf.truncate(n);
        Ok(Some((buf.freeze(), file)))
    });
    Ok((metadata.len(), content))
}

/// Deletes a file. Deleting a not existing file is no error, so a failed deletion can simply be
/// repeated.
///
/// # Arguments
///
/// * `root` - The storage-location
/// * `remote_file_path` - Relative path of the file
///
/// # Returns
///
/// Ok, or `InvalidInput`, if the path is invalid or a directory.
pub async fn delete_file(root: &Path, remote_file_path: &str) -> Result<(), AinariError> {
    let file_path = resolve_entry_path(root, remote_file_path)?;

    match tokio::fs::symlink_metadata(&file_path).await {
        Ok(metadata) if metadata.is_dir() => {
            return Err(AinariError::InvalidInput(format!(
                "not a file: {:?}",
                remote_file_path
            )));
        }
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            return Err(AinariError::internal(format!(
                "failed to read metadata: {e}"
            )));
        }
    }

    // a symbolic link is removed itself and not followed
    match tokio::fs::remove_file(&file_path).await {
        Ok(()) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        Err(e) => {
            return Err(AinariError::internal(format!(
                "failed to delete file {:?}: {e}",
                file_path
            )));
        }
    }
    log::info!("Deleted file: {:?}", file_path);

    Ok(())
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
/// The entries, `NotFound`, if the directory doesn't exist, or `InvalidInput`, if the path is
/// invalid or not a directory.
pub async fn list_directory(
    root: &Path,
    remote_dir_path: &str,
) -> Result<Vec<StorageDirectoryEntry>, AinariError> {
    let dir_path = resolve_path(root, remote_dir_path)?;

    let mut read_dir = tokio::fs::read_dir(&dir_path)
        .await
        .map_err(|e| match e.kind() {
            ErrorKind::NotFound => {
                AinariError::NotFound(format!("directory not found: {:?}", remote_dir_path))
            }
            ErrorKind::NotADirectory => {
                AinariError::InvalidInput(format!("not a directory: {:?}", remote_dir_path))
            }
            _ => AinariError::internal(format!("failed to read directory: {e}")),
        })?;

    let mut entries = Vec::new();
    while let Some(entry) = read_dir
        .next_entry()
        .await
        .map_err(|e| AinariError::internal(format!("failed to read directory: {e}")))?
    {
        // `DirEntry::metadata` doesn't follow symbolic links
        let metadata = entry
            .metadata()
            .await
            .map_err(|e| AinariError::internal(format!("failed to read metadata: {e}")))?;
        let is_dir = metadata.is_dir();
        entries.push(StorageDirectoryEntry {
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
/// Ok, or `InvalidInput`, if the path is invalid, the storage-location itself or not a
/// directory.
pub async fn delete_directory(root: &Path, remote_dir_path: &str) -> Result<(), AinariError> {
    let dir_path = resolve_entry_path(root, remote_dir_path)?;

    match tokio::fs::symlink_metadata(&dir_path).await {
        Ok(metadata) if !metadata.is_dir() => {
            return Err(AinariError::InvalidInput(format!(
                "not a directory: {:?}",
                remote_dir_path
            )));
        }
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            return Err(AinariError::internal(format!(
                "failed to read metadata: {e}"
            )));
        }
    }

    tokio::fs::remove_dir_all(&dir_path).await.map_err(|e| {
        AinariError::internal(format!("failed to delete directory {:?}: {e}", dir_path))
    })?;
    log::info!("Deleted directory: {:?}", dir_path);

    Ok(())
}

/// Returns the size and the sha256-checksum of a file.
///
/// The file is hashed on the blocking thread-pool, because it can be a disk-image with many
/// gigabytes, which would otherwise block the http-server.
///
/// # Arguments
///
/// * `root` - The storage-location
/// * `remote_file_path` - Relative path of the file
///
/// # Returns
///
/// The info, `NotFound`, if the file doesn't exist, or `InvalidInput`, if the path is invalid
/// or not a file.
pub async fn get_file_info(
    root: &Path,
    remote_file_path: &str,
) -> Result<StorageFileInfoResp, AinariError> {
    let file_path = resolve_entry_path(root, remote_file_path)?;
    let remote_file_path = remote_file_path.to_owned();

    tokio::task::spawn_blocking(move || {
        let not_found = || AinariError::NotFound(format!("file not found: {:?}", remote_file_path));

        // symbolic links are not followed, so no file outside of the storage-location is read
        let metadata = std::fs::symlink_metadata(&file_path).map_err(|e| match e.kind() {
            ErrorKind::NotFound => not_found(),
            _ => AinariError::internal(format!("failed to read metadata: {e}")),
        })?;
        if !metadata.is_file() {
            return Err(AinariError::InvalidInput(format!(
                "not a file: {:?}",
                remote_file_path
            )));
        }

        let mut file = File::open(&file_path).map_err(|e| match e.kind() {
            ErrorKind::NotFound => not_found(),
            _ => AinariError::internal(format!("failed to open file: {e}")),
        })?;

        // the size is counted while hashing, so it matches the hashed content, even if the file
        // was changed in between
        let mut hasher = Sha256::new();
        let mut buf = vec![0u8; CHUNK_SIZE];
        let mut size: u64 = 0;
        loop {
            let n = file
                .read(&mut buf)
                .map_err(|e| AinariError::internal(format!("read error: {e}")))?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            size += n as u64;
        }

        Ok(StorageFileInfoResp {
            size,
            sha256: hex::encode(hasher.finalize()),
        })
    })
    .await
    .map_err(|e| AinariError::internal(format!("failed to hash file: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_not_found<T>(result: Result<T, AinariError>) -> bool {
        matches!(result, Err(AinariError::NotFound(_)))
    }

    fn is_invalid<T>(result: Result<T, AinariError>) -> bool {
        matches!(result, Err(AinariError::InvalidInput(_)))
    }

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

        assert!(is_not_found(list_directory(&root.0, "missing").await));
        assert!(is_invalid(list_directory(&root.0, "file.txt").await));
        assert!(is_invalid(list_directory(&root.0, "../").await));
        assert!(is_invalid(list_directory(&root.0, "/etc").await));
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
            assert!(
                is_invalid(delete_directory(&root.0, path).await),
                "path: {path:?}"
            );
        }
        assert!(root.0.join("file.txt").exists());

        assert!(is_invalid(delete_directory(&root.0, "file.txt").await));
        assert!(root.0.join("file.txt").exists());

        assert!(is_invalid(delete_directory(&root.0, "../other").await));
        assert!(is_invalid(delete_directory(&root.0, "/tmp").await));
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
        assert!(is_invalid(delete_directory(&root.0, "link").await));
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

        assert!(is_not_found(get_file_info(&root.0, "missing.txt").await));
        assert!(is_invalid(get_file_info(&root.0, "dir").await));
        assert!(is_invalid(get_file_info(&root.0, "../file.txt").await));

        let outside = TestRoot::new();
        outside.write("secret.txt", b"secret");
        std::os::unix::fs::symlink(outside.0.join("secret.txt"), root.0.join("link")).unwrap();
        assert!(is_invalid(get_file_info(&root.0, "link").await));
    }

    /// Returns the given chunks as upload, which breaks off with an error at the end, if
    /// `broken` is set.
    fn upload(
        chunks: Vec<&'static [u8]>,
        broken: bool,
    ) -> impl Stream<Item = Result<Bytes, String>> + Unpin {
        let mut items: Vec<Result<Bytes, String>> = chunks
            .into_iter()
            .map(|chunk| Ok(Bytes::from_static(chunk)))
            .collect();
        if broken {
            items.push(Err("connection lost".to_string()));
        }
        stream::iter(items)
    }

    /// Reads the whole content of a downloaded file.
    async fn read_all(root: &Path, remote_file_path: &str) -> Vec<u8> {
        let (size, content) = read_file(root, remote_file_path).await.unwrap();
        let chunks: Vec<_> = content.collect().await;
        let data: Vec<u8> = chunks.into_iter().flat_map(|c| c.unwrap()).collect();
        assert_eq!(size, data.len() as u64);
        data
    }

    #[tokio::test]
    async fn test_write_and_read_file() {
        let root = TestRoot::new();

        let size = write_file(
            &root.0,
            "images/a/image.bin",
            upload(vec![b"abc", b"def"], false),
        )
        .await
        .unwrap();
        assert_eq!(size, 6);
        assert_eq!(read_all(&root.0, "images/a/image.bin").await, b"abcdef");

        // an existing file is overwritten
        write_file(&root.0, "images/a/image.bin", upload(vec![b"x"], false))
            .await
            .unwrap();
        assert_eq!(read_all(&root.0, "images/a/image.bin").await, b"x");

        // an empty file
        write_file(&root.0, "empty.bin", upload(vec![], false))
            .await
            .unwrap();
        assert!(read_all(&root.0, "empty.bin").await.is_empty());

        // file bigger than one chunk
        let content = vec![3u8; CHUNK_SIZE * 2 + 5];
        root.write("big.bin", &content);
        assert_eq!(read_all(&root.0, "big.bin").await, content);
    }

    #[tokio::test]
    async fn test_write_file_removes_broken_upload() {
        let root = TestRoot::new();
        assert!(is_invalid(
            write_file(&root.0, "images/broken.bin", upload(vec![b"abc"], true)).await
        ));
        assert!(!root.0.join("images/broken.bin").exists());
    }

    #[tokio::test]
    async fn test_write_and_read_file_errors() {
        let root = TestRoot::new();
        root.write("dir/file.txt", b"content");

        for path in ["", ".", "../file.bin", "/tmp/file.bin"] {
            assert!(
                is_invalid(write_file(&root.0, path, upload(vec![b"abc"], false)).await),
                "path: {path:?}"
            );
            assert!(is_invalid(read_file(&root.0, path).await), "path: {path:?}");
        }
        assert!(is_not_found(read_file(&root.0, "missing.bin").await));
        assert!(is_invalid(read_file(&root.0, "dir").await));

        let outside = TestRoot::new();
        outside.write("secret.txt", b"secret");
        std::os::unix::fs::symlink(outside.0.join("secret.txt"), root.0.join("link")).unwrap();
        assert!(is_invalid(read_file(&root.0, "link").await));
    }

    #[tokio::test]
    async fn test_delete_file() {
        let root = TestRoot::new();
        root.write("images/a.bin", b"a");
        root.write("images/b.bin", b"b");

        delete_file(&root.0, "images/a.bin").await.unwrap();
        assert!(!root.0.join("images/a.bin").exists());
        assert!(root.0.join("images/b.bin").exists());

        // deleting again is no error
        delete_file(&root.0, "images/a.bin").await.unwrap();

        assert!(is_invalid(delete_file(&root.0, "images").await));
        assert!(is_invalid(delete_file(&root.0, "").await));
        assert!(is_invalid(delete_file(&root.0, "../images/b.bin").await));

        // a symlink is removed, but not followed
        let outside = TestRoot::new();
        outside.write("keep.txt", b"keep");
        std::os::unix::fs::symlink(outside.0.join("keep.txt"), root.0.join("link")).unwrap();
        delete_file(&root.0, "link").await.unwrap();
        assert!(outside.0.join("keep.txt").exists());
    }
}
