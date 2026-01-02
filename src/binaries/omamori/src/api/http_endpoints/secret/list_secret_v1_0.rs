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
use ainari_api_structs::secret_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "secret",
    summary = "List secret",
    description = r###"List basic information of all secret from the database."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_secret(context: UserContext) -> Result<Json<SecretListResp>, ErrorResponse> {
    let secrets =
        secret_table::list_secrets(&context).map_err(|e| map_db_list_error("secrets", e))?;

    let mut resp = SecretListResp {
        secrets: Vec::new(),
    };

    for secret in secrets {
        let uuid = convert_uuid(&secret.uuid)?;
        let obj = SecretBasicResp {
            uuid,
            name: secret.name.clone(),
        };

        resp.secrets.push(obj);
    }

    Ok(Json(resp))
}
