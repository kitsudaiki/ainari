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

use ainari_common::error::AinariError;
use data::DataChunk;
use data::DeleteDirectoryRequest;
use data::DeleteRequest;
use data::DirectoryEntry;
use data::DownloadRequest;
use data::FileInfoRequest;
use data::FileInfoResponse;
use data::ListDirectoryRequest;
use data::data_service_client::DataServiceClient;
use tonic::transport::Channel;
use tonic::{Code, Request, Status};
// use std::fs;

use async_stream::stream;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::fs::File;
use tokio::fs::OpenOptions;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

use ainari_common::constants::CHUNK_SIZE;

/// Module containing the data service protocol definitions.
pub mod data {
    tonic::include_proto!("data");
}

/// Uploads a file to a remote server using gRPC streaming.
///
/// # Arguments
///
/// * `onsen_address` - The address of the remote server.
/// * `remote_file_path` - The path where the file will be stored on the remote server.
/// * `local_file_path` - The path to the local file to be uploaded.
///
/// # Returns
///
/// * `Result<(), Box<dyn std::error::Error>>` - Ok if the upload is successful, otherwise an error.
pub async fn upload_file(
    onsen_address: &str,
    remote_file_path: &str,
    local_file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: check if local_file_path exist

    // HINT(kitsudaiki): Must be cloned here to avoid an error
    // "escapes the function body hereargument requires that `'1` must outlive `'static`"
    // below at the "upload_file"-function
    let _remote_file_path = remote_file_path.to_owned();
    let _local_file_path = local_file_path.to_owned();

    let mut client = DataServiceClient::connect(onsen_address.to_owned()).await?;

    // Create a streaming iterator for reading the file in chunks
    let outbound = stream! {
        // Open the local file for reading
        let mut f = File::open(&_local_file_path).await.expect("failed to open input file");
        // Create a buffer for reading chunks of the file
        let mut buf = vec![0u8; CHUNK_SIZE];

        // Counter for tracking the current chunk number
        let mut counter: i64 = 0;
        loop {
            // Read a chunk of data from the file
            let n = f.read(&mut buf).await.expect("read error");
            if n == 0 {
                // End of file reached
                yield DataChunk {
                    remote_file_path: _remote_file_path.clone(),
                    chunk: vec![],
                    chunk_number: counter,
                    eof: true,
                };
                break;
            }

            // Yield the current chunk of data
            yield DataChunk {
                remote_file_path: _remote_file_path.clone(),
                chunk: buf[..n].to_vec(),
                chunk_number: counter,
                eof: false,
            };

            // Increment the chunk counter
            counter += 1;
        }
    };

    // Send the file chunks to the remote server
    let response = client.upload_file(Request::new(outbound)).await?;
    println!("Server response: {:?}", response.into_inner().status);

    Ok(())
}

/// Downloads a file from a remote server using gRPC streaming.
///
/// # Arguments
///
/// * `onsen_address` - The address of the remote server.
/// * `remote_file_path` - The path to the file on the remote server.
/// * `local_file_path` - The path where the downloaded file will be stored locally.
///
/// # Returns
///
/// * `Result<(), AinariError>` - Ok if the download is successful, otherwise an AinariError.
pub async fn download_file(
    onsen_address: &str,
    remote_file_path: &str,
    local_file_path: &str,
) -> Result<(), AinariError> {
    // Connect to the remote server
    let mut client = DataServiceClient::connect(onsen_address.to_owned())
        .await
        .map_err(|e| {
            AinariError::internal(format!(
                "Failed to open grpc-connection to onsen with error: {e}"
            ))
        })?;

    // Create a request for the file to be downloaded
    let req = DownloadRequest {
        remote_file_path: remote_file_path.to_owned(),
    };

    // Start the file download stream
    let mut stream = client
        .download_file(Request::new(req))
        .await
        .map_err(|e| {
            AinariError::internal(format!(
                "Failed start file-download over grpc with error: {e}"
            ))
        })?
        .into_inner();
    //fs::create_dir_all(&local_file_path)?;

    // Open local destination file for writing
    let mut fh = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&local_file_path)
        .await
        .map_err(|e| {
            AinariError::internal(format!("Failed to open local dest file with error: {e}"))
        })?;

    // Process each chunk received from the server
    while let Some(chunk_res) = stream.message().await.transpose() {
        let chunk = chunk_res.map_err(|e| {
            AinariError::internal(format!(
                "Failed to receive downloaded chunk with error: {e}"
            ))
        })?;
        if !chunk.chunk.is_empty() {
            // Write the chunk to the local file
            fh.write_all(&chunk.chunk).await.map_err(|e| {
                AinariError::internal(format!(
                    "Failed to write downloaded chunk into dest file with error: {e}"
                ))
            })?;
        }
        if chunk.eof {
            // End of file reached
            break;
        }
    }

    // Ensure all data is written to the file
    fh.flush()
        .await
        .map_err(|e| AinariError::internal(format!("Failed to flush dest file with error: {e}")))?;
    log::debug!("Downloaded to {}", local_file_path);

    Ok(())
}

/// Deletes a file from a remote server using gRPC.
///
/// # Arguments
///
/// * `onsen_address` - The address of the remote server.
/// * `remote_file_path` - The path to the file on the remote server to be deleted.
///
/// # Returns
///
/// * `Result<(), Box<dyn std::error::Error>>` - Ok if the deletion is successful, otherwise an error.
pub async fn delete_file(
    onsen_address: &str,
    remote_file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = DataServiceClient::connect(onsen_address.to_owned()).await?;

    let req = DeleteRequest {
        remote_file_path: remote_file_path.to_owned(),
    };

    let response = client.delete_file(Request::new(req)).await?;
    println!("Server response: {:?}", response.into_inner().status);

    Ok(())
}

/// Opens a grpc-connection to an onsen.
async fn connect(onsen_address: &str) -> Result<DataServiceClient<Channel>, AinariError> {
    DataServiceClient::connect(onsen_address.to_owned())
        .await
        .map_err(|e| {
            AinariError::internal(format!(
                "Failed to open grpc-connection to onsen with error: {e}"
            ))
        })
}

/// Converts the status of a failed grpc-request into an error, so `NOT_FOUND` and
/// `INVALID_ARGUMENT` of the onsen can be handled by the caller.
fn status_to_error(action: &str, status: Status) -> AinariError {
    let msg = format!("Failed to {action} on onsen: {}", status.message());
    match status.code() {
        Code::NotFound => AinariError::NotFound(msg),
        Code::InvalidArgument => AinariError::InvalidInput(msg),
        _ => AinariError::internal(msg),
    }
}

/// Uploads data from memory as file to a remote server using gRPC streaming.
///
/// An existing file with the same path is overwritten.
///
/// # Arguments
///
/// * `onsen_address` - The address of the remote server.
/// * `remote_file_path` - The path where the file will be stored on the remote server.
/// * `data` - The content of the file.
///
/// # Returns
///
/// * `Result<(), AinariError>` - Ok if the upload is successful, otherwise an AinariError.
pub async fn upload_data(
    onsen_address: &str,
    remote_file_path: &str,
    data: Vec<u8>,
) -> Result<(), AinariError> {
    let remote_file_path = remote_file_path.to_owned();
    let outbound = stream! {
        let mut counter: i64 = 0;
        for chunk in data.chunks(CHUNK_SIZE) {
            yield DataChunk {
                remote_file_path: remote_file_path.clone(),
                chunk: chunk.to_vec(),
                chunk_number: counter,
                eof: false,
            };
            counter += 1;
        }
        yield DataChunk {
            remote_file_path: remote_file_path.clone(),
            chunk: vec![],
            chunk_number: counter,
            eof: true,
        };
    };

    connect(onsen_address)
        .await?
        .upload_file(Request::new(outbound))
        .await
        .map_err(|e| status_to_error("upload file", e))?;

    Ok(())
}

/// Downloads a file from a remote server into memory using gRPC streaming.
///
/// # Arguments
///
/// * `onsen_address` - The address of the remote server.
/// * `remote_file_path` - The path to the file on the remote server.
/// * `max_size` - Maximum size of the file in bytes, so a large file can't fill the memory.
///
/// # Returns
///
/// * `Result<Vec<u8>, AinariError>` - The content of the file, `NotFound`, if the file doesn't
///   exist, `InvalidInput`, if the file is larger than `max_size`, otherwise an AinariError.
pub async fn download_data(
    onsen_address: &str,
    remote_file_path: &str,
    max_size: usize,
) -> Result<Vec<u8>, AinariError> {
    let req = DownloadRequest {
        remote_file_path: remote_file_path.to_owned(),
    };
    let mut stream = connect(onsen_address)
        .await?
        .download_file(Request::new(req))
        .await
        .map_err(|e| status_to_error("download file", e))?
        .into_inner();

    let mut data = Vec::new();
    loop {
        let chunk = stream
            .message()
            .await
            .map_err(|e| status_to_error("download file", e))?
            .ok_or_else(|| {
                AinariError::internal(format!(
                    "Download of file '{remote_file_path}' ended before the end of the file"
                ))
            })?;
        if data.len() + chunk.chunk.len() > max_size {
            return Err(AinariError::InvalidInput(format!(
                "File '{remote_file_path}' is larger than {max_size} bytes"
            )));
        }
        data.extend_from_slice(&chunk.chunk);
        if chunk.eof {
            return Ok(data);
        }
    }
}

/// Copies a file from one remote server to another using gRPC streaming, without storing it
/// locally.
///
/// The download-stream of the source is passed chunk by chunk into the upload-stream of the
/// target, so also large files are never held in memory. The target only reports, that the
/// upload-stream has ended, not if the file is complete, so the end of the source is checked
/// here: if the download fails or ends before the end of the file, the incomplete target-file is
/// deleted again. An existing target-file with the same path is overwritten.
///
/// # Arguments
///
/// * `source_address` - The address of the remote server, which stores the file.
/// * `source_path` - The path of the file on the source.
/// * `target_address` - The address of the remote server, where the file is copied to.
/// * `target_path` - The path of the copy on the target.
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
) -> Result<u64, AinariError> {
    let mut target = connect(target_address).await?;
    let req = DownloadRequest {
        remote_file_path: source_path.to_owned(),
    };
    let mut source = connect(source_address)
        .await?
        .download_file(Request::new(req))
        .await
        .map_err(|e| status_to_error("download file", e))?
        .into_inner();

    // the stream can't return an error, so the result of the download is shared with it
    let download_error: Arc<Mutex<Option<AinariError>>> = Arc::new(Mutex::new(None));
    let is_complete = Arc::new(AtomicBool::new(false));
    let copied_bytes = Arc::new(AtomicU64::new(0));

    let outbound = {
        let download_error = download_error.clone();
        let is_complete = is_complete.clone();
        let copied_bytes = copied_bytes.clone();
        let target_path = target_path.to_owned();
        stream! {
            let mut counter: i64 = 0;
            loop {
                match source.message().await {
                    Ok(Some(chunk)) => {
                        copied_bytes.fetch_add(chunk.chunk.len() as u64, Ordering::Relaxed);
                        let eof = chunk.eof;
                        yield DataChunk {
                            remote_file_path: target_path.clone(),
                            chunk: chunk.chunk,
                            chunk_number: counter,
                            eof,
                        };
                        counter += 1;
                        if eof {
                            is_complete.store(true, Ordering::Relaxed);
                            break;
                        }
                    }
                    // the source ended the stream before the end of the file
                    Ok(None) => break,
                    Err(e) => {
                        let error = status_to_error("download file", e);
                        *download_error.lock().expect("mutex poisoned") = Some(error);
                        break;
                    }
                }
            }
        }
    };

    let upload_result = target
        .upload_file(Request::new(outbound))
        .await
        .map_err(|e| status_to_error("upload file", e));

    let download_error = download_error.lock().expect("mutex poisoned").take();
    let result = match (upload_result, download_error) {
        (_, Some(e)) => Err(e),
        (Err(e), None) => Err(e),
        (Ok(_), None) if !is_complete.load(Ordering::Relaxed) => Err(AinariError::internal(
            format!("Download of file '{source_path}' ended before the end of the file"),
        )),
        (Ok(_), None) => Ok(copied_bytes.load(Ordering::Relaxed)),
    };

    if result.is_err() {
        // the onsen accepts the deletion of a file, which doesn't exist
        let req = DeleteRequest {
            remote_file_path: target_path.to_owned(),
        };
        if let Err(e) = target.delete_file(Request::new(req)).await {
            log::error!(
                "Failed to delete incomplete copy '{target_path}' on onsen '{target_address}': {e}"
            );
        }
    }

    result
}

/// Lists the direct entries of a directory on a remote server using gRPC.
///
/// # Arguments
///
/// * `onsen_address` - The address of the remote server.
/// * `remote_dir_path` - The path of the directory on the remote server.
///
/// # Returns
///
/// * `Result<Vec<DirectoryEntry>, AinariError>` - The entries sorted by name, `NotFound`, if the
///   directory doesn't exist, otherwise an AinariError.
pub async fn list_directory(
    onsen_address: &str,
    remote_dir_path: &str,
) -> Result<Vec<DirectoryEntry>, AinariError> {
    let req = ListDirectoryRequest {
        remote_dir_path: remote_dir_path.to_owned(),
    };
    let response = connect(onsen_address)
        .await?
        .list_directory(Request::new(req))
        .await
        .map_err(|e| status_to_error("list directory", e))?;

    Ok(response.into_inner().entries)
}

/// Deletes a directory with all of its content from a remote server using gRPC.
///
/// Deleting a not existing directory is no error.
///
/// # Arguments
///
/// * `onsen_address` - The address of the remote server.
/// * `remote_dir_path` - The path of the directory on the remote server to be deleted.
///
/// # Returns
///
/// * `Result<(), AinariError>` - Ok if the deletion is successful, otherwise an AinariError.
pub async fn delete_directory(
    onsen_address: &str,
    remote_dir_path: &str,
) -> Result<(), AinariError> {
    let req = DeleteDirectoryRequest {
        remote_dir_path: remote_dir_path.to_owned(),
    };
    connect(onsen_address)
        .await?
        .delete_directory(Request::new(req))
        .await
        .map_err(|e| status_to_error("delete directory", e))?;

    Ok(())
}

/// Requests size and sha256-checksum of a file on a remote server using gRPC.
///
/// # Arguments
///
/// * `onsen_address` - The address of the remote server.
/// * `remote_file_path` - The path of the file on the remote server.
///
/// # Returns
///
/// * `Result<FileInfoResponse, AinariError>` - Size and hex-encoded sha256 of the file,
///   `NotFound`, if the file doesn't exist, otherwise an AinariError.
pub async fn get_file_info(
    onsen_address: &str,
    remote_file_path: &str,
) -> Result<FileInfoResponse, AinariError> {
    let req = FileInfoRequest {
        remote_file_path: remote_file_path.to_owned(),
    };
    let response = connect(onsen_address)
        .await?
        .get_file_info(Request::new(req))
        .await
        .map_err(|e| status_to_error("get file-info", e))?;

    Ok(response.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_to_error() {
        assert!(matches!(
            status_to_error("test", Status::not_found("missing")),
            AinariError::NotFound(_)
        ));
        assert!(matches!(
            status_to_error("test", Status::invalid_argument("invalid")),
            AinariError::InvalidInput(_)
        ));
        assert!(matches!(
            status_to_error("test", Status::internal("broken")),
            AinariError::InternalError(_)
        ));
    }
}
