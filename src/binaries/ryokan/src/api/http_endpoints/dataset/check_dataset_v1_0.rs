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

use actix_web::web::Json;
use actix_web::web::Path;
use ainari_common::error::AinariError;
use apistos::api_operation;
use bytemuck::cast_slice_mut;
use std::cmp::Ordering;
use std::fs;
use std::io::SeekFrom;
use std::io::{Read, Seek};
use uuid::Uuid;
use validator::Validate;

use crate::config;
use crate::database::dataset_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::dataset_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::get_endpoints;
use ainari_clients::onsen_file_transfer::*;
use ainari_clients::secret::get_secret_payload;
use ainari_common::secret::Secret;
use ainari_dataset::dataset_io::read_data_set_file;
use ainari_dataset::dataset_io::{Column, DataSetFileReadHandle};
use ainari_dataset::file_encryption::decrypt_file;

#[api_operation(
    tag = "dataset",
    summary = "Check dataset",
    description = r###"Check two datasets against each other to get the accurary compared to the reference."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn check_dataset(
    body: Json<DatasetCheckReq>,
    dataset_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<DatasetCheckResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let dataset_uuid = dataset_uuid;
    let reference_uuid = body.reference_uuid;
    let dataset_column = body.dataset_column.clone();
    let reference_column = body.reference_column.clone();

    // create directory, where all temp-files of this operation are stored
    let compare_dir = format!(
        "{}/cmp_{}_{}",
        config::CONFIG.storage.tempfile_location,
        dataset_uuid,
        reference_uuid
    );
    create_directory(&compare_dir).await?;

    let result = {
        // get data to compare
        let (mut dataset_file_handle, dataset_col_get, mut row_count) =
            get_dataset_column(&dataset_uuid, &dataset_column, &compare_dir, &context).await?;
        let (mut reference_file_handle, ref_col_get, ref_row_count) =
            get_dataset_column(&reference_uuid, &reference_column, &compare_dir, &context).await?;

        if row_count > ref_row_count {
            row_count = ref_row_count;
        }

        let mut accuracy = 0f32;

        for i in 0..row_count {
            let correct = check_row(
                &mut dataset_file_handle,
                &dataset_col_get,
                &mut reference_file_handle,
                &ref_col_get,
                i,
            )
            .map_err(|e| {
                log::error!("{e}");
                ErrorResponse::InternalError("Internal Error".to_string())
            })?;

            if correct {
                accuracy += 1f32;
            }
        }

        let resp = DatasetCheckResp {
            accuracy: accuracy / row_count as f32,
        };

        Ok(resp)
    };

    // delete temp-directory first before handling error-returns
    super::remove_all(&compare_dir);
    let resp = result?;

    Ok(Json(resp))
}

fn check_row(
    dataset_file_handle: &mut DataSetFileReadHandle,
    dataset_column: &Column,
    reference_file_handle: &mut DataSetFileReadHandle,
    reference_columns: &Column,
    row: u64,
) -> Result<bool, AinariError> {
    let dataset_row = get_highest_pos_in_row(dataset_file_handle, dataset_column, row)?;
    let reference_row = get_highest_pos_in_row(reference_file_handle, reference_columns, row)?;
    // println!("row: {row}    dataset_row: {dataset_row}  reference_row: {reference_row}");

    if dataset_row != reference_row {
        return Ok(false);
    }

    Ok(true)
}

fn get_highest_pos_in_row(
    file_handle: &mut DataSetFileReadHandle,
    col_get: &Column,
    row: u64,
) -> Result<u64, AinariError> {
    // calculate position in dataset-file
    let size_input = (col_get.end - col_get.start) as usize;
    let mut offset_bytes = (file_handle.header.row_size) * 4 * row;
    offset_bytes += col_get.start * 4;

    let mut input_read = vec![0.0f32; size_input];
    let byte_slice_input: &mut [u8] = cast_slice_mut(input_read.as_mut_slice());
    let start_pos = file_handle.payload_offset + offset_bytes;
    match file_handle.target_file.seek(SeekFrom::Start(start_pos)) {
        Ok(_) => {}
        Err(e) => {
            let msg = format!("Failed to set file-seek to position {start_pos} with error: '{e}'");
            log::error!("{msg}");
            return Err(AinariError::Error(msg));
        }
    }
    let _ = file_handle.target_file.read_exact(byte_slice_input);

    // println!("{:?}", input_read);
    if let Some((index, _)) = input_read
        .iter()
        .enumerate()
        .filter(|(_, v)| v.is_finite())
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(Ordering::Less))
    {
        Ok(index as u64)
    } else {
        let msg = "Failed to get hightest index in output".to_string();
        log::error!("{msg}");
        Err(AinariError::Error(msg))
    }
}

async fn get_dataset_column(
    dataset_uuid: &Uuid,
    column_name: &String,
    compare_dir: &String,
    context: &UserContext,
) -> Result<(DataSetFileReadHandle, Column, u64), ErrorResponse> {
    let dataset_resp = dataset_table::get_dataset(dataset_uuid, context)
        .map_err(|e| map_db_uuid_get_delete_error("dataset", dataset_uuid, e))?;

    let secret_uuid = convert_uuid(&dataset_resp.secret_uuid)?;
    let secret = get_secret(&secret_uuid, context).await?;

    // create temporary file-paths
    let local_file_path = format!("{compare_dir}/{dataset_uuid}");
    let local_encrypted_file_path = format!("{local_file_path}_encrypted");

    download_file(
        &dataset_resp.onsen_address,
        &dataset_resp.file_path,
        &local_encrypted_file_path,
    )
    .await
    .map_err(|e| {
        log::error!("Failed to download dataset-file from onsen: {e}");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    decrypt_file(&local_encrypted_file_path, &local_file_path, &secret)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    let _ = fs::remove_file(&local_encrypted_file_path);

    let file_handle = read_data_set_file(&local_file_path).map_err(|e| {
        log::error!(
            "Failed to read dataset-file '{}' with error: {e}",
            dataset_resp.file_path
        );
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    // get column-information
    let dataset_col_get = match file_handle.header.columns.get(column_name) {
        Some(col) => col.clone(),
        _ => {
            let msg = format!(
                "Column with name '{column_name}' not found in dataset with UUID '{dataset_uuid}."
            );
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    let row_count = file_handle.get_number_of_rows();

    Ok((file_handle, dataset_col_get, row_count))
}

async fn get_secret(secret_uuid: &Uuid, context: &UserContext) -> Result<Secret, ErrorResponse> {
    let miko_endpoint = &config::CONFIG.miko;
    let endpoints = get_endpoints(miko_endpoint, config::CONFIG.skip_tls_verification)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    let secret_payload = get_secret_payload(
        &endpoints.omamori,
        &context.token,
        secret_uuid,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    Ok(Secret::from(secret_payload.secret_payload))
}
