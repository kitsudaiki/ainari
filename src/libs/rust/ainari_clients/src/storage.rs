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

//! Clients of the internal endpoints of a storage-pool (onsen), which stores files under paths
//! relative to its storage-location.
//!
//! The content of the files is streamed in both directions, so also large disk-images are never
//! held in memory. Every upload announces its size in the header `Content-Length`, so the server
//! detects an upload, which broke off, and doesn't keep the incomplete file.

use std::time::Duration;

use actix_web::dev::{Decompress, Payload};
use actix_web::web::{Bytes, BytesMut};
use awc::body::SizedStream;
use awc::error::SendRequestError;
use awc::http::StatusCode;
use awc::{ClientRequest, ClientResponse};
use futures_util::{Stream, StreamExt, stream};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use ainari_api_structs::storage_structs::*;
use ainari_common::constants::CHUNK_SIZE;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::{handle_empty_response, handle_response, prepare_client};

/// Timeout of the requests, which transfer or hash a whole file. The timeout of a request covers
/// the time until the response arrives, which is only sent after an upload is complete or a file
/// was hashed completely, so it has to be long enough for large disk-images.
const TRANSFER_TIMEOUT: Duration = Duration::from_secs(6 * 60 * 60);

/// Maximum size of the body of an error-response, which is read into the error-message.
const MAX_ERROR_BODY_SIZE: usize = 64 * 1024;

/// Endpoint of the files within the storage-location.
const FILE_ENDPOINT: &str = "/v1alpha/file/internal";
/// Endpoint of the size and checksum of the files within the storage-location.
const FILE_INFO_ENDPOINT: &str = "/v1alpha/file/info/internal";
/// Endpoint of the directories within the storage-location.
const DIRECTORY_ENDPOINT: &str = "/v1alpha/directory/internal";

/// Builds an authorized request against an endpoint of the storage, which addresses the given
/// path.
///
/// # Arguments
///
/// * `request` - The request without url-query and headers
/// * `token` - Authentication token of the user, on whose behalf the file is accessed
/// * `internal_api_key` - Internal API key for authorization
/// * `path` - Path of the file or directory relative to the storage-location
///
/// # Returns
///
/// The request, or an `AinariError`, if the path can't be encoded.
fn authorized(
    request: ClientRequest,
    token: &str,
    internal_api_key: &Secret,
    path: &str,
) -> Result<ClientRequest, AinariError> {
    let query = StoragePathQuery {
        path: path.to_owned(),
    };
    let request = request.query(&query).map_err(|e| {
        AinariError::internal(format!("Failed to encode storage-path '{path}': {e}"))
    })?;
    Ok(request
        .insert_header(("Authorization", format!("Bearer {token}")))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal())))
}

/// Converts a failed response of the storage into an error, so `NotFound` and `InvalidInput`
/// can be handled by the caller.
///
/// # Arguments
///
/// * `action` - Description of the request for the error-message
/// * `status` - Status-code of the response
/// * `body` - Body of the response
///
/// # Returns
///
/// The error, which matches the status-code.
fn status_to_error(action: &str, status: StatusCode, body: &[u8]) -> AinariError {
    let msg = format!(
        "Failed to {action} on storage: {status} {}",
        String::from_utf8_lossy(body)
    );
    match status {
        StatusCode::NOT_FOUND => AinariError::NotFound(msg),
        StatusCode::BAD_REQUEST => AinariError::InvalidInput(msg),
        StatusCode::UNAUTHORIZED => AinariError::Unauthorized(msg),
        _ => AinariError::internal(msg),
    }
}

/// Checks the response of a request, which streams the content of a file.
///
/// # Arguments
///
/// * `response` - The response of the request
/// * `action` - Description of the request for the error-message
///
/// # Returns
///
/// The response with the status `200 OK`, whose body can be streamed, or the matching error.
async fn successful_response(
    response: Result<ClientResponse<Decompress<Payload>>, SendRequestError>,
    action: &str,
) -> Result<ClientResponse<Decompress<Payload>>, AinariError> {
    let mut response = response
        .map_err(|e| AinariError::internal(format!("Failed to {action} on storage: {e}")))?;
    if response.status() == StatusCode::OK {
        return Ok(response);
    }
    let body = response
        .body()
        .limit(MAX_ERROR_BODY_SIZE)
        .await
        .unwrap_or_default();
    Err(status_to_error(action, response.status(), &body))
}

/// Returns the size of the content of a response, which the storage always sends in the header
/// `Content-Length` for the content of a file.
fn content_length(
    response: &ClientResponse<Decompress<Payload>>,
    remote_file_path: &str,
) -> Result<u64, AinariError> {
    response
        .headers()
        .get(awc::http::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| {
            AinariError::internal(format!(
                "Download of file '{remote_file_path}' has no valid content-length"
            ))
        })
}

/// Reads a local file chunk by chunk as stream.
fn file_stream(file: File) -> impl Stream<Item = Result<Bytes, std::io::Error>> + 'static {
    stream::try_unfold(file, |mut file| async move {
        let mut buf = BytesMut::zeroed(CHUNK_SIZE);
        let n = file.read(&mut buf).await?;
        if n == 0 {
            return Ok(None);
        }
        buf.truncate(n);
        Ok(Some((buf.freeze(), file)))
    })
}

/// Uploads a local file into the storage.
///
/// An existing file with the same path is overwritten.
///
/// # Arguments
///
/// * `onsen_address` - The address of the storage.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `remote_file_path` - The path where the file is stored within the storage.
/// * `local_file_path` - The path of the local file, which is uploaded.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// * `Result<(), AinariError>` - Ok if the upload is successful, otherwise an AinariError.
pub async fn upload_file(
    onsen_address: &str,
    token: &str,
    internal_api_key: &Secret,
    remote_file_path: &str,
    local_file_path: &str,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let file = File::open(local_file_path).await.map_err(|e| {
        AinariError::internal(format!(
            "Failed to open local file '{local_file_path}' for upload: {e}"
        ))
    })?;
    let size = file
        .metadata()
        .await
        .map_err(|e| {
            AinariError::internal(format!(
                "Failed to read size of local file '{local_file_path}': {e}"
            ))
        })?
        .len();

    let client = prepare_client(onsen_address, insecure_client);
    let request = authorized(
        client.put(format!("{onsen_address}{FILE_ENDPOINT}")),
        token,
        internal_api_key,
        remote_file_path,
    )?;
    let response = request
        .timeout(TRANSFER_TIMEOUT)
        .content_type("application/octet-stream")
        .send_body(SizedStream::new(size, file_stream(file)))
        .await;

    handle_empty_response(response, "storage-file", remote_file_path).await
}

/// Downloads a file from the storage into a local file.
///
/// # Arguments
///
/// * `onsen_address` - The address of the storage.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `remote_file_path` - The path of the file within the storage.
/// * `local_file_path` - The path, where the downloaded file is stored locally.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// * `Result<(), AinariError>` - Ok if the download is successful, `NotFound`, if the file
///   doesn't exist, otherwise an AinariError.
pub async fn download_file(
    onsen_address: &str,
    token: &str,
    internal_api_key: &Secret,
    remote_file_path: &str,
    local_file_path: &str,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let client = prepare_client(onsen_address, insecure_client);
    let request = authorized(
        client.get(format!("{onsen_address}{FILE_ENDPOINT}")),
        token,
        internal_api_key,
        remote_file_path,
    )?;
    let mut response = successful_response(request.send().await, "download file").await?;

    let mut fh = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&local_file_path)
        .await
        .map_err(|e| {
            AinariError::internal(format!("Failed to open local dest file with error: {e}"))
        })?;

    // the timeout of the client only covers the time until the header arrived, the body is
    // streamed as long as it takes. It ends with an error, if the connection breaks off before
    // the announced content-length is reached.
    while let Some(chunk) = response.next().await {
        let chunk = chunk.map_err(|e| {
            AinariError::internal(format!(
                "Download of file '{remote_file_path}' broke off with error: {e}"
            ))
        })?;
        fh.write_all(&chunk).await.map_err(|e| {
            AinariError::internal(format!(
                "Failed to write downloaded chunk into dest file with error: {e}"
            ))
        })?;
    }

    fh.flush()
        .await
        .map_err(|e| AinariError::internal(format!("Failed to flush dest file with error: {e}")))?;
    log::debug!("Downloaded to {}", local_file_path);

    Ok(())
}

/// Deletes a file from the storage.
///
/// Deleting a not existing file is no error.
///
/// # Arguments
///
/// * `onsen_address` - The address of the storage.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `remote_file_path` - The path of the file within the storage.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// * `Result<(), AinariError>` - Ok if the deletion is successful, otherwise an AinariError.
pub async fn delete_file(
    onsen_address: &str,
    token: &str,
    internal_api_key: &Secret,
    remote_file_path: &str,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let client = prepare_client(onsen_address, insecure_client);
    let request = authorized(
        client.delete(format!("{onsen_address}{FILE_ENDPOINT}")),
        token,
        internal_api_key,
        remote_file_path,
    )?;

    handle_empty_response(request.send().await, "storage-file", remote_file_path).await
}

/// Uploads data from memory as file into the storage.
///
/// An existing file with the same path is overwritten.
///
/// # Arguments
///
/// * `onsen_address` - The address of the storage.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `remote_file_path` - The path where the file is stored within the storage.
/// * `data` - The content of the file.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// * `Result<(), AinariError>` - Ok if the upload is successful, otherwise an AinariError.
pub async fn upload_data(
    onsen_address: &str,
    token: &str,
    internal_api_key: &Secret,
    remote_file_path: &str,
    data: Vec<u8>,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let client = prepare_client(onsen_address, insecure_client);
    let request = authorized(
        client.put(format!("{onsen_address}{FILE_ENDPOINT}")),
        token,
        internal_api_key,
        remote_file_path,
    )?;
    let response = request
        .timeout(TRANSFER_TIMEOUT)
        .content_type("application/octet-stream")
        .send_body(data)
        .await;

    handle_empty_response(response, "storage-file", remote_file_path).await
}

/// Downloads a file from the storage into memory.
///
/// # Arguments
///
/// * `onsen_address` - The address of the storage.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `remote_file_path` - The path of the file within the storage.
/// * `max_size` - Maximum size of the file in bytes, so a large file can't fill the memory.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// * `Result<Vec<u8>, AinariError>` - The content of the file, `NotFound`, if the file doesn't
///   exist, `InvalidInput`, if the file is larger than `max_size`, otherwise an AinariError.
pub async fn download_data(
    onsen_address: &str,
    token: &str,
    internal_api_key: &Secret,
    remote_file_path: &str,
    max_size: usize,
    insecure_client: bool,
) -> Result<Vec<u8>, AinariError> {
    let client = prepare_client(onsen_address, insecure_client);
    let request = authorized(
        client.get(format!("{onsen_address}{FILE_ENDPOINT}")),
        token,
        internal_api_key,
        remote_file_path,
    )?;
    let mut response = successful_response(request.send().await, "download file").await?;

    let too_large = || {
        AinariError::InvalidInput(format!(
            "File '{remote_file_path}' is larger than {max_size} bytes"
        ))
    };
    if content_length(&response, remote_file_path)? > max_size as u64 {
        return Err(too_large());
    }

    let mut data = Vec::new();
    while let Some(chunk) = response.next().await {
        let chunk = chunk.map_err(|e| {
            AinariError::internal(format!(
                "Download of file '{remote_file_path}' broke off with error: {e}"
            ))
        })?;
        if data.len() + chunk.len() > max_size {
            return Err(too_large());
        }
        data.extend_from_slice(&chunk);
    }

    Ok(data)
}

/// Copies a file from one storage to another, without storing it locally.
///
/// The download of the source is passed chunk by chunk into the upload to the target, so also
/// large files are never held in memory. The upload announces the size of the source, so the
/// target doesn't keep the file, if the download breaks off. An incomplete target-file is
/// deleted here again anyway, in case the target could store a part of it. An existing
/// target-file with the same path is overwritten.
///
/// # Arguments
///
/// * `source_address` - The address of the storage, which stores the file.
/// * `source_path` - The path of the file within the source.
/// * `target_address` - The address of the storage, where the file is copied to.
/// * `target_path` - The path of the copy within the target.
/// * `token` - Authentication token for the API on both storages.
/// * `internal_api_key` - Internal API key for authorization on both storages.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// * `Result<u64, AinariError>` - The number of copied bytes, `NotFound`, if the source-file
///   doesn't exist, otherwise an AinariError.
pub async fn copy_file(
    source_address: &str,
    source_path: &str,
    target_address: &str,
    target_path: &str,
    token: &str,
    internal_api_key: &Secret,
    insecure_client: bool,
) -> Result<u64, AinariError> {
    let source_client = prepare_client(source_address, insecure_client);
    let request = authorized(
        source_client.get(format!("{source_address}{FILE_ENDPOINT}")),
        token,
        internal_api_key,
        source_path,
    )?;
    let source = successful_response(request.send().await, "download file").await?;
    let size = content_length(&source, source_path)?;

    let target_client = prepare_client(target_address, insecure_client);
    let request = authorized(
        target_client.put(format!("{target_address}{FILE_ENDPOINT}")),
        token,
        internal_api_key,
        target_path,
    )?;
    let response = request
        .timeout(TRANSFER_TIMEOUT)
        .content_type("application/octet-stream")
        .send_body(SizedStream::new(size, source))
        .await;
    let result = handle_empty_response(response, "storage-file", target_path)
        .await
        .map(|_| size);

    if result.is_err() {
        // the storage accepts the deletion of a file, which doesn't exist
        if let Err(e) = delete_file(
            target_address,
            token,
            internal_api_key,
            target_path,
            insecure_client,
        )
        .await
        {
            log::error!(
                "Failed to delete incomplete copy '{target_path}' on storage '{target_address}': {e}"
            );
        }
    }

    result
}

/// Lists the direct entries of a directory within the storage.
///
/// # Arguments
///
/// * `onsen_address` - The address of the storage.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `remote_dir_path` - The path of the directory within the storage.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// * `Result<Vec<StorageDirectoryEntry>, AinariError>` - The entries sorted by name, `NotFound`,
///   if the directory doesn't exist, otherwise an AinariError.
pub async fn list_directory(
    onsen_address: &str,
    token: &str,
    internal_api_key: &Secret,
    remote_dir_path: &str,
    insecure_client: bool,
) -> Result<Vec<StorageDirectoryEntry>, AinariError> {
    let client = prepare_client(onsen_address, insecure_client);
    let request = authorized(
        client.get(format!("{onsen_address}{DIRECTORY_ENDPOINT}")),
        token,
        internal_api_key,
        remote_dir_path,
    )?;

    let resp: StorageDirectoryResp =
        handle_response(request.send().await, "storage-directory", remote_dir_path).await?;
    Ok(resp.entries)
}

/// Deletes a directory with all of its content from the storage.
///
/// Deleting a not existing directory is no error.
///
/// # Arguments
///
/// * `onsen_address` - The address of the storage.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `remote_dir_path` - The path of the directory within the storage.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// * `Result<(), AinariError>` - Ok if the deletion is successful, otherwise an AinariError.
pub async fn delete_directory(
    onsen_address: &str,
    token: &str,
    internal_api_key: &Secret,
    remote_dir_path: &str,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let client = prepare_client(onsen_address, insecure_client);
    let request = authorized(
        client.delete(format!("{onsen_address}{DIRECTORY_ENDPOINT}")),
        token,
        internal_api_key,
        remote_dir_path,
    )?;

    handle_empty_response(request.send().await, "storage-directory", remote_dir_path).await
}

/// Requests size and sha256-checksum of a file within the storage.
///
/// # Arguments
///
/// * `onsen_address` - The address of the storage.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `remote_file_path` - The path of the file within the storage.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// * `Result<StorageFileInfoResp, AinariError>` - Size and hex-encoded sha256 of the file,
///   `NotFound`, if the file doesn't exist, otherwise an AinariError.
pub async fn get_file_info(
    onsen_address: &str,
    token: &str,
    internal_api_key: &Secret,
    remote_file_path: &str,
    insecure_client: bool,
) -> Result<StorageFileInfoResp, AinariError> {
    let client = prepare_client(onsen_address, insecure_client);
    let request = authorized(
        client.get(format!("{onsen_address}{FILE_INFO_ENDPOINT}")),
        token,
        internal_api_key,
        remote_file_path,
    )?;

    // the storage reads the whole file for its checksum, which takes its time for large disks
    let response = request.timeout(TRANSFER_TIMEOUT).send().await;
    handle_response(response, "storage-file-info", remote_file_path).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_to_error() {
        assert!(matches!(
            status_to_error("test", StatusCode::NOT_FOUND, b"missing"),
            AinariError::NotFound(_)
        ));
        assert!(matches!(
            status_to_error("test", StatusCode::BAD_REQUEST, b"invalid"),
            AinariError::InvalidInput(_)
        ));
        assert!(matches!(
            status_to_error("test", StatusCode::INTERNAL_SERVER_ERROR, b"broken"),
            AinariError::InternalError(_)
        ));
    }
}
