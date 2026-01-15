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
use apistos::actix::NoContent;
use apistos::api_operation;
use uuid::Uuid;
use validator::Validate;

use crate::core::model_handler;
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
pub async fn train_model(
    body: Json<ModelTrainReq>,
    model_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    // check if model exist
    model_table::get_model(&model_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("model", &model_uuid, e))?;

    // get model-interface
    let model_handler = model_handler::CLUSTER_HANDLER
        .read()
        .expect("mutex poisoned");
    let model_interface_mutex = model_handler
        .get_model_interface(&model_uuid)
        .map_err(map_ainari_error_to_api_response)?;
    drop(model_handler);

    // run train-process in model
    let mut model_interface = model_interface_mutex.lock().expect("mutex poisoned");
    model_interface
        .train(&body.inputs, &body.outputs)
        .map_err(map_ainari_error_to_api_response)?;

    Ok(NoContent)
}
