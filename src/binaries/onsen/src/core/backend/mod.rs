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

//! Storage-backends of the onsen.
//!
//! Every onsen uses exactly one backend, which is selected by the config. A backend implements
//! the trait `FileBackend` for the files of the images, snapshots and backups, and the trait
//! `VolumeBackend` for the volumes, which the sakura-hosts attach as disks. The endpoints only
//! use the enum `Backend`, so a new backend doesn't change the REST-API of the onsen.

pub mod local;
mod local_volume;

use actix_web::web::Bytes;
use futures_util::Stream;
use futures_util::stream::BoxStream;
use std::fmt::Display;

use uuid::Uuid;

use ainari_api_structs::storage_structs::{
    StorageBackendResp, StorageBackendType, StorageDirectoryEntry, StorageFileInfoResp,
    VolumeConnection, VolumeInfoResp,
};
use ainari_common::error::AinariError;

use crate::config::StorageBackend;
use local::LocalBackend;

/// Content of a file, which is streamed to the caller.
pub type FileContent = BoxStream<'static, Result<Bytes, std::io::Error>>;

/// Files of the images, snapshots and backups.
///
/// All paths are relative to the root of the files within the backend and are checked by the
/// backend, so no path can leave this root.
pub trait FileBackend {
    /// Writes the content of an upload into a file, an existing file is overwritten.
    /// An upload, which breaks off, leaves no file.
    ///
    /// # Returns
    ///
    /// The size of the written file.
    async fn write_file<S, E>(&self, remote_file_path: &str, upload: S) -> Result<u64, AinariError>
    where
        S: Stream<Item = Result<Bytes, E>> + Unpin,
        E: Display;

    /// Opens a file for its download.
    ///
    /// # Returns
    ///
    /// The size of the file and a stream of its content.
    async fn read_file(&self, remote_file_path: &str) -> Result<(u64, FileContent), AinariError>;

    /// Deletes a file, deleting a not existing file is no error.
    async fn delete_file(&self, remote_file_path: &str) -> Result<(), AinariError>;

    /// Lists the direct entries of a directory, sorted by name. An empty path is the root.
    async fn list_directory(
        &self,
        remote_dir_path: &str,
    ) -> Result<Vec<StorageDirectoryEntry>, AinariError>;

    /// Deletes a directory with all of its content, deleting a not existing directory is no error.
    async fn delete_directory(&self, remote_dir_path: &str) -> Result<(), AinariError>;

    /// Returns the size and the sha256-checksum of a file.
    async fn get_file_info(
        &self,
        remote_file_path: &str,
    ) -> Result<StorageFileInfoResp, AinariError>;
}

/// Volumes, which are stored in the backend and attached by the sakura-hosts as disks.
///
/// A volume is only addressed by its name within the backend (`id`), never by the UUID of the
/// volume in ryokan. The content of a volume is always encrypted by the sakura-hosts, so the
/// backend never sees plain data. Nothing of this trait is specific to a single backend.
pub trait VolumeBackend {
    /// True, if the backend provides volumes. Without it, all other functions fail with
    /// `Conflict`.
    fn supports_volumes(&self) -> bool;

    /// Creates an empty volume. Creating an existing volume with the same size is no error.
    async fn create_volume(&self, id: &Uuid, size: u64) -> Result<VolumeInfoResp, AinariError>;

    /// Deletes a volume, deleting a not existing volume is no error.
    async fn delete_volume(&self, id: &Uuid) -> Result<(), AinariError>;

    /// Grows a volume, a volume can never shrink.
    async fn resize_volume(&self, id: &Uuid, size: u64) -> Result<VolumeInfoResp, AinariError>;

    /// Returns the information about a volume, which includes the state of a copy into it.
    async fn volume_info(&self, id: &Uuid) -> Result<VolumeInfoResp, AinariError>;

    /// Starts the copy of a whole volume into a new volume in the background. The state of the
    /// copy is reported by `volume_info` of the target.
    async fn copy_volume(&self, source_id: &Uuid, target_id: &Uuid) -> Result<(), AinariError>;

    /// Returns the information, how a sakura-host connects to a volume.
    async fn volume_connection(&self, id: &Uuid) -> Result<VolumeConnection, AinariError>;
}

/// Storage-backend of this onsen, which is selected by the config.
#[derive(Debug)]
pub enum Backend {
    /// Files on the local disk of the onsen-host
    Local(LocalBackend),
}

impl Backend {
    /// Creates the backend, which is selected by the config, and prepares its storage.
    ///
    /// # Arguments
    ///
    /// * `config` - Selected backend with its configuration
    ///
    /// # Returns
    ///
    /// The backend, or an error, if its storage can't be prepared.
    pub async fn new(config: &StorageBackend) -> Result<Self, AinariError> {
        match config {
            StorageBackend::Local(local) => Ok(Backend::Local(LocalBackend::new(local).await?)),
        }
    }

    /// Type of the backend
    pub fn backend_type(&self) -> StorageBackendType {
        match self {
            Backend::Local(_) => StorageBackendType::Local,
        }
    }

    /// Returns the type, the volume-support and the capacity of the backend.
    pub fn backend_info(&self) -> Result<StorageBackendResp, AinariError> {
        match self {
            Backend::Local(backend) => backend.backend_info(),
        }
    }
}

impl FileBackend for Backend {
    async fn write_file<S, E>(&self, remote_file_path: &str, upload: S) -> Result<u64, AinariError>
    where
        S: Stream<Item = Result<Bytes, E>> + Unpin,
        E: Display,
    {
        match self {
            Backend::Local(backend) => backend.write_file(remote_file_path, upload).await,
        }
    }

    async fn read_file(&self, remote_file_path: &str) -> Result<(u64, FileContent), AinariError> {
        match self {
            Backend::Local(backend) => backend.read_file(remote_file_path).await,
        }
    }

    async fn delete_file(&self, remote_file_path: &str) -> Result<(), AinariError> {
        match self {
            Backend::Local(backend) => backend.delete_file(remote_file_path).await,
        }
    }

    async fn list_directory(
        &self,
        remote_dir_path: &str,
    ) -> Result<Vec<StorageDirectoryEntry>, AinariError> {
        match self {
            Backend::Local(backend) => backend.list_directory(remote_dir_path).await,
        }
    }

    async fn delete_directory(&self, remote_dir_path: &str) -> Result<(), AinariError> {
        match self {
            Backend::Local(backend) => backend.delete_directory(remote_dir_path).await,
        }
    }

    async fn get_file_info(
        &self,
        remote_file_path: &str,
    ) -> Result<StorageFileInfoResp, AinariError> {
        match self {
            Backend::Local(backend) => backend.get_file_info(remote_file_path).await,
        }
    }
}

impl VolumeBackend for Backend {
    fn supports_volumes(&self) -> bool {
        match self {
            Backend::Local(backend) => backend.supports_volumes(),
        }
    }

    async fn create_volume(&self, id: &Uuid, size: u64) -> Result<VolumeInfoResp, AinariError> {
        match self {
            Backend::Local(backend) => backend.create_volume(id, size).await,
        }
    }

    async fn delete_volume(&self, id: &Uuid) -> Result<(), AinariError> {
        match self {
            Backend::Local(backend) => backend.delete_volume(id).await,
        }
    }

    async fn resize_volume(&self, id: &Uuid, size: u64) -> Result<VolumeInfoResp, AinariError> {
        match self {
            Backend::Local(backend) => backend.resize_volume(id, size).await,
        }
    }

    async fn volume_info(&self, id: &Uuid) -> Result<VolumeInfoResp, AinariError> {
        match self {
            Backend::Local(backend) => backend.volume_info(id).await,
        }
    }

    async fn copy_volume(&self, source_id: &Uuid, target_id: &Uuid) -> Result<(), AinariError> {
        match self {
            Backend::Local(backend) => backend.copy_volume(source_id, target_id).await,
        }
    }

    async fn volume_connection(&self, id: &Uuid) -> Result<VolumeConnection, AinariError> {
        match self {
            Backend::Local(backend) => backend.volume_connection(id).await,
        }
    }
}
