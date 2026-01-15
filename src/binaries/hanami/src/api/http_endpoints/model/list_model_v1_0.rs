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

use crate::config;
use crate::database::meta_model_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::model_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::*;
use ainari_clients::proxy as proxy_clients;

#[api_operation(
    tag = "model",
    summary = "List model",
    description = r###"List basic information of all model from the database."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_model(context: UserContext) -> Result<Json<ModelListResp>, ErrorResponse> {
    // get models from db
    let models =
        meta_model_table::list_meta_models(&context).map_err(|e| map_db_list_error("hosts", e))?;

    // get endpoints from miko
    let miko_endpoint = &config::CONFIG.miko;
    let endpoints = get_endpoints(miko_endpoint, config::CONFIG.skip_tls_verification)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    // prepare response
    let mut resp = ModelListResp { models: Vec::new() };

    // fill reponse
    for model in models {
        let uuid = convert_uuid(&model.uuid)?;

        // get port of the model from torii
        let proxy_uuid = convert_uuid(&model.proxy_uuid)?;
        let proxy_resp = proxy_clients::get_proxy(
            &endpoints.torii,
            &context.token,
            &proxy_uuid,
            config::CONFIG.skip_tls_verification,
        )
        .await
        .map_err(map_ainari_error_to_api_response)?;

        // add single object to the reponse-list
        let obj = ModelBasicResp {
            uuid,
            name: model.name.clone(),
            proxy_port: proxy_resp.port,
        };

        resp.models.push(obj);
    }

    Ok(Json(resp))
}
