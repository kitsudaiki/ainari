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

use crate::database::error_log_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::log_structs::*;

#[api_operation(
    tag = "error_log",
    summary = "Add error-message (internal)",
    description = r###"Add a new error-message of another component to the central error-log."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn add_error_log_internal(
    body: Json<ErrorLogAddInternalReq>,
) -> Result<CreatedJson<ErrorLogResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let error_log_uuid = Uuid::new_v4();

    // add new error-message to database
    error_log_table::add_new_error_log(
        &error_log_uuid,
        &body.message,
        &body.component_name,
        &body.host_name,
        &body.user_id,
        &body.project_id,
    )
    .map_err(|e| {
        map_db_write_error(
            &format!("add error-message with UUID '{error_log_uuid}' to database"),
            e,
        )
    })?;

    // get new created error-message from database to get additional information
    let error_log_entry = error_log_table::get_error_log(&error_log_uuid)
        .map_err(|e| map_db_uuid_get_after_add_error("error_log", &error_log_uuid, e))?;

    Ok(CreatedJson(error_log_entry.into()))
}
