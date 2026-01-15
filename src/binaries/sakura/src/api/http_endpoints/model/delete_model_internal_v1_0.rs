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

use actix_web::web::Path;
use apistos::actix::NoContent;
use apistos::api_operation;
use uuid::Uuid;

use crate::core::model_handler;
use crate::database::model_table;

use ainari_api::common_functions::map_ainari_error_to_api_response;
use ainari_api::common_functions::map_db_uuid_get_delete_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "model",
    summary = "Delete model",
    description = r###"Delete a model from the database and core."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_model_internal(
    model_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    // delete model from database
    model_table::delete_model(&model_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("model", &model_uuid, e))?;

    // delete model from core
    let mut model_handle = model_handler::CLUSTER_HANDLER
        .write()
        .expect("mutex poisoned");
    model_handle
        .delete_model(&model_uuid)
        .map_err(map_ainari_error_to_api_response)?;

    Ok(NoContent)
}
