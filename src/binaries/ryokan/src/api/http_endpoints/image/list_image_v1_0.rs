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
use apistos::api_operation;

use crate::database::image_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::image_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "image",
    summary = "List images",
    description = r###"List basic information of all images from the database."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_image(context: UserContext) -> Result<Json<ImageListResp>, ErrorResponse> {
    let images = image_table::list_images(&context).map_err(|e| map_db_list_error("images", e))?;

    let mut resp = ImageListResp { images: Vec::new() };

    for image in images {
        let uuid = convert_uuid(&image.uuid)?;
        let obj = ImageBasicResp {
            uuid,
            name: image.name.clone(),
            number_of_rows: image.number_of_rows as u64,
            number_of_columns: image.number_of_columns as u64,
        };

        resp.images.push(obj);
    }

    Ok(Json(resp))
}
