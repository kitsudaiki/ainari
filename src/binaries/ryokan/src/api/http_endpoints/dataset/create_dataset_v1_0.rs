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

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::dataset_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::error::AinariError;
use ainari_dataset::dataset_io::read_data_set_file;
use ainari_dataset::file_encryption::encrypt_file;

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
    let target_dir_path = format!(
        "{}/{}",
        config::CONFIG.storage.tempfile_location,
        dataset_uuid
    );
    let converted_result_path = format!("{target_dir_path}/converted_result");
    let encrypted_result_path = format!("{target_dir_path}/encrypted_result");
    let upload_file_path_str: String = format!("datasets/{dataset_uuid}");

    super::check_dataset_type(&dataset_type)?;

    super::check_dataset_quota(&context).await?;

    create_directory(&target_dir_path).await?;

    let selected_onsen = select_onsen(&context)?;

    // handle payload, create key encrypt data and upload them to the selected onsen
    let result = {
        let temp_file_paths = write_payload_into_file(payload, &target_dir_path).await?;

        convert_uploaded_files(
            &dataset_uuid,
            &name,
            &dataset_type,
            &converted_result_path,
            &temp_file_paths,
        )
        .await?;

        let (number_of_rows, column_names) = get_dataset_dimension(&converted_result_path)?;

        let (secret_uuid, secret) = super::super::generate_new_key(&dataset_uuid, &context).await?;

        encrypt_file(&converted_result_path, &encrypted_result_path, &secret)
            .await
            .map_err(map_ainari_error_to_api_response)?;

        upload_file_to_onsen(
            &selected_onsen.address,
            &upload_file_path_str,
            &encrypted_result_path,
        )
        .await?;

        Ok((number_of_rows, column_names, secret_uuid))
    };

    // remove temporary directory again
    super::remove_all(&target_dir_path);

    let (number_of_rows, column_names, secret_uuid) = result?;

    let dimension = (number_of_rows as i64, column_names.clone());
    dataset_table::add_new_dataset(
        &dataset_uuid,
        &name,
        &selected_onsen.address,
        &upload_file_path_str,
        &secret_uuid,
        &dimension,
        &context,
    )
    .map_err(|e| {
        log::error!("Failed to add dataset to database: {e}");
        ErrorResponse::InternalError("Internal error".to_string())
    })?;

    let dataset_data = dataset_table::get_dataset(&dataset_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("dataset", &dataset_uuid, e))?;

    let resp = DatasetResp {
        uuid: dataset_uuid,
        name: dataset_data.name,
        number_of_rows: dataset_data.number_of_rows as u64,
        column_names,
        created_by: dataset_data.created_by,
        created_at: dataset_data.created_at,
        updated_by: dataset_data.updated_by,
        updated_at: dataset_data.updated_at,
    };

    Ok(CreatedJson(resp))
}

async fn write_payload_into_file(
    mut payload: Multipart,
    target_dir_path: &String,
) -> Result<Vec<PathBuf>, ErrorResponse> {
    let mut temp_file_paths = Vec::new();
    // process items from payload
    while let Some(item) = payload.next().await {
        let mut field = item.map_err(|_| {
            ErrorResponse::BadRequest("Failed to read next item from input.".to_string())
        })?;

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
        let mut f = fs::File::create(&temp_file_path).await.map_err(|e| {
            let path = temp_file_path
                .as_os_str()
                .to_str()
                .unwrap_or("Invalid-path");
            log::error!("Failed to create upload-file '{path}' with error: {e}.");
            ErrorResponse::InternalError("Internal Error".to_string())
        })?;

        temp_file_paths.push(temp_file_path.clone());

        // fill content into file
        let result = async {
            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| {
                    log::error!("Failed to fill content into file with error '{e}'");
                    ErrorResponse::InternalError("Internal Error".to_string())
                })?;

                f.write_all(&data).await.map_err(|e| {
                    log::error!("Failed to write all chunks into file with error: '{e}'");
                    ErrorResponse::InternalError("Internal Error".to_string())
                })?;
            }

            f.sync_all().await.map_err(|e| {
                log::error!("Failed to sync file to disc with error: '{e}'");
                ErrorResponse::InternalError("Internal Error".to_string())
            })?;

            Ok(())
        }
        .await;

        match result {
            Ok(_) => {}
            Err(e) => {
                log::debug!("Dataset-upload broken or canceled.");
                let _ = std::fs::remove_file(&temp_file_path).map_err(|e| {
                    let tempfile_path_str: String = temp_file_path.to_string_lossy().into();
                    log::error!(
                        "Failed to delete temp-file {tempfile_path_str} from disc with error {e}."
                    );
                });
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
            return Err(ErrorResponse::BadRequest(format!(
                "MNIST-dataset expect 2 uploaded files, but there were {path_len} files found."
            )));
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
            return Err(ErrorResponse::BadRequest(format!(
                "CSV-dataset expect 1 uploaded files, but there were {path_len} files found."
            )));
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

fn get_dataset_dimension(target_path: &String) -> Result<(u64, Vec<String>), ErrorResponse> {
    let file_handle = read_data_set_file(target_path).map_err(|e| {
        log::error!("Failed to read dataset dimensions from file '{target_path}' with error: {e}");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    let number_of_rows = file_handle.get_number_of_rows();

    let mut column_names: Vec<String> = Vec::new();
    for col in file_handle.header.columns {
        column_names.push(col.0);
    }

    Ok((number_of_rows, column_names))
}
