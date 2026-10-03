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

use crate::database::mls_message_table;

use ainari_api::common_functions::map_db_uuid_get_delete_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::mls_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "mls_message",
    summary = "Acknowledge MLS-message",
    description = r###"Acknowledge a MLS-message, after the gateway has processed it, which removes it from
its pending messages."###,
    error_code = 401,
    error_code = 403,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_mls_message_internal(
    path: Path<MlsMessagePath>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    mls_message_table::delete_mls_message(&path.client_id, &path.message_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("mls-message", &path.message_uuid, e))?;

    Ok(NoContent)
}
