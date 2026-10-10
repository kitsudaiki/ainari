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
use apistos::api_operation;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

use crate::core::mls_key_exchange::state::MLS_STATE_HANDLE;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_crypto_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "network_crypto",
    summary = "Get MLS-identity",
    description = r###"Get the identity of this gateway within the MLS-groups of the networks.

The identity is the underlay-address of the gateway together with its public MLS signature-key.
hanami pins the key, when it places the first VM on the host of the gateway, and names it in every
membership-grant of the gateway, so no other gateway can join a group in its name. The identity is
created with the first call."###,
    error_code = 401,
    error_code = 500
)]
pub async fn get_mls_identity_internal(
    _context: UserContext,
) -> Result<Json<MlsIdentityResp>, ErrorResponse> {
    let mut mls = MLS_STATE_HANDLE.lock().await;
    let identity = mls.ensure_identity()?;

    let resp = MlsIdentityResp {
        client_id: identity.client_id.clone(),
        signature_key: BASE64.encode(identity.signer.public()),
    };
    // a new identity has to survive a restart, before hanami pins it
    mls.save()?;

    Ok(Json(resp))
}
