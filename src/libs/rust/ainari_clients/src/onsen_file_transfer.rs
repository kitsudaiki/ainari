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

pub mod data {
    tonic::include_proto!("data");
}

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

    let outbound = stream! {
        let mut f = File::open(&_local_file_path).await.expect("failed to open input file");
        let mut buf = vec![0u8; CHUNK_SIZE];

        let mut counter: i64 = 0;
        loop {
            let n = f.read(&mut buf).await.expect("read error");
            if n == 0 {
                yield DataChunk {
                    remote_file_path: _remote_file_path.clone(),
                    chunk: vec![],
                    chunk_number: counter,
                    eof: true,
                };
                break;
            }

            yield DataChunk {
                remote_file_path: _remote_file_path.clone(),
                chunk: buf[..n].to_vec(),
                chunk_number: counter,
                eof: false,
            };

            counter += 1;
        }
    };

    let response = client.upload_file(Request::new(outbound)).await?;
    println!("Server response: {:?}", response.into_inner().status);

    Ok(())
}

pub async fn download_file(
    onsen_address: &str,
    remote_file_path: &str,
    local_file_path: &str,
) -> Result<(), AinariError> {
    let mut client = DataServiceClient::connect(onsen_address.to_owned())
        .await
        .map_err(|e| {
            AinariError::InternalError(format!(
                "Failed to open grpc-connection to onsen with error: {e}"
            ))
        })?;

    let req = DownloadRequest {
        remote_file_path: remote_file_path.to_owned(),
    };

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

    while let Some(chunk_res) = stream.message().await.transpose() {
        let chunk = chunk_res.map_err(|e| {
            AinariError::InternalError(format!(
                "Failed to receive downloaded chunk with error: {e}"
            ))
        })?;
        if !chunk.chunk.is_empty() {
            fh.write_all(&chunk.chunk).await.map_err(|e| {
                AinariError::InternalError(format!(
                    "Failed to write downloaded chunk into dest file with error: {e}"
                ))
            })?;
        }
        if chunk.eof {
            break;
        }
    }

    fh.flush().await.map_err(|e| {
        AinariError::InternalError(format!("Failed to flush dest file with error: {e}"))
    })?;
    log::debug!("Downloaded to {}", local_file_path);

    Ok(())
}

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
