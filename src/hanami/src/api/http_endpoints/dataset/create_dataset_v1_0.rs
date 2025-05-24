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

use actix_web::web::Path;
use actix_web::http::header::ContentDisposition;
use actix_multipart::Multipart;
use apistos::actix::CreatedJson;
use apistos::api_operation;
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;
use tokio::fs;
use log::{error, debug};
use std::path::PathBuf;
use uuid::Uuid;

use crate::api::user_context::UserContext;
use crate::api::errors::ErrorResponse;
use crate::database::dataset_table;
use crate::config;

use hanami_dataset::converter::{load_mnist_images, load_csv_file};
use hanami_dataset::dataset_io::read_data_set_file;
use hanami_common::error::HanamiError;

use super::dataset_structs::DatasetResp;

#[api_operation(
    tag = "dataset",
    summary = "Create new dataset",
    description = r###"Create new dataset by uploading files."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn upload_binary(mut payload: Multipart, path: Path<(String, String)>, context: UserContext) -> Result<CreatedJson<DatasetResp>, ErrorResponse> {
    let tempfile_location = config::CONFIG.storage.tempfile_location.clone();
    let dataset_location = config::CONFIG.storage.dataset_location.clone();

    let tempfile_dir = PathBuf::from(&tempfile_location);
    let dataset_dir = PathBuf::from(&dataset_location);

    let dataset_uuid = Uuid::new_v4();
    let target_filepath: PathBuf = dataset_dir.join(&dataset_uuid.to_string());

    let (dataset_type_str, name) = path.into_inner();
    let dataset_type = dataset_type_str.to_string();

    // check given type
    if ["mnist", "csv"].contains(&dataset_type.as_str()) == false {
        let msg = format!("Type '{dataset_type}' is not in list [ mnist, csv ]");
        return Err(ErrorResponse::BadRequest(msg.to_string()));
    }

    // Ensure directory exists
    match fs::create_dir_all(&tempfile_dir).await {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to create dataset-upload-directory '{tempfile_location}' with error: {e}");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    }
    match fs::create_dir_all(&dataset_dir).await {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to create dataset-upload-directory '{dataset_location}' with error: {e}");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    }

    let mut temp_file_paths = Vec::new();
    // process items from payload
    while let Some(item) = payload.next().await {
        let mut field = match item {
            Ok(value) => value,
            Err(_) => return Err(ErrorResponse::BadRequest("Failed to read next item from input.".to_string())),
        };

        // get file-name of item
        let content_disposition = field.content_disposition();
        let filename = match content_disposition {
            Some(ContentDisposition { parameters, .. }) => {
                parameters.iter().find_map(|param| {
                    if let actix_web::http::header::DispositionParam::Filename(ref filename) = *param {
                        Some(sanitize_filename::sanitize(filename))
                    } else {
                        None
                    }
                }).unwrap_or_else(|| "upload.bin".to_string())
            }
            None => "upload.bin".to_string(),
        };

        // create file
        let temp_file_path: PathBuf = tempfile_dir.join(filename + &dataset_uuid.to_string());
        let mut f = match fs::File::create(&temp_file_path).await {
            Ok(value) => value,
            Err(e) => {
                let path = temp_file_path.as_os_str().to_str().unwrap();
                let msg = format!("Failed to create upload-file '{path}' with error: {e}.");
                error!("{}", msg);
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
                        error!("{}", e);
                        return Err(ErrorResponse::BadRequest("Failed to read chunk.".to_string()));
                    }
                };
                
                let _ = f.write_all(&data).await;
            }
            
            Ok(())
        }
        .await;

        match result {
            Ok(_) => {},
            Err(e) => {
                debug!("Dataset-upload broken or canceled.");
                match std::fs::remove_file(&temp_file_path) {
                    Ok(()) => {},
                    Err(e) => {
                        let tempfile_path_str: String = temp_file_path.to_string_lossy().into();
                        error!("Failed to delete temp-file {tempfile_path_str} from disc with error {}.", e);
                    }
                }
                return Err(e);
            }
        }
    }

    // process mnist-dataset
    if dataset_type == "mnist" {
        let path_len = temp_file_paths.len();
        if temp_file_paths.len() != 2 {
            let msg = format!("MNIST-dataset expect 2 uploaded files, but there were {path_len} files found.");
            return Err(ErrorResponse::BadRequest(msg));
        }
        match load_mnist_images(
            &temp_file_paths[0], 
            &temp_file_paths[1], 
            &target_filepath,
            dataset_uuid.clone(),
            name.clone(),
            None) 
        {
            Ok(()) => {},
            Err(e) => match e.downcast_ref::<HanamiError>() {
                Some(HanamiError::InputError(e)) => {
                    let msg = format!("{}", e);
                    return Err(ErrorResponse::BadRequest(msg));
                },
                _ => {
                    error!("{}", e);
                    return Err(ErrorResponse::InternalError("".to_string()));
                }
            },
        };
    } else if dataset_type == "csv" {
        let path_len = temp_file_paths.len();
        if temp_file_paths.len() != 1 {
            let msg = format!("CSV-dataset expect 1 uploaded files, but there were {path_len} files found.");
            return Err(ErrorResponse::BadRequest(msg));
        }
        match load_csv_file(
            &temp_file_paths[0], 
            &target_filepath,
            dataset_uuid.clone(),
            name.clone()) 
        {
            Ok(()) => {},
            Err(e) => match e.downcast_ref::<HanamiError>() {
                Some(HanamiError::InputError(e)) => {
                    let msg = format!("{}", e);
                    return Err(ErrorResponse::BadRequest(msg));
                },
                _ => {
                    error!("{}", e);
                    return Err(ErrorResponse::InternalError("".to_string()));
                }
            },
        };
    }

    // add new dataset to datbase
    let file_path_str: String = target_filepath.to_string_lossy().into();
    match dataset_table::add_new_dataset(&dataset_uuid, &name, &file_path_str, &context) {
        Ok(_) => {},
        Err(_) => {
            error!("Failed to add dataset with ID '{dataset_uuid}' to database.");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // get new created dataset from database to get addtional information
    let dataset = match dataset_table::get_dataset(&dataset_uuid, &context) {
        Ok(dataset) => dataset,
        Err(_) => 
        {
            error!("Failed to get dataset with ID '{dataset_uuid}' from database, even the user should exist.");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    for file_path in temp_file_paths {
        match std::fs::remove_file(&file_path) {
            Ok(()) => {},
            Err(e) => {
                let tempfile_path_str: String = file_path.to_string_lossy().into();
                error!("Failed to delete temp-file {tempfile_path_str} from disc with error {}.", e);
            }
        }
    }

    let file_handle = match read_data_set_file(&target_filepath) {
        Ok(file_handle) => file_handle,
        Err(_) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // create response
    let resp = DatasetResp {
        uuid: dataset_uuid.clone(),
        name: dataset.name.clone(),
        number_of_rows: file_handle.get_number_of_rows(),
        number_of_columns: file_handle.header.columns.len() as u64,
        created_by: dataset.created_by.clone(),
        created_at: dataset.created_at.clone(),
        updated_by: dataset.updated_by.clone(),
        updated_at: dataset.updated_at.clone(),
    };

    return Ok(CreatedJson(resp));
}
