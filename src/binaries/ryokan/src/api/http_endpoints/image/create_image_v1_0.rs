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
use crate::database::image_table;
use crate::onsen_functions::select_onsen;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::image_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_files::file_encryption::encrypt_file;

#[api_operation(
    tag = "image",
    summary = "Create new image",
    description = r###"Create new image by uploading files."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn upload_binary(
    payload: Multipart,
    path: Path<(String, String)>,
    context: UserContext,
) -> Result<CreatedJson<ImageResp>, ErrorResponse> {
    let (image_type, name) = path.into_inner();
    let image_uuid = Uuid::new_v4();
    let target_dir_path = format!(
        "{}/{}",
        config::CONFIG.storage.tempfile_location,
        image_uuid
    );
    let encrypted_result_path = format!("{target_dir_path}/encrypted_result");
    let upload_file_path_str: String = format!("images/{image_uuid}");

    super::check_image_type(&image_type)?;

    super::check_image_quota(&context).await?;

    create_directory_api(&target_dir_path).await?;

    let selected_onsen = select_onsen(&context)?;

    // handle payload, create key encrypt data and upload them to the selected onsen
    let result = {
        let temp_file_paths = write_payload_into_file(payload, &target_dir_path).await?;

        // a disk-image is the boot-disk of a virtual_machine, which is stored as it is. There is
        // nothing to convert and it has no rows and columns like the data-sets.
        let source_path = get_disk_image_path(&temp_file_paths)?;
        let number_of_rows: u64 = 0;
        let column_names: Vec<String> = Vec::new();

        let (secret_uuid, secret) = super::super::generate_new_key(&image_uuid, &context).await?;

        encrypt_file(&source_path, &encrypted_result_path, &secret)
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
    image_table::add_new_image(
        &image_uuid,
        &name,
        &selected_onsen.address,
        &upload_file_path_str,
        &secret_uuid,
        &dimension,
        false,
        &context,
    )
    .map_err(|e| {
        log::error!("Failed to add image to database: {e}");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    let image_data = image_table::get_image(&image_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("image", &image_uuid, e))?;

    let resp = ImageResp {
        uuid: image_uuid,
        name: image_data.name,
        number_of_rows: image_data.number_of_rows as u64,
        column_names,
        is_snapshot: image_data.is_snapshot,
        created_by: image_data.created_by,
        created_at: image_data.created_at,
        updated_by: image_data.updated_by,
        updated_at: image_data.updated_at,
    };

    Ok(CreatedJson(resp))
}

/// Returns the path of the single uploaded file of a disk-image.
///
/// # Arguments
///
/// * `temp_file_paths` - Paths to the temporary files containing the uploaded data
///
/// # Returns
///
/// A `Result` containing the path of the uploaded disk-image, or an `ErrorResponse`, if there was
/// not exactly one file uploaded
fn get_disk_image_path(temp_file_paths: &[PathBuf]) -> Result<String, ErrorResponse> {
    let path_len = temp_file_paths.len();
    if path_len != 1 {
        return Err(ErrorResponse::BadRequest(format!(
            "Disk-image expect 1 uploaded files, but there were {path_len} files found."
        )));
    }

    match temp_file_paths[0].to_str() {
        Some(path) => Ok(path.to_string()),
        None => {
            log::error!("Path of the uploaded disk-image is not valid utf-8.");
            Err(ErrorResponse::InternalError("Internal Error".to_string()))
        }
    }
}

/// Writes the contents of a multipart payload to temporary files.
///
/// This function processes each part of the multipart payload, extracts filenames,
/// creates temporary files, and writes the contents to those files.
///
/// # Arguments
///
/// * `payload` - The multipart data to process
/// * `target_dir_path` - The directory where temporary files will be created
///
/// # Returns
///
/// A `Result` with either:
/// - `Vec<PathBuf>` containing paths to created temporary files on success
/// - `ErrorResponse` on failure
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
                log::debug!("Image-upload broken or canceled.");
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
