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
use apistos::actix::NoContent;
use apistos::api_operation;
use validator::Validate;

use crate::config::GRANT_PUBLIC_KEY;
use chrono::Utc;
use uuid::Uuid;

use crate::core::coordinator::{self, exclusively, internal_error, remove_member};
use crate::database::mls_grant_table::{self, MlsGrantEntry};

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::mls_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "mls_grant",
    summary = "Store membership-grant",
    description = r###"Store a membership-grant of hanami for a gateway and the group of a network.

Only a grant with a valid signature of hanami is accepted. A grant, which adds the gateway, allows
it to subscribe to the group. A grant, which removes it, drops its grant and removes it from the
group, which rotates the keys of the remaining members."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn store_mls_grant_internal(
    body: Json<MlsGrantReq>,
    _context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    store_grant(&body.grant, &GRANT_PUBLIC_KEY, coordinator::now())?;

    Ok(NoContent)
}

/// Stores a grant of hanami.
///
/// A grant, which adds a gateway, allows it to join the group of the network. A grant, which
/// removes a gateway, drops its grant and removes it from the group.
///
/// # Arguments
/// * `grant` - The signed grant
/// * `key` - Public key of hanami
/// * `now` - Current unix-time in seconds
///
/// # Returns
/// `Ok(())`, or `BadRequest` if the grant is not valid
pub(crate) fn store_grant(
    grant: &MlsGrant,
    key: &VerifyingKey,
    now: i64,
) -> Result<(), ErrorResponse> {
    let payload = grant
        .verify(key)
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid grant: {e}")))?;
    if payload.is_expired_at(now) {
        return Err(ErrorResponse::BadRequest(
            "Invalid grant: it is expired".to_string(),
        ));
    }

    exclusively(|| match payload.action {
        MlsGrantAction::Add => {
            mls_grant_table::set_grant(MlsGrantEntry {
                uuid: Uuid::new_v4(),
                vni: payload.vni,
                client_id: payload.client_id.clone(),
                signature_key: payload.signature_key.clone(),
                payload: grant.payload.clone(),
                signature: grant.signature.clone(),
                expires_at: payload.expires_at,
                created_at: Utc::now(),
            })
            .map_err(|e| internal_error("store a grant", e))?;
            // hanami refreshes the grants regularly, so this is no news
            log::debug!(
                "'{}' may join the MLS-group of tenant {}",
                payload.client_id,
                payload.vni
            );
            Ok(())
        }
        MlsGrantAction::Remove => {
            mls_grant_table::delete_grant(payload.vni, &payload.client_id)
                .map_err(|e| internal_error("delete a grant", e))?;
            log::info!(
                "'{}' must leave the MLS-group of tenant {}",
                payload.client_id,
                payload.vni
            );
            remove_member(payload.vni, &payload.client_id, now)
        }
    })
}
