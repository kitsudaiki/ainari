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

use crate::database::host_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::host_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "host",
    summary = "Get host",
    description = r###"Get information of a host from the database."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_host_admin(
    host_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<HostResp>, ErrorResponse> {
    check_admin_context(&context)?;

    let host_data = host_table::get_host(&host_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("onsen-host", &host_uuid, e))?;

    let resp = HostResp {
        uuid: *host_uuid,
        name: host_data.name,
        host_address: host_data.address,
        created_by: host_data.created_by,
        created_at: host_data.created_at,
        updated_by: host_data.updated_by,
        updated_at: host_data.updated_at,
    };

    Ok(Json(resp))
}
