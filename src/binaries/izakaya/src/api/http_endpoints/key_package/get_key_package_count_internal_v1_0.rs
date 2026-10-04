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

use crate::database::key_package_table;

use ainari_api::common_functions::map_db_count_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::mls_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "key_package",
    summary = "Count key-packages",
    description = r###"Count the MLS key-packages of a gateway, which can still be claimed."###,
    error_code = 401,
    error_code = 500
)]
pub async fn get_key_package_count_internal(
    path: Path<ClientPath>,
    _context: UserContext,
) -> Result<Json<KeyPackageCountResp>, ErrorResponse> {
    let count = key_package_table::count_key_packages(&path.client_id)
        .map_err(|e| map_db_count_error("key-package", e))?;

    Ok(Json(KeyPackageCountResp {
        client_id: path.client_id.clone(),
        count: count.max(0) as u64,
    }))
}
