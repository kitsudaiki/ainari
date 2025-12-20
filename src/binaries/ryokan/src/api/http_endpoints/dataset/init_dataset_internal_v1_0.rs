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
use apistos::actix::CreatedJson;
use apistos::api_operation;
use validator::Validate;

use crate::database::dataset_table;
use crate::onsen_functions::select_onsen;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::dataset_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "dataset",
    summary = "Initialize new dataset",
    description = r###"Initialize  new dataset. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 409,
    error_code = 500
)]
pub async fn init_dataset(
    body: Json<DatasetInitReq>,
    context: UserContext,
) -> Result<CreatedJson<DatasetInternalResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let name = &body.name;
    let dataset_uuid = &body.uuid.clone();
    let file_path_str: String = format!("datasets/{}", dataset_uuid);

    super::check_dataset_quota(&context).await?;

    let (secret_uuid, _) = super::super::generate_new_key(dataset_uuid, &context).await?;

    let selected_onsen = select_onsen(&context)?;

    let dimension = (body.number_of_rows as i64, body.column_names.clone());
    dataset_table::add_new_dataset(
        dataset_uuid,
        name,
        &selected_onsen.address,
        &file_path_str,
        &secret_uuid,
        &dimension,
        &context,
    )
    .map_err(|e| {
        log::error!("Failed to add dataset to database: {e}");
        ErrorResponse::InternalError("Internal error".to_string())
    })?;

    let dataset_data = dataset_table::get_dataset(dataset_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("dataset", dataset_uuid, e))?;

    let secret_uuid = convert_uuid(&dataset_data.secret_uuid)?;
    let resp = DatasetInternalResp {
        uuid: *dataset_uuid,
        name: dataset_data.name,
        onsen_address: dataset_data.onsen_address,
        file_path: dataset_data.file_path,
        number_of_rows: dataset_data.number_of_rows as u64,
        column_names: body.column_names.clone(),
        secret_uuid,
        created_by: dataset_data.created_by,
        created_at: dataset_data.created_at,
        updated_by: dataset_data.updated_by,
        updated_at: dataset_data.updated_at,
    };

    Ok(CreatedJson(resp))
}
