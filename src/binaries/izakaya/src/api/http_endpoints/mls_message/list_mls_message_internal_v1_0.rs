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

use actix_web::web::{Json, Path};
use apistos::api_operation;

use crate::core::coordinator::record_contact;
use crate::database::mls_message_table;

use ainari_api::common_functions::map_db_list_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::mls_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "mls_message",
    summary = "List MLS-messages",
    description = r###"List the MLS-messages, which wait for a gateway.

The messages are ordered by their group and their epoch, which is the order, in which the gateway
has to process them. They stay until the gateway acknowledges them by deleting them."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_mls_message_internal(
    path: Path<ClientPath>,
    _context: UserContext,
) -> Result<Json<MlsMessageListResp>, ErrorResponse> {
    // the gateways poll their messages all the time, which tells, that they are alive
    record_contact(&path.client_id);

    let entries = mls_message_table::list_mls_messages(&path.client_id)
        .map_err(|e| map_db_list_error("mls-message", e))?;

    let mut resp = MlsMessageListResp::default();
    for entry in entries {
        // a message-type, which can not be read, could only come from a broken database
        let message_type = match entry.message_type.parse::<MlsMessageType>() {
            Ok(message_type) => message_type,
            Err(e) => {
                log::error!("Skip mls-message '{}': {e}", entry.uuid);
                continue;
            }
        };

        let grants = entry.grants();
        resp.messages.push(MlsMessageResp {
            grants,
            uuid: entry.uuid,
            group_id: entry.group_id,
            epoch: entry.epoch.max(0) as u64,
            message_type,
            sender: entry.sender,
            recipient: entry.recipient,
            payload: entry.payload,
            created_at: entry.created_at,
        });
    }

    Ok(Json(resp))
}
