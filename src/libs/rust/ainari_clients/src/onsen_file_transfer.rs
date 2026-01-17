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
use data::DeleteRequest;
use data::DownloadRequest;
use data::data_service_client::DataServiceClient;
use tonic::Request;
// use std::fs;

use async_stream::stream;
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

    // Connect to the remote server
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
            AinariError::InternalError(format!(
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
            AinariError::InternalError(format!(
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
            AinariError::InternalError(format!("Failed to open local dest file with error: {e}"))
        })?;

    // Process each chunk received from the server
    while let Some(chunk_res) = stream.message().await.transpose() {
        let chunk = chunk_res.map_err(|e| {
            AinariError::InternalError(format!(
                "Failed to receive downloaded chunk with error: {e}"
            ))
        })?;
        if !chunk.chunk.is_empty() {
            // Write the chunk to the local file
            fh.write_all(&chunk.chunk).await.map_err(|e| {
                AinariError::InternalError(format!(
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
    fh.flush().await.map_err(|e| {
        AinariError::InternalError(format!("Failed to flush dest file with error: {e}"))
    })?;
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
    // Connect to the remote server
    let mut client = DataServiceClient::connect(onsen_address.to_owned()).await?;

    // Create a request for the file to be deleted
    let req = DeleteRequest {
        remote_file_path: remote_file_path.to_owned(),
    };

    // Send the deletion request to the server
    let response = client.delete_file(Request::new(req)).await?;
    println!("Server response: {:?}", response.into_inner().status);

    Ok(())
}
