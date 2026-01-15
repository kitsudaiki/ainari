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

use crate::config;
use crate::database::host_table;
use crate::database::meta_model_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::host_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "host",
    summary = "Register new host",
    description = r###"Register new host."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn register_host_internal(
    body: Json<HostCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<HostResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let mut host_uuid = Uuid::new_v4();

    // check registration key
    let conf_registration_key = &config::SAKURA_REGISTRATION_KEY.clone();
    if conf_registration_key != &body.registration_key {
        return Err(ErrorResponse::Unauthorized(
            "Invalid Registration-key".to_string(),
        ));
    }

    match host_table::get_host_by_address(&body.host_address, &context) {
        Ok(host_data) => {
            host_uuid = convert_uuid(&host_data.uuid)?;
        }
        Err(_) => {
            // add new host to database if address not already exist
            host_table::add_new_host(&host_uuid, &body.name, &body.host_address, &context)
                .map_err(|e| {
                    log::error!(
                        "Failed to add host with UUID '{host_uuid}' to database with error: {e}."
                    );
                    ErrorResponse::InternalError("Internal Error".to_string())
                })?;
        }
    };

    // delete all model in the meta-table too, which are marked by the new host as deleted
    for uuid in &body.deleted_uuids.list {
        // if the deletion failed, it is most likely because the uuid is already deleted in hanami
        // because of this, we ignore the response from the database at the moment
        // TODO: handle response to filter the case that the uuid is already deleted
        let _ = meta_model_table::force_delete_meta_model(uuid);
    }

    // get new created host from database to get addtional information
    let host_data = host_table::get_host(&host_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("host", &host_uuid, e))?;

    let resp = HostResp {
        uuid: host_uuid,
        name: host_data.name,
        host_address: host_data.address,
        created_by: host_data.created_by,
        created_at: host_data.created_at,
        updated_by: host_data.updated_by,
        updated_at: host_data.updated_at,
    };

    Ok(CreatedJson(resp))
}
