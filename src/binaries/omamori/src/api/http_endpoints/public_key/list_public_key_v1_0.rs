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

use crate::database::public_key_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::public_key_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "public_key",
    summary = "List public-keys",
    description = r###"List basic information of all public-keys from the database."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_public_key(
    context: UserContext,
) -> Result<Json<PublicKeyListResp>, ErrorResponse> {
    // get public-keys from db
    let public_keys = public_key_table::list_public_keys(&context)
        .map_err(|e| map_db_list_error("public_keys", e))?;

    // prepare response
    let mut resp = PublicKeyListResp {
        public_keys: Vec::new(),
    };

    // fill reponse
    for public_key in public_keys {
        // add single object to the reponse-list
        let obj = PublicKeyBasicResp {
            uuid: public_key.uuid,
            name: public_key.name,
            fingerprint: public_key.fingerprint,
        };

        resp.public_keys.push(obj);
    }

    Ok(Json(resp))
}
