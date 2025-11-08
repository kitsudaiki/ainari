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
use crate::database::dataset_table;
use crate::onsen_functions::select_onsen;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::dataset_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::onsen_file_transfer;
use ainari_clients::quota::get_quota;
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

    // check given type
    if !["mnist", "csv"].contains(&dataset_type.as_str()) {
        let msg = format!("Type '{dataset_type}' is not in list [ mnist, csv ]");
        return Err(ErrorResponse::BadRequest(msg.to_string()));
    }
    // create path
    let tempfile_dir = config::CONFIG.storage.tempfile_location.clone();
    let target_dir_path = format!("{tempfile_dir}/{}", dataset_uuid);
    let converted_result_path = format!("{target_dir_path}/converted_result");
    let upload_file_path_str: String = format!("datasets/{dataset_uuid}");

    // Ensure directory exists
    match fs::create_dir_all(&target_dir_path).await {
        Ok(_) => (),
        Err(e) => {
            log::error!(
                "Failed to create dataset-upload-directory '{target_dir_path}' with error: {e}"
            );
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    }

    check_quota(&context).await?;

    let selected_onsen = select_onsen(&context)?;

    let temp_file_paths = handle_payload(payload, &target_dir_path).await?;

    handle_uploaded_files(
        &dataset_uuid,
        &name,
        &dataset_type,
        &converted_result_path,
        &temp_file_paths,
    )
    .await?;

    // upload converted file to the selected onsen
    match onsen_file_transfer::upload_file(
        &selected_onsen.address,
        &converted_result_path,
        &upload_file_path_str,
    )
    .await
    {
        Ok(()) => {}
        Err(e) => {
            let onsen_addr = selected_onsen.address;
            log::error!(
                "Failed to send file with path '{converted_result_path}' to onsen '{onsen_addr}' to '{upload_file_path_str}' with error: {e}"
            );
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    }

    // delete all temporary files
    match std::fs::remove_dir_all(&target_dir_path) {
        Ok(()) => {}
        Err(e) => {
            log::error!("Failed to delete temp-dir {target_dir_path} from disk with error {e}.");
        }
    }

    // add new dataset to datbase
    match dataset_table::add_new_dataset(
        &dataset_uuid,
        &name,
        &selected_onsen.address,
        &upload_file_path_str,
        &context,
    ) {
        Ok(_) => {}
        Err(_) => {
            log::error!("Failed to add dataset with ID '{dataset_uuid}' to database.");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // get new created dataset from database to get addtional information
    let dataset = match dataset_table::get_dataset(&dataset_uuid, &context) {
        Ok(dataset) => dataset,
        Err(_) => {
            log::error!(
                "Failed to get dataset with ID '{dataset_uuid}' from database, even the user should exist."
            );
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    let (number_of_rows, number_of_columns) = match onsen_file_transfer::get_dataset_dimension(
        &dataset.onsen_address,
        &dataset.file_path,
    )
    .await
    {
        Ok((number_of_rows, number_of_columns)) => (number_of_rows, number_of_columns),
        Err(e) => {
            let onsen_addr = dataset.onsen_address;
            let file_path = dataset.file_path;
            log::error!(
                "Failed to get dataset-dimensions form onsen '{onsen_addr}' to '{file_path}' with error: {e}"
            );
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // create response
    let resp = DatasetResp {
        uuid: dataset_uuid,
        name: dataset.name.clone(),
        number_of_rows: number_of_rows as u64,
        number_of_columns: number_of_columns as u64,
        created_by: dataset.created_by.clone(),
        created_at: dataset.created_at.clone(),
        updated_by: dataset.updated_by.clone(),
        updated_at: dataset.updated_at.clone(),
    };

    return Ok(CreatedJson(resp));
}

async fn handle_payload(
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
                return Err(ErrorResponse::InternalError("".to_string()));
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
                        return Err(ErrorResponse::InternalError("".to_string()));
                    }
                };
            }

            match f.sync_all().await {
                Ok(()) => {}
                Err(e) => {
                    log::error!("Failed to sync file to disc with error: '{e}'");
                    return Err(ErrorResponse::InternalError("".to_string()));
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

async fn handle_uploaded_files(
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
                    return Err(ErrorResponse::InternalError("".to_string()));
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
                    return Err(ErrorResponse::InternalError("".to_string()));
                }
            },
        };
    }

    Ok(())
}

async fn check_quota(context: &UserContext) -> Result<(), ErrorResponse> {
    // get number of datasets of the user
    let current_number_of_datasets = match dataset_table::count_datasets(context) {
        Ok(number) => number,
        Err(e) => {
            log::error!("Failed to count datasets in database.: {e}");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // check the maximum number of datasets defined in miko
    let miko_endpoint = &config::CONFIG.miko;
    let max_number_of_datasets = match get_quota(
        miko_endpoint,
        &context.token,
        &context.user_id,
        config::CONFIG.insecure_clients,
    )
    .await
    {
        Ok(body) => body.max_dataset as i64,
        Err(AinariError::Unauthorized(msg)) => {
            return Err(ErrorResponse::Unauthorized(msg));
        }
        Err(AinariError::InvalidInput(msg)) => {
            return Err(ErrorResponse::BadRequest(msg));
        }
        Err(AinariError::Error(msg)) => {
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // check if quota is already exceeded
    if current_number_of_datasets as i64 >= max_number_of_datasets {
        return Err(ErrorResponse::Conflict(
            "Maximum number of datasets exceeded.".to_string(),
        ));
    }

    Ok(())
}
