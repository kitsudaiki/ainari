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

use crate::database::audit_log_table;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::log_structs::*;

#[api_operation(
    tag = "audit_log",
    summary = "Add audit-log entry (internal)",
    description = r###"Add a new entry of another component to the central audit-log."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn add_audit_log_internal(
    body: Json<AuditLogAddInternalReq>,
) -> Result<CreatedJson<AuditLogResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let audit_log_uuid = Uuid::new_v4();

    // add new audit-log to database
    audit_log_table::add_new_audit_log(
        &audit_log_uuid,
        &body.request_type,
        &body.endpoint,
        &body.component_name,
        &body.host_name,
        &body.user_id,
        &body.project_id,
    )
    .map_err(|e| {
        // only logged locally and not reported to omikuji, which is this service itself, because
        // else each failed entry would add another entry
        log::error!("Failed to add audit-log with UUID '{audit_log_uuid}' to database: {e}");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    // get new created audit-log from database to get additional information
    let audit_log_entry = audit_log_table::get_audit_log(&audit_log_uuid).map_err(|_| {
        log::error!("Failed to get audit-log with UUID '{audit_log_uuid}' after adding it");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    Ok(CreatedJson(audit_log_entry.into()))
}
