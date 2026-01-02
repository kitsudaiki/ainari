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

use actix_web::web::Json;
use actix_web::web::Path;
use apistos::api_operation;
use uuid::Uuid;

use crate::database::dataset_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::dataset_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "dataset",
    summary = "Get dataset (internal)",
    description = r###"Get information of a dataset from the database with additional information."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_dataset_internal(
    dataset_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<DatasetInternalResp>, ErrorResponse> {
    let dataset_data = dataset_table::get_dataset(&dataset_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("dataset", &dataset_uuid, e))?;

    // deserialize name-lists
    let column_names: Vec<String> =
        serde_json::from_str(&dataset_data.column_names).map_err(|e| {
            log::error!("Failed to deserialize column_names: '{e}'");
            ErrorResponse::InternalError("Internal Error".to_string())
        })?;

    let secret_uuid = convert_uuid(&dataset_data.secret_uuid)?;
    let resp = DatasetInternalResp {
        uuid: *dataset_uuid,
        name: dataset_data.name,
        onsen_address: dataset_data.onsen_address,
        file_path: dataset_data.file_path,
        number_of_rows: dataset_data.number_of_rows as u64,
        column_names,
        secret_uuid,
        created_by: dataset_data.created_by,
        created_at: dataset_data.created_at,
        updated_by: dataset_data.updated_by,
        updated_at: dataset_data.updated_at,
    };

    Ok(Json(resp))
}
