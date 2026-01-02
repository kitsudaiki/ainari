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

use crate::database::secret_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::common_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "secret",
    summary = "Get number of secret of the user",
    description = r###"Get number of secret of the user from the database."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_secret_count(context: UserContext) -> Result<Json<Count>, ErrorResponse> {
    let number_of_secret =
        secret_table::count_secrets(&context).map_err(|e| map_db_count_error("secret-meta", e))?;

    let secret_resp = Count {
        number_of_items: number_of_secret as u64,
    };

    Ok(Json(secret_resp))
}
