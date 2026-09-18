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
use actix_web::web::Path;
use apistos::api_operation;
use uuid::Uuid;

use crate::database::public_key_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::public_key_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "public_key",
    summary = "Get public-key",
    description = r###"Get information of a public-key from the database."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_public_key(
    public_key_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<PublicKeyResp>, ErrorResponse> {
    let public_key_data = public_key_table::get_public_key(&public_key_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("public_key", &public_key_uuid, e))?;

    let resp = PublicKeyResp {
        uuid: public_key_data.uuid,
        name: public_key_data.name,
        fingerprint: public_key_data.fingerprint,
        created_by: public_key_data.created_by,
        created_at: public_key_data.created_at,
        updated_by: public_key_data.updated_by,
        updated_at: public_key_data.updated_at,
    };

    Ok(Json(resp))
}
