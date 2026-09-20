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

use crate::database::public_key_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::public_key_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::functions::{create_ssh_key_fingerprint, is_valid_ssh_public_key};

#[api_operation(
    tag = "public_key",
    summary = "Upload new public-key",
    description = r###"Upload new ssh-public-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn upload_public_key(
    body: Json<PublicKeyUploadReq>,
    context: UserContext,
) -> Result<CreatedJson<PublicKeyResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    // only valid ssh-public-keys are accepted
    if !is_valid_ssh_public_key(&body.public_key) {
        return Err(ErrorResponse::BadRequest(
            "Invalid input: 'public_key' is not a valid ssh-public-key.".to_string(),
        ));
    }

    // the fingerprint is not provided by the user, but calculated from the public-key itself.
    // the key was already validated above, so this can not fail here.
    let fingerprint = create_ssh_key_fingerprint(&body.public_key).ok_or_else(|| {
        log::error!("Failed to create the fingerprint of an already validated ssh-public-key.");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    let public_key_uuid = Uuid::new_v4();

    // add new public-key to database
    public_key_table::add_new_public_key(
        &public_key_uuid,
        &body.name,
        &body.public_key,
        &fingerprint,
        &context,
    )
    .map_err(|e| {
        log::error!("Failed to add public-key with UUID '{public_key_uuid}' to database.: {e}");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    // get new created public-key from database to get additional information
    let public_key_entry = public_key_table::get_public_key(&public_key_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("public_key", &public_key_uuid, e))?;

    let resp = PublicKeyResp {
        uuid: public_key_entry.uuid,
        name: public_key_entry.name,
        fingerprint: public_key_entry.fingerprint,
        created_by: public_key_entry.created_by,
        created_at: public_key_entry.created_at,
        updated_by: public_key_entry.updated_by,
        updated_at: public_key_entry.updated_at,
    };

    Ok(CreatedJson(resp))
}
