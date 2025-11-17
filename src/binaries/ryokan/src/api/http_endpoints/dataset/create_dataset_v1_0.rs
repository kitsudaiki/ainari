// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use actix_multipart::Multipart;
use actix_web::http::header::ContentDisposition;
use actix_web::web::Path;
use apistos::actix::CreatedJson;
use apistos::api_operation;
use futures_util::StreamExt;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::config;
use crate::core::converter::{load_csv_file, load_mnist_images};
use crate::onsen_functions::select_onsen;

use ainari_api::common_functions::{create_directory, upload_file_to_onsen};
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::dataset_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::error::AinariError;

#[api_operation(
    tag = "dataset",
    summary = "Create new dataset",
    description = r###"Create new dataset by uploading files."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn upload_binary(
    payload: Multipart,
    path: Path<(String, String)>,
    context: UserContext,
) -> Result<CreatedJson<DatasetResp>, ErrorResponse> {
    let (dataset_type, name) = path.into_inner();
    let dataset_uuid = Uuid::new_v4();
    let tempfile_dir = config::CONFIG.storage.tempfile_location.clone();
    let target_dir_path = format!("{tempfile_dir}/{}", dataset_uuid);
    let converted_result_path = format!("{target_dir_path}/converted_result");
    let upload_file_path_str: String = format!("datasets/{dataset_uuid}");

    super::check_dataset_type(&dataset_type)?;

    create_directory(&target_dir_path).await?;

    super::check_dataset_quota(&context).await?;

    let selected_onsen = select_onsen(&context)?;

    let temp_file_paths = write_payload_into_file(payload, &target_dir_path).await?;

    convert_uploaded_files(
        &dataset_uuid,
        &name,
        &dataset_type,
        &converted_result_path,
        &temp_file_paths,
    )
    .await?;

    upload_file_to_onsen(
        &selected_onsen.address,
        &upload_file_path_str,
        &converted_result_path,
    )
    .await?;

    // delete all temporary files
    match std::fs::remove_dir_all(&target_dir_path) {
        Ok(()) => {}
        Err(e) => {
            log::error!("Failed to delete temp-dir {target_dir_path} from disk with error {e}.");
        }
    }

    super::add_dataset_to_database(
        &dataset_uuid,
        &name,
        &selected_onsen.address,
        &upload_file_path_str,
        &context,
    )?;

    let resp = super::get_dataset(&dataset_uuid, &context).await?;

    return Ok(CreatedJson(resp));
}

async fn write_payload_into_file(
    mut payload: Multipart,
    target_dir_path: &String,
) -> Result<Vec<PathBuf>, ErrorResponse> {
    let mut temp_file_paths = Vec::new();
    // process items from payload
    while let Some(item) = payload.next().await {
        let mut field = match item {
            Ok(value) => value,
            Err(_) => {
                return Err(ErrorResponse::BadRequest(
                    "Failed to read next item from input.".to_string(),
                ));
            }
        };

        // get file-name of item
        let content_disposition = field.content_disposition();
        let filename = match content_disposition {
            Some(ContentDisposition { parameters, .. }) => parameters
                .iter()
                .find_map(|param| {
                    if let actix_web::http::header::DispositionParam::Filename(ref filename) =
                        *param
                    {
                        Some(sanitize_filename::sanitize(filename))
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| "upload.bin".to_string()),
            None => "upload.bin".to_string(),
        };

        // create file
        let temp_file_path = PathBuf::from(format!("{target_dir_path}/{filename}"));
        let mut f = match fs::File::create(&temp_file_path).await {
            Ok(value) => value,
            Err(e) => {
                let path = temp_file_path
                    .as_os_str()
                    .to_str()
                    .unwrap_or("Invalid-path");
                let msg = format!("Failed to create upload-file '{path}' with error: {e}.");
                log::error!("{msg}");
                return Err(ErrorResponse::InternalError("Internal Error".to_string()));
            }
        };

        temp_file_paths.push(temp_file_path.clone());

        // fill content into file
        let result = async {
            while let Some(chunk) = field.next().await {
                let data = match chunk {
                    Ok(value) => value,
                    Err(e) => {
                        log::error!("Failed to fill content into file with error '{e}'");
                        return Err(ErrorResponse::BadRequest(
                            "Failed to read chunk.".to_string(),
                        ));
                    }
                };

                match f.write_all(&data).await {
                    Ok(()) => {}
                    Err(e) => {
                        log::error!("Failed to write all chunks into file with error: '{e}'");
                        return Err(ErrorResponse::InternalError("Internal Error".to_string()));
                    }
                };
            }

            match f.sync_all().await {
                Ok(()) => {}
                Err(e) => {
                    log::error!("Failed to sync file to disc with error: '{e}'");
                    return Err(ErrorResponse::InternalError("Internal Error".to_string()));
                }
            };

            Ok(())
        }
        .await;

        match result {
            Ok(_) => {}
            Err(e) => {
                log::debug!("Dataset-upload broken or canceled.");
                match std::fs::remove_file(&temp_file_path) {
                    Ok(()) => {}
                    Err(e) => {
                        let tempfile_path_str: String = temp_file_path.to_string_lossy().into();
                        log::error!(
                            "Failed to delete temp-file {tempfile_path_str} from disc with error {e}."
                        );
                    }
                }
                return Err(e);
            }
        }
    }

    Ok(temp_file_paths)
}

async fn convert_uploaded_files(
    dataset_uuid: &Uuid,
    name: &str,
    dataset_type: &String,
    target_filepath: &str,
    temp_file_paths: &[PathBuf],
) -> Result<(), ErrorResponse> {
    // process mnist-dataset
    if dataset_type == "mnist" {
        let path_len = temp_file_paths.len();
        if temp_file_paths.len() != 2 {
            let msg = format!(
                "MNIST-dataset expect 2 uploaded files, but there were {path_len} files found."
            );
            return Err(ErrorResponse::BadRequest(msg));
        }
        match load_mnist_images(
            &temp_file_paths[0],
            &temp_file_paths[1],
            target_filepath,
            *dataset_uuid,
            name,
            None,
        ) {
            Ok(()) => {}
            Err(e) => match e.downcast_ref::<AinariError>() {
                Some(AinariError::InvalidInput(e)) => {
                    let msg = e.to_string();
                    return Err(ErrorResponse::BadRequest(msg));
                }
                _ => {
                    log::error!("Failed to load mnist-images with error: '{e}'");
                    return Err(ErrorResponse::InternalError("Internal Error".to_string()));
                }
            },
        };
    } else if dataset_type == "csv" {
        let path_len = temp_file_paths.len();
        if temp_file_paths.len() != 1 {
            let msg = format!(
                "CSV-dataset expect 1 uploaded files, but there were {path_len} files found."
            );
            return Err(ErrorResponse::BadRequest(msg));
        }
        match load_csv_file(&temp_file_paths[0], target_filepath, *dataset_uuid, name) {
            Ok(()) => {}
            Err(e) => match e.downcast_ref::<AinariError>() {
                Some(AinariError::InvalidInput(e)) => {
                    let msg = e.to_string();
                    return Err(ErrorResponse::BadRequest(msg));
                }
                _ => {
                    log::error!("Failed to load csv-data with error: '{e}'");
                    return Err(ErrorResponse::InternalError("Internal Error".to_string()));
                }
            },
        };
    }

    Ok(())
}
