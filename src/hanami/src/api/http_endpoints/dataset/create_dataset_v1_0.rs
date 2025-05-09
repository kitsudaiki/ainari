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
use log::error;
use std::path::PathBuf;
use uuid::Uuid;

use crate::api::user_context::UserContext;
use crate::api::errors::ErrorResponse;
use crate::database::dataset_table;
use crate::config;

use hanami_dataset::dataset_io::{DataSetType, init_new_data_set_file, Column};
use hanami_dataset::converter::load_mnist_images;
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
    let upload_dir_path = config::CONFIG.storage.dataset_location.clone();
    let upload_dir = PathBuf::from(&upload_dir_path);
    let dataset_uuid = Uuid::new_v4();
    let target_filepath: PathBuf = upload_dir.join(&dataset_uuid.to_string());

    let (dataset_type_str, name) = path.into_inner();
    let dataset_type = dataset_type_str.to_string();

    // check given type
    if ["mnist", "csv"].contains(&dataset_type.as_str()) == false {
        let msg = format!("Type '{dataset_type}' is not in list [ mnist, csv ]");
        return Err(ErrorResponse::BadRequest(msg.to_string()));
    }

    // Ensure directory exists
    match fs::create_dir_all(&upload_dir).await {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to create dataset-upload-directory '{upload_dir_path}' with error: {e}");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    }

    let mut filepaths = Vec::new();
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
        let filepath: PathBuf = upload_dir.join(filename + &dataset_uuid.to_string());
        let mut f = match fs::File::create(&filepath).await {
            Ok(value) => value,
            Err(e) => {
                let path = filepath.as_os_str().to_str().unwrap();
                let msg = format!("Failed to create upload-file '{path}' with error: {e}.");
                error!("{}", msg);
                return Err(ErrorResponse::InternalError("".to_string()));
            }
        };

        filepaths.push(filepath);

        // fill content into file
        while let Some(chunk) = field.next().await {
            let data = match chunk {
                Ok(value) => value,
                Err(_) => return Err(ErrorResponse::BadRequest("Failed to read chunk.".to_string())),
            };
            
            let _ = f.write_all(&data).await;
        }
    }

    // process mnist-dataset
    if dataset_type == "mnist" {
        let path_len = filepaths.len();
        if filepaths.len() != 2 {
            let msg = format!("MNIST-dataset expect 2 uploaded files, but there were {path_len} files found.");
            return Err(ErrorResponse::BadRequest(msg));
        }
        match load_mnist_images(
            &filepaths[0], 
            &filepaths[1], 
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
    match dataset_table::get_dataset(&dataset_uuid, &context) {
        Ok(dataset) => {
            let resp = DatasetResp {
                uuid: dataset_uuid.clone(),
                name: dataset.name.clone(),
                created_by: dataset.created_by.clone(),
                created_at: dataset.created_at.clone(),
                updated_by: dataset.updated_by.clone(),
                updated_at: dataset.updated_at.clone(),
            };
        
            return Ok(CreatedJson(resp));
        },
        Err(_) => 
        {
            error!("Failed to get dataset with ID '{dataset_uuid}' from database, even the user should exist.");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };
}
