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

//! Volumes of the local storage-backend.
//!
//! Every volume is a sparse file `volumes/<backend_id>.img` within the storage-location, which the
//! deployer exports over NFS to the sakura-hosts. A copy is written into
//! `volumes/<backend_id>.img.partial` and renamed at its end, so the state of a copy is visible
//! by its files, also after a restart of the onsen.

use std::fs::{File, Metadata, OpenOptions};
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use ainari_api_structs::storage_structs::{VolumeConnection, VolumeInfoResp, VolumeState};
use ainari_common::error::AinariError;

use super::VolumeBackend;
use super::local::{LocalBackend, NfsExport, VOLUME_DIRECTORY};

/// Block-size, which the size of a volume must be a multiple of, so it can be used as
/// block-device.
const VOLUME_BLOCK_SIZE: u64 = 512;

/// Size of the chunks, in which a volume is copied. Chunks, which only contain zeros, are skipped,
/// so the copy stays sparse.
const COPY_CHUNK_SIZE: usize = 1024 * 1024;

impl LocalBackend {
    /// Directory, which holds the volumes
    pub(super) fn volume_dir(&self) -> PathBuf {
        self.location.join(VOLUME_DIRECTORY)
    }

    /// Name of the file of a volume within the directory of the volumes
    fn volume_file_name(id: &Uuid) -> String {
        format!("{id}.img")
    }

    /// Path of the file of a volume. The name is built from the UUID, so it can never leave the
    /// directory of the volumes.
    fn volume_path(&self, id: &Uuid) -> PathBuf {
        self.volume_dir().join(Self::volume_file_name(id))
    }

    /// Path of the incomplete file of a volume, while a copy is written into it
    fn partial_path(&self, id: &Uuid) -> PathBuf {
        self.volume_dir().join(format!("{id}.img.partial"))
    }

    /// Returns the NFS-export, or `Conflict`, if the backend doesn't support volumes.
    fn nfs_export(&self) -> Result<&NfsExport, AinariError> {
        self.nfs.as_ref().ok_or_else(|| {
            AinariError::Conflict("volumes are not supported by this onsen".to_string())
        })
    }
}

/// Checks, that a size can be used for a volume.
fn check_volume_size(size: u64) -> Result<(), AinariError> {
    if size == 0 || size % VOLUME_BLOCK_SIZE != 0 {
        return Err(AinariError::InvalidInput(format!(
            "size of a volume must be a multiple of {VOLUME_BLOCK_SIZE} bytes and not 0"
        )));
    }
    Ok(())
}

/// Reads the metadata of a file without following symbolic links.
///
/// # Returns
///
/// The metadata, `None`, if the file doesn't exist, or an internal error.
fn file_metadata(path: &Path) -> Result<Option<Metadata>, AinariError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(Some(metadata)),
        Ok(_) => Err(AinariError::internal(format!(
            "volume {:?} is not a regular file",
            path
        ))),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(AinariError::internal(format!(
            "failed to read metadata of {:?}: {e}",
            path
        ))),
    }
}

/// Builds the information of a volume from the metadata of its file.
fn volume_info(id: &Uuid, state: VolumeState, metadata: &Metadata) -> VolumeInfoResp {
    VolumeInfoResp {
        backend_id: *id,
        state,
        size: metadata.len(),
        // the blocks are always counted in units of 512 bytes
        used_size: metadata.blocks() * 512,
    }
}

/// Removes a file, a not existing file is no error.
fn remove_file_if_exists(path: &Path) -> Result<(), AinariError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AinariError::internal(format!(
            "failed to delete {:?}: {e}",
            path
        ))),
    }
}

/// Copies a file into an already existing file and skips the chunks, which only contain zeros,
/// so the copy stays sparse.
///
/// # Arguments
///
/// * `source` - The file, which is copied
/// * `target` - The file, which gets the copy
///
/// # Returns
///
/// Ok, or the io-error, which made the copy fail.
fn copy_sparse(source: &Path, target: &Path) -> std::io::Result<()> {
    let mut source = File::open(source)?;
    let size = source.metadata()?.len();
    let mut target = OpenOptions::new().write(true).truncate(true).open(target)?;

    let mut buf = vec![0u8; COPY_CHUNK_SIZE];
    loop {
        // fill the whole chunk, so the zero-check covers whole chunks
        let mut filled = 0;
        while filled < buf.len() {
            let n = source.read(&mut buf[filled..])?;
            if n == 0 {
                break;
            }
            filled += n;
        }
        if filled == 0 {
            break;
        }

        let chunk = &buf[..filled];
        if chunk.iter().all(|byte| *byte == 0) {
            target.seek(SeekFrom::Current(filled as i64))?;
        } else {
            target.write_all(chunk)?;
        }
    }

    // a hole at the end is not written, so the size is set explicitly
    target.set_len(size)?;
    target.sync_all()
}

impl VolumeBackend for LocalBackend {
    fn supports_volumes(&self) -> bool {
        self.nfs.is_some()
    }

    async fn create_volume(&self, id: &Uuid, size: u64) -> Result<VolumeInfoResp, AinariError> {
        self.nfs_export()?;
        check_volume_size(size)?;

        let volume_dir = self.volume_dir();
        tokio::fs::create_dir_all(&volume_dir).await.map_err(|e| {
            AinariError::internal(format!(
                "failed to create volume-directory {:?}: {e}",
                volume_dir
            ))
        })?;

        let path = self.volume_path(id);
        // a running copy into the same volume is checked under the lock of the copies
        let running = self.running_copies.lock().expect("mutex poisoned");
        if running.contains(id) || file_metadata(&self.partial_path(id))?.is_some() {
            return Err(AinariError::Conflict(format!(
                "volume '{id}' is the target of a copy"
            )));
        }

        let file = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                // a repeated request with the same size is no error
                let metadata = file_metadata(&path)?.ok_or_else(|| {
                    AinariError::internal(format!("volume {:?} disappeared", path))
                })?;
                if metadata.len() != size {
                    return Err(AinariError::Conflict(format!(
                        "volume '{id}' already exists with another size"
                    )));
                }
                return Ok(volume_info(id, VolumeState::Ready, &metadata));
            }
            Err(e) => {
                return Err(AinariError::internal(format!(
                    "failed to create volume {:?}: {e}",
                    path
                )));
            }
        };
        drop(running);

        // only the size is set, so the file is sparse and uses no space yet
        let metadata = file
            .set_len(size)
            .and_then(|_| file.metadata())
            .map_err(|e| {
                let _ = std::fs::remove_file(&path);
                AinariError::internal(format!("failed to set size of volume {:?}: {e}", path))
            })?;
        log::info!("Created volume '{id}' with {size} bytes");

        Ok(volume_info(id, VolumeState::Ready, &metadata))
    }

    async fn delete_volume(&self, id: &Uuid) -> Result<(), AinariError> {
        self.nfs_export()?;

        // a running copy into the volume fails at its end, because its file is gone
        remove_file_if_exists(&self.partial_path(id))?;
        remove_file_if_exists(&self.volume_path(id))?;
        log::info!("Deleted volume '{id}'");

        Ok(())
    }

    async fn resize_volume(&self, id: &Uuid, size: u64) -> Result<VolumeInfoResp, AinariError> {
        self.nfs_export()?;
        check_volume_size(size)?;

        let path = self.volume_path(id);
        let metadata = file_metadata(&path)?
            .ok_or_else(|| AinariError::NotFound(format!("volume '{id}' not found")))?;
        if size < metadata.len() {
            return Err(AinariError::InvalidInput(format!(
                "volume '{id}' can only grow, but it has already {} bytes",
                metadata.len()
            )));
        }
        if size == metadata.len() {
            return Ok(volume_info(id, VolumeState::Ready, &metadata));
        }

        let file = OpenOptions::new()
            .write(true)
            .open(&path)
            .map_err(|e| AinariError::internal(format!("failed to open volume {:?}: {e}", path)))?;
        let metadata = file
            .set_len(size)
            .and_then(|_| file.metadata())
            .map_err(|e| {
                AinariError::internal(format!("failed to resize volume {:?}: {e}", path))
            })?;
        log::info!("Resized volume '{id}' to {size} bytes");

        Ok(volume_info(id, VolumeState::Ready, &metadata))
    }

    async fn volume_info(&self, id: &Uuid) -> Result<VolumeInfoResp, AinariError> {
        self.nfs_export()?;

        // the end of a copy renames its file under this lock, so the state is consistent
        let running = self.running_copies.lock().expect("mutex poisoned");
        if let Some(metadata) = file_metadata(&self.volume_path(id))? {
            return Ok(volume_info(id, VolumeState::Ready, &metadata));
        }

        let partial_path = self.partial_path(id);
        let Some(metadata) = file_metadata(&partial_path)? else {
            return Err(AinariError::NotFound(format!("volume '{id}' not found")));
        };
        if running.contains(id) {
            return Ok(volume_info(id, VolumeState::Copying, &metadata));
        }

        // an incomplete copy without a running copy failed or was interrupted by a restart
        remove_file_if_exists(&partial_path)?;
        log::warn!("Removed incomplete copy into volume '{id}'");
        Ok(volume_info(id, VolumeState::Failed, &metadata))
    }

    async fn copy_volume(&self, source_id: &Uuid, target_id: &Uuid) -> Result<(), AinariError> {
        self.nfs_export()?;
        if source_id == target_id {
            return Err(AinariError::InvalidInput(
                "a volume can't be copied into itself".to_string(),
            ));
        }

        let source = self.volume_path(source_id);
        let target = self.volume_path(target_id);
        let partial = self.partial_path(target_id);
        {
            let mut running = self.running_copies.lock().expect("mutex poisoned");
            if file_metadata(&source)?.is_none() {
                return Err(AinariError::NotFound(format!(
                    "volume '{source_id}' not found"
                )));
            }
            if running.contains(target_id)
                || file_metadata(&target)?.is_some()
                || file_metadata(&partial)?.is_some()
            {
                return Err(AinariError::Conflict(format!(
                    "volume '{target_id}' already exists"
                )));
            }

            // the incomplete file exists from the start, so the copy is visible immediately
            File::create_new(&partial).map_err(|e| {
                AinariError::internal(format!("failed to create {:?}: {e}", partial))
            })?;
            running.insert(*target_id);
        }

        let running_copies = self.running_copies.clone();
        let source_id = *source_id;
        let target_id = *target_id;
        tokio::task::spawn_blocking(move || {
            log::info!("Start copy of volume '{source_id}' into '{target_id}'");
            let result = copy_sparse(&source, &partial);

            // the rename and the end of the copy are done under the lock, so `volume_info` never
            // sees a state in between
            let mut running = running_copies.lock().expect("mutex poisoned");
            let result = result.and_then(|_| std::fs::rename(&partial, &target));
            running.remove(&target_id);
            match result {
                Ok(()) => log::info!("Copied volume '{source_id}' into '{target_id}'"),
                Err(e) => {
                    log::error!("Failed to copy volume '{source_id}' into '{target_id}': {e}")
                }
            }
        });

        Ok(())
    }

    async fn volume_connection(&self, id: &Uuid) -> Result<VolumeConnection, AinariError> {
        let nfs = self.nfs_export()?;

        let info = self.volume_info(id).await?;
        if info.state != VolumeState::Ready {
            return Err(AinariError::Conflict(format!("volume '{id}' is not ready")));
        }

        Ok(VolumeConnection::Nfs {
            server: nfs.server.clone(),
            export: nfs.export.clone(),
            file_name: Self::volume_file_name(id),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    const MIB: u64 = 1024 * 1024;

    /// Local backend with volumes on a temporary storage-location, which is deleted at the end of
    /// the test.
    struct TestBackend(LocalBackend);

    impl TestBackend {
        fn new(supports_volumes: bool) -> Self {
            let location = std::env::temp_dir().join(format!("onsen_test_{}", Uuid::new_v4()));
            std::fs::create_dir_all(location.join(VOLUME_DIRECTORY)).unwrap();
            TestBackend(LocalBackend {
                location,
                nfs: supports_volumes.then(|| NfsExport {
                    server: "10.0.0.5".to_string(),
                    export: "/srv/onsen/volumes".to_string(),
                }),
                running_copies: Arc::default(),
            })
        }

        /// Waits, until the copy into the volume is not running anymore.
        async fn wait_for_copy(&self, id: &Uuid) {
            for _ in 0..200 {
                if !self.0.running_copies.lock().unwrap().contains(id) {
                    return;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            panic!("copy into volume '{id}' didn't finish");
        }
    }

    impl Drop for TestBackend {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0.location);
        }
    }

    fn is_conflict<T>(result: Result<T, AinariError>) -> bool {
        matches!(result, Err(AinariError::Conflict(_)))
    }

    #[tokio::test]
    async fn test_create_resize_delete() {
        let backend = TestBackend::new(true);
        let id = Uuid::new_v4();

        // the volume is sparse
        let info = backend.0.create_volume(&id, 512 * MIB).await.unwrap();
        assert_eq!(info.size, 512 * MIB);
        assert!(info.used_size < MIB);
        assert_eq!(backend.0.volume_info(&id).await.unwrap(), info);

        for size in [0, 511, MIB + 1] {
            assert!(matches!(
                backend.0.create_volume(&Uuid::new_v4(), size).await,
                Err(AinariError::InvalidInput(_))
            ));
        }

        assert_eq!(
            backend.0.resize_volume(&id, MIB * 1024).await.unwrap().size,
            MIB * 1024
        );
        // the same size is no change
        assert_eq!(
            backend.0.resize_volume(&id, MIB * 1024).await.unwrap().size,
            MIB * 1024
        );

        backend.0.delete_volume(&id).await.unwrap();
        assert!(!backend.0.volume_path(&id).exists());
        backend.0.delete_volume(&id).await.unwrap();
    }

    #[tokio::test]
    async fn test_volumes_not_supported() {
        let backend = TestBackend::new(false);
        let id = Uuid::new_v4();

        assert!(!backend.0.supports_volumes());
        assert!(is_conflict(backend.0.create_volume(&id, MIB).await));
        assert!(is_conflict(backend.0.volume_info(&id).await));
        assert!(is_conflict(backend.0.resize_volume(&id, MIB).await));
        assert!(is_conflict(backend.0.delete_volume(&id).await));
        assert!(is_conflict(
            backend.0.copy_volume(&id, &Uuid::new_v4()).await
        ));
        assert!(is_conflict(backend.0.volume_connection(&id).await));
    }

    #[tokio::test]
    async fn test_copy_is_sparse_and_complete() {
        let backend = TestBackend::new(true);
        let source = Uuid::new_v4();
        let target = Uuid::new_v4();

        // data at the start, in the middle and at the end, holes in between
        backend.0.create_volume(&source, 64 * MIB).await.unwrap();
        {
            let mut file = OpenOptions::new()
                .write(true)
                .open(backend.0.volume_path(&source))
                .unwrap();
            for offset in [0, 20 * MIB + 7, 64 * MIB - 3] {
                file.seek(SeekFrom::Start(offset)).unwrap();
                file.write_all(b"abc").unwrap();
            }
        }

        backend.0.copy_volume(&source, &target).await.unwrap();
        backend.wait_for_copy(&target).await;

        let info = backend.0.volume_info(&target).await.unwrap();
        assert_eq!(info.state, VolumeState::Ready);
        assert_eq!(info.size, 64 * MIB);
        assert!(info.used_size < 8 * MIB, "used size: {}", info.used_size);
        assert_eq!(
            std::fs::read(backend.0.volume_path(&target)).unwrap(),
            std::fs::read(backend.0.volume_path(&source)).unwrap()
        );
        assert!(!backend.0.partial_path(&target).exists());
    }

    #[tokio::test]
    async fn test_copy_errors() {
        let backend = TestBackend::new(true);
        let source = Uuid::new_v4();
        let target = Uuid::new_v4();
        backend.0.create_volume(&source, MIB).await.unwrap();
        backend.0.create_volume(&target, MIB).await.unwrap();

        assert!(matches!(
            backend.0.copy_volume(&source, &source).await,
            Err(AinariError::InvalidInput(_))
        ));
        assert!(matches!(
            backend
                .0
                .copy_volume(&Uuid::new_v4(), &Uuid::new_v4())
                .await,
            Err(AinariError::NotFound(_))
        ));
        // an existing target is never overwritten
        assert!(is_conflict(backend.0.copy_volume(&source, &target).await));
    }

    #[tokio::test]
    async fn test_copy_states() {
        let backend = TestBackend::new(true);
        let id = Uuid::new_v4();
        std::fs::write(backend.0.partial_path(&id), b"part").unwrap();

        // a running copy
        backend.0.running_copies.lock().unwrap().insert(id);
        let info = backend.0.volume_info(&id).await.unwrap();
        assert_eq!(info.state, VolumeState::Copying);
        assert_eq!(info.size, 4);
        assert!(is_conflict(backend.0.volume_connection(&id).await));
        assert!(is_conflict(backend.0.create_volume(&id, MIB).await));
        let source = Uuid::new_v4();
        backend.0.create_volume(&source, MIB).await.unwrap();
        assert!(is_conflict(backend.0.copy_volume(&source, &id).await));

        // the onsen was restarted, so the incomplete copy runs no more
        backend.0.running_copies.lock().unwrap().clear();
        assert!(is_conflict(backend.0.create_volume(&id, MIB).await));
        assert_eq!(
            backend.0.volume_info(&id).await.unwrap().state,
            VolumeState::Failed
        );
        assert!(!backend.0.partial_path(&id).exists());
        assert!(matches!(
            backend.0.volume_info(&id).await,
            Err(AinariError::NotFound(_))
        ));

        // afterwards the volume can be created again
        backend.0.create_volume(&id, MIB).await.unwrap();
    }

    #[tokio::test]
    async fn test_delete_during_copy() {
        let backend = TestBackend::new(true);
        let id = Uuid::new_v4();
        std::fs::write(backend.0.partial_path(&id), b"part").unwrap();
        backend.0.running_copies.lock().unwrap().insert(id);

        backend.0.delete_volume(&id).await.unwrap();
        assert!(!backend.0.partial_path(&id).exists());
    }

    #[tokio::test]
    async fn test_connection() {
        let backend = TestBackend::new(true);
        let id = Uuid::new_v4();
        backend.0.create_volume(&id, MIB).await.unwrap();

        assert_eq!(
            backend.0.volume_connection(&id).await.unwrap(),
            VolumeConnection::Nfs {
                server: "10.0.0.5".to_string(),
                export: "/srv/onsen/volumes".to_string(),
                file_name: format!("{id}.img"),
            }
        );
        assert!(matches!(
            backend.0.volume_connection(&Uuid::new_v4()).await,
            Err(AinariError::NotFound(_))
        ));
    }
}
