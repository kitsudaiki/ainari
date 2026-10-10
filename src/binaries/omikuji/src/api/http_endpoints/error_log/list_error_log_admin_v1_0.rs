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

use actix_web::web::{Json, Query};
use apistos::api_operation;
use validator::Validate;

use crate::database::error_log_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::log_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "error_log",
    summary = "List error-messages",
    description = r###"List the error-messages of the central error-log, sorted by their creation-time with the newest first. They can be filtered by the optional query-parameters `project_id`, `user_id` and `host_name`, and the number of returned entries can be limited by `limit`, which defaults to 100. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn list_error_log_admin(
    query: Query<LogEntryListQuery>,
    context: UserContext,
) -> Result<Json<ErrorLogListResp>, ErrorResponse> {
    check_admin_context(&context)?;

    // validate incoming query
    query
        .validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    // get error-messages from db
    let error_logs = error_log_table::list_error_logs(
        query.project_id.as_deref(),
        query.user_id.as_deref(),
        query.host_name.as_deref(),
        query.limit.unwrap_or(DEFAULT_LOG_LIST_LIMIT).into(),
    )
    .map_err(|e| map_db_list_error("error_logs", e, Some(&context)))?;

    let resp = ErrorLogListResp {
        entries: error_logs.into_iter().map(ErrorLogResp::from).collect(),
    };

    Ok(Json(resp))
}
