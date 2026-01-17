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

use data::data_service_server::{DataService, DataServiceServer};
use data::{
    DataChunk, DataResponse, DatasetDimensionRequest, DatasetDimensionResponse, DeleteRequest,
    DownloadRequest,
};
use tonic::{Code, Request, Response, Status, transport::Server};

use async_stream::try_stream;
use futures_core::Stream;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

use crate::config;

use ainari_common::constants::CHUNK_SIZE;
use ainari_common::functions::is_safe_subpath;
use ainari_dataset::dataset_io::read_data_set_file;

pub mod data {
    tonic::include_proto!("data");
}

/// Server implementation for the DataService gRPC service.
#[derive(Default)]
pub struct OnsenServer;

/// Implementation of the DataService trait for the OnsenServer.
#[tonic::async_trait]
impl DataService for OnsenServer {
    /// Type alias for the stream used in file downloads.
    type DownloadFileStream =
        Pin<Box<dyn Stream<Item = Result<DataChunk, Status>> + Send + 'static>>;

    /// Handles file upload requests.
    ///
    /// This method receives a stream of data chunks and writes them to a file.
    /// The file path is constructed from the provided remote_file_path and the
    /// configured storage location.
    ///
    /// # Arguments
    ///
    /// * `request` - A tonic Request containing a streaming DataChunk.
    ///
    /// # Returns
    ///
    /// * `Result<Response<DataResponse>, Status>` - The response containing the status message
    ///   or an error status.
    async fn upload_file(
        &self,
        request: Request<tonic::Streaming<DataChunk>>,
    ) -> Result<Response<DataResponse>, Status> {
        let mut stream = request.into_inner();
        let mut file: Option<tokio::fs::File> = None;
        let mut target_str = "".to_string();

        while let Some(chunk_res) = stream.message().await.transpose() {
            let chunk = match chunk_res {
                Ok(c) => c,
                Err(e) => {
                    let msg = format!("stream error: {}", e);
                    log::error!("{}", msg);
                    return Err(Status::new(Code::Unknown, msg))
                }
            };

            // open file on first real chunk (or immediately after we collected metadata)
            if file.is_none() {
                let mut target_path = PathBuf::from(&config::CONFIG.storage.location);
                let remote_file_path = Path::new(&chunk.remote_file_path);
                if !is_safe_subpath(remote_file_path) {
                    let msg = format!("provided remote-path is invalid: {:?}", remote_file_path);
                    log::error!("{}", msg);
                    return Err(Status::internal(msg));
                }
                target_path.push(remote_file_path);

                // create directory of target-file, if not already exist
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)
                        .await
                        .map_err(|e| {
                            let msg = format!("mkdir error: {}", e);
                            log::error!("{}", msg);
                            Status::internal(msg)
                        })?;
                }

                let f = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(&target_path)
                    .await
                    .map_err(|e| {
                        let msg = format!("failed to open file {:?}: {}", target_path, e);
                        log::error!("{}", msg);
                        Status::internal(msg)
                    })?;

                target_str = format!("{:?}", target_path);
                println!("Receiving file: {target_str}");
                file = Some(f);
            }

            // write chunk bytes
            if let Some(fh) = file.as_mut() {
                if !chunk.chunk.is_empty() {
                    fh.write_all(&chunk.chunk)
                        .await
                        .map_err(|e| {
                            let msg = format!("write error: {}", e);
                            log::error!("{}", msg);
                            Status::internal(msg)
                        })?;
                }
            }

            if chunk.eof {
                break;
            }
        }

        if let Some(mut fh) = file {
            fh.flush()
                .await
                .map_err(|e| {
                    let msg = format!("flush error: {}", e);
                    log::error!("{}", msg);
                    Status::internal(msg)
                })?;
        }

        println!("File received successfully: {target_str}");

        Ok(Response::new(DataResponse {
            status: format!("Stored at: {target_str}"),
        }))
    }

    /// Handles file download requests.
    ///
    /// This method reads the file in chunks and streams them to the client.
    /// The file path is constructed from the provided remote_file_path and the
    /// configured storage location.
    ///
    /// # Arguments
    ///
    /// * `request` - A tonic Request containing a DownloadRequest.
    ///
    /// # Returns
    ///
    /// * `Result<Response<Self::DownloadFileStream>, Status>` - The response containing the stream
    ///   of data chunks or an error status.
    async fn download_file(
        &self,
        request: Request<DownloadRequest>,
    ) -> Result<Response<Self::DownloadFileStream>, Status> {
        let req = request.into_inner();

        // Build path under STORAGE_ROOT and sanitize subpath
        let mut target_path = PathBuf::from(&config::CONFIG.storage.location);
        let remote_file_path = Path::new(&req.remote_file_path);
        if !is_safe_subpath(remote_file_path) {
            return Err(Status::internal(format!(
                "provided remote-path is invalid: {:?}",
                remote_file_path
            )));
        }
        target_path.push(remote_file_path);

        let path_clone = target_path.clone();
        // read data-stream
        let s = try_stream! {
            let mut f = File::open(&path_clone)
                .await
                .map_err(|_| Status::not_found(format!("file not found: {:?}", req.remote_file_path)))?;

            let mut buf = vec![0u8; CHUNK_SIZE];

            let mut counter: i64 = 0;
            loop {
                let n = f.read(&mut buf).await
                    .map_err(|e| Status::internal(format!("read error: {}", e)))?;
                if n == 0 {
                    // final EOF message (optional)
                    yield DataChunk {
                        remote_file_path: "".to_string(),
                        chunk: vec![],
                        chunk_number: counter,
                        eof: true,
                    };
                    break;
                }
                yield DataChunk {
                    remote_file_path: "".to_string(),
                    chunk: buf[..n].to_vec(),
                    chunk_number: counter,
                    eof: false,
                };

                counter += 1;
            }
        };

        let boxed: Self::DownloadFileStream = Box::pin(s);
        Ok(Response::new(boxed))
    }

    /// Handles file deletion requests.
    ///
    /// This method deletes the file at the specified path.
    /// The file path is constructed from the provided remote_file_path and the
    /// configured storage location.
    ///
    /// # Arguments
    ///
    /// * `request` - A tonic Request containing a DeleteRequest.
    ///
    /// # Returns
    ///
    /// * `Result<Response<DataResponse>, Status>` - The response containing the status message
    ///   or an error status.
    async fn delete_file(
        &self,
        request: Request<DeleteRequest>,
    ) -> Result<Response<DataResponse>, Status> {
        let req = request.into_inner();

        // Build path under STORAGE_ROOT and sanitize subpath
        let mut target_path = PathBuf::from(&config::CONFIG.storage.location);
        let remote_file_path = Path::new(&req.remote_file_path);
        if !is_safe_subpath(remote_file_path) {
            let msg = format!("provided remote-path is invalid: {:?}", remote_file_path);
            log::error!("{}", msg);
            return Err(Status::internal(msg));
        }
        target_path.push(remote_file_path);

        match tokio::fs::remove_file(&target_path).await {
            Ok(_) => {}
            Err(e) => {
                log::error!(
                    "Failed to delete file '{:?}' with error: {}",
                    target_path,
                    e
                );
            }
        }

        Ok(Response::new(DataResponse {
            status: format!("Deleted file: {:?}", target_path),
        }))
    }

    /// Handles requests for dataset dimensions.
    ///
    /// This method reads the dataset file and returns the number of rows and columns.
    /// The file path is constructed from the provided remote_file_path and the
    /// configured storage location.
    ///
    /// # Arguments
    ///
    /// * `request` - A tonic Request containing a DatasetDimensionRequest.
    ///
    /// # Returns
    ///
    /// * `Result<Response<DatasetDimensionResponse>, Status>` - The response containing the dataset
    ///   dimensions or an error status.
    async fn get_dataset_dimension(
        &self,
        request: Request<DatasetDimensionRequest>,
    ) -> Result<Response<DatasetDimensionResponse>, Status> {
        let req = request.into_inner();

        let target_path = format!(
            "{}/{}",
            config::CONFIG.storage.location,
            req.remote_file_path
        );
        let file_handle = match read_data_set_file(&target_path) {
            Ok(file_handle) => file_handle,
            Err(_) => {
                return Err(Status::internal(format!(
                    "provided remote-path is invalid: {:?}",
                    req.remote_file_path
                )));
            }
        };

        let number_of_rows = file_handle.get_number_of_rows();
        let number_of_columns = file_handle.header.columns.len() as u64;

        Ok(Response::new(DatasetDimensionResponse {
            status: format!("Deleted file: {:?}", target_path),
            number_of_rows: number_of_rows as i64,
            number_of_columns: number_of_columns as i64,
        }))
    }
}

/// Starts the gRPC server.
///
/// This function creates the necessary directories, sets up the server address,
/// and starts the server with the OnsenServer implementation.
///
/// # Returns
///
/// * `Result<(), Box<dyn std::error::Error>>` - The result of the server operation.
pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    tokio::fs::create_dir_all(&config::CONFIG.storage.location).await?;

    let addr = "0.0.0.0:50051".parse()?;
    let svc = OnsenServer;

    println!("Server listening on {}", addr);
    Server::builder()
        .add_service(DataServiceServer::new(svc))
        .serve(addr)
        .await?;

    Ok(())
}
