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
use apistos::actix::CreatedJson;
use apistos::api_operation;
use validator::Validate;

use crate::database::image_table;
use crate::onsen_functions::select_onsen;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::image_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "image",
    summary = "Initialize new image",
    description = r###"Initialize a new image.

Prepares the database-entry and the onsen, before the files are uploaded.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn init_image(
    body: Json<ImageInitReq>,
    context: UserContext,
) -> Result<CreatedJson<ImageInternalResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let name = &body.name;
    let image_uuid = &body.uuid.clone();
    let file_path_str: String = format!("images/{}", image_uuid);

    super::check_image_quota(&context).await?;

    let (secret_uuid, _) = super::super::generate_new_key(image_uuid, &context).await?;

    let selected_onsen = select_onsen(&context)?;

    let dimension = (body.number_of_rows as i64, body.column_names.clone());
    image_table::add_new_image(
        image_uuid,
        name,
        &selected_onsen.address,
        &file_path_str,
        &secret_uuid,
        &dimension,
        &context,
    )
    .map_err(|e| {
        log::error!("Failed to add image to database: {e}");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    let image_data = image_table::get_image(image_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("image", image_uuid, e))?;

    let secret_uuid = convert_uuid(&image_data.secret_uuid)?;
    let resp = ImageInternalResp {
        uuid: *image_uuid,
        name: image_data.name,
        onsen_address: image_data.onsen_address,
        file_path: image_data.file_path,
        number_of_rows: image_data.number_of_rows as u64,
        column_names: body.column_names.clone(),
        secret_uuid,
        created_by: image_data.created_by,
        created_at: image_data.created_at,
        updated_by: image_data.updated_by,
        updated_at: image_data.updated_at,
    };

    Ok(CreatedJson(resp))
}
