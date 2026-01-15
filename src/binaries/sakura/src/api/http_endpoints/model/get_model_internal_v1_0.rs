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

use crate::database::model_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::model_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "model",
    summary = "Get model",
    description = r###"Get information of a model from the database."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_model_internal(
    model_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<ModelResp>, ErrorResponse> {
    let model_data = model_table::get_model(&model_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("model", &model_uuid, e))?;

    // deserialize name-lists
    let inputs: Vec<String> = serde_json::from_str(&model_data.inputs).map_err(|e| {
        log::error!("Failed to deserialize inputs: '{e}'");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;
    let outputs: Vec<String> = serde_json::from_str(&model_data.outputs).map_err(|e| {
        log::error!("Failed to deserialize outputs: '{e}'");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    let resp = ModelResp {
        uuid: *model_uuid,
        name: model_data.name,
        inputs,
        outputs,
        template: model_data.template,
        torii_port: 0,
        created_by: model_data.created_by,
        created_at: model_data.created_at,
        updated_by: model_data.updated_by,
        updated_at: model_data.updated_at,
    };

    Ok(Json(resp))
}
