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

use crate::database::model_table;

use ainari_api::common_functions::convert_uuid;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::model_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "model",
    summary = "List model",
    description = r###"List basic information of all model from the database."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_model_internal(
    context: UserContext,
) -> Result<Json<ModelListResp>, ErrorResponse> {
    let models = match model_table::list_models(&context) {
        Ok(models) => models,
        Err(e) => {
            log::error!("Failed to get list of models form database: '{e}'");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    let mut resp = ModelListResp { models: Vec::new() };

    for model in models {
        let uuid = convert_uuid(&model.uuid)?;
        let obj = ModelBasicResp {
            uuid,
            name: model.name,
            proxy_port: 0,
        };

        resp.models.push(obj);
    }

    Ok(Json(resp))
}
