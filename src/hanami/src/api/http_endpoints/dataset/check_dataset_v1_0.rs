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
use uuid::Uuid;
use apistos::api_operation;
use std::path::PathBuf;
use std::io::SeekFrom;
use std::io::{Read, Seek};
use bytemuck::cast_slice_mut;

use crate::api::errors::ErrorResponse;
use crate::api::user_context::UserContext;
use crate::database::dataset_table;

use hanami_common::enums;
use hanami_dataset::dataset_io::read_data_set_file;

use super::dataset_structs::{DatasetCheckResp, DatasetCheckReq};
use hanami_dataset::dataset_io::{DataSetFileReadHandleV1_0, Column};

#[api_operation(
    tag = "dataset",
    summary = "Check dataset",
    description = r###"Check two datasets against each other to get the accurary compared to the reference."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn check_dataset(body: Json<DatasetCheckReq>, dataset_uuid: Path<Uuid>, context: UserContext) -> Result<Json<DatasetCheckResp>, ErrorResponse> {
    let dataset_uuid = dataset_uuid;
    let reference_uuid = body.reference_uuid;
    let dataset_column = body.dataset_column.clone();
    let reference_column = body.reference_column.clone();

    // get information from dataset
    let dataset_data = match dataset_table::get_dataset(&dataset_uuid, &context) {
        Ok(dataset_data) => dataset_data,
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        },
        Err(enums::DbError::NotFound) => {
            let msg = format!("Dataset with UUID '{dataset_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    let reference_data = match dataset_table::get_dataset(&reference_uuid, &context) {
        Ok(reference_data) => reference_data,
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        },
        Err(enums::DbError::NotFound) => {
            let msg = format!("Dataset with UUID '{reference_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    // get file-handle
    let mut dataset_file_handle = match read_data_set_file(&PathBuf::from(dataset_data.file_path)) {
        Ok(dataset_file_handle) => dataset_file_handle,
        Err(_) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    let mut reference_file_handle = match read_data_set_file(&PathBuf::from(reference_data.file_path)) {
        Ok(reference_file_handle) => reference_file_handle,
        Err(_) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // get column-information
    let dataset_col_get = match dataset_file_handle.header.columns.get(&dataset_column) {
        Some(col) => col.clone(),
        _ => {
            let msg = format!("Column with name '{dataset_column}' not found in dataset.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    let ref_col_get = match reference_file_handle.header.columns.get(&reference_column) {
        Some(col) => col.clone(),
        _ => {
            let msg = format!("Column with name '{reference_column}' not found in reference-dataset.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };


    // get number of rows to check
    let mut row_count = dataset_file_handle.get_number_of_rows();
    let ref_row_count = reference_file_handle.get_number_of_rows();
    if row_count > ref_row_count {
        row_count = ref_row_count;
    }

    let mut accuracy = 0f32;

    for i in 0..row_count {
        if check_row(&mut dataset_file_handle, 
            &dataset_col_get, 
                     &mut reference_file_handle, 
                     &ref_col_get, 
                     i) == true {
            accuracy += 1f32;
        }
    }

    let resp = DatasetCheckResp {
        accuracy: accuracy / row_count as f32,
    };

    return Ok(Json(resp));
}

fn check_row(dataset_file_handle: &mut DataSetFileReadHandleV1_0,
             dataset_column: &Column,
             reference_file_handle: &mut DataSetFileReadHandleV1_0,
             reference_columns: &Column,
             row: u64) -> bool 
{
    let dataset_row = get_highest_pos_in_row(dataset_file_handle, dataset_column, row); 
    let reference_row = get_highest_pos_in_row(reference_file_handle, reference_columns, row); 
    // println!("row: {row}    dataset_row: {dataset_row}  reference_row: {reference_row}");

    if dataset_row != reference_row {
        return false;
    }

    true
}

fn get_highest_pos_in_row(file_handle: &mut DataSetFileReadHandleV1_0,
                          col_get: &Column,
                          row: u64) -> u64
{
    // calculate position in dataset-file
    let size_input = (col_get.end - col_get.start) as usize;
    let mut offset_bytes = (file_handle.header.row_size) * 4 * row;
    offset_bytes += col_get.start * 4;

    let mut input_read = vec![0.0f32; size_input];
    let byte_slice_input: &mut [u8] = cast_slice_mut(input_read.as_mut_slice());
    file_handle.target_file.seek(SeekFrom::Start(file_handle.payload_offset + offset_bytes)).unwrap();
    let _ = file_handle.target_file.read_exact(byte_slice_input);

    // println!("{:?}", input_read);
    if let Some((index, _)) = input_read
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
    {
        return index as u64;
    } else {
        return 0;
    }
}