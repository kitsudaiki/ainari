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
use uuid::Uuid;
use validator::Validate;

use crate::core::model_handler::MODEL_HANDLER;
use crate::database::model_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::model_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_model_parser::model_parser::parse_model_template;

#[api_operation(
    tag = "model",
    summary = "Create new model",
    description = r###"Create new model based on a model-template."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn create_model_internal(
    body: Json<ModelCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<ModelResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let model_uuid = Uuid::new_v4();

    // parse model-template
    let mut parsed_model = match parse_model_template(&body.name, &body.template) {
        Ok(parsed) => parsed,
        Err(e) => {
            let msg = format!("Failed to parse model-template: {e:?}");
            return Err(ErrorResponse::BadRequest(msg));
        }
    };
    parsed_model.uuid = model_uuid;

    // parse model-template and create model from it
    let mut model_handler = MODEL_HANDLER.write().expect("mutex poisoned");
    model_handler
        .init_new_model(&model_uuid, &parsed_model)
        .map_err(map_ainari_error_to_api_response)?;

    // filter input-names
    let mut inputs: Vec<String> = Vec::new();
    for input in parsed_model.inputs {
        inputs.push(input.name);
    }

    // filter output-names
    let mut outputs: Vec<String> = Vec::new();
    for output in parsed_model.outputs {
        outputs.push(output.name);
    }

    // add new model to database
    match model_table::add_new_model(
        &model_uuid,
        &body.name,
        &body.template,
        &inputs,
        &outputs,
        &context,
    ) {
        Ok(_) => {}
        Err(e) => {
            let msg =
                format!("Failed to add model with UUID '{model_uuid}' to database with error: {e}");
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    // get new created model from database to get addtional information
    let model_data = model_table::get_model(&model_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("model", &model_uuid, e))?;

    let resp = ModelResp {
        uuid: model_uuid,
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

    Ok(CreatedJson(resp))
}
