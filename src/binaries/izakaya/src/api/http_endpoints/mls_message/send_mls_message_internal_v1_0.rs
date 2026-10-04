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
use validator::Validate;

use crate::database::mls_message_table;

use ainari_api::common_functions::map_db_write_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::mls_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "mls_message",
    summary = "Send MLS-message",
    description = r###"Hand a MLS-message over for delivery to its recipients.

The message is stored once for every recipient, so each of them can fetch and acknowledge it on
its own. The izakaya doesn't read the message: a welcome is encrypted for the invited gateway and
a commit is signed by the member of the group, which created it, so the recipients verify it
themselves."###,
    error_code = 400,
    error_code = 401,
    error_code = 403,
    error_code = 500
)]
pub async fn send_mls_message_internal(
    body: Json<MlsMessageReq>,
    context: UserContext,
) -> Result<CreatedJson<MlsMessageCreateResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;
    if body.recipients.iter().any(|recipient| recipient.is_empty()) {
        return Err(ErrorResponse::BadRequest(
            "Invalid input: recipients must not be empty".to_string(),
        ));
    }

    let uuids = mls_message_table::add_mls_message(&body, &context).map_err(|e| {
        map_db_write_error(
            &format!("add {} of group '{}'", body.message_type, body.group_id),
            e,
        )
    })?;

    log::debug!(
        "Stored {} of group '{}' for epoch {} from '{}' for {} recipient(s)",
        body.message_type,
        body.group_id,
        body.epoch,
        body.sender,
        uuids.len()
    );

    Ok(CreatedJson(MlsMessageCreateResp { uuids }))
}
