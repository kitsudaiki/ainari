// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

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
    match body.validate() {
        Ok(_) => (),
        Err(e) => {
            let msg = format!("Invalid input: {e}");
            return Err(ErrorResponse::BadRequest(msg));
        }
    };

    let host_uuid = Uuid::new_v4();

    // check registration key
    let conf_registration_key = &config::CONFIG.sakura.registation_key;
    if conf_registration_key != &body.registration_key {
        return Err(ErrorResponse::Unauthorized(
            "Invalid Registration-key".to_string(),
        ));
    }

    // add new host to database
    match host_table::add_new_host(&host_uuid, &body.name, &body.host_address, &context) {
        Ok(_) => {}
        Err(_) => {
            let msg = format!("Failed to add host with UUID '{host_uuid}' to database.");
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    // get new created host from database to get addtional information
    let host_data: host_table::HostEntry = match host_table::get_host(&host_uuid, &context) {
        Ok(host_data) => host_data,
        Err(_) => {
            let msg = format!(
                "Failed to get host with ID '{host_uuid}' from database, even the host should exist."
            );
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    let resp = HostResp {
        uuid: host_uuid,
        name: host_data.name.clone(),
        host_address: host_data.address.clone(),
        created_by: host_data.created_by.clone(),
        created_at: host_data.created_at.clone(),
        updated_by: host_data.updated_by.clone(),
        updated_at: host_data.updated_at.clone(),
    };

    return Ok(CreatedJson(resp));
}
