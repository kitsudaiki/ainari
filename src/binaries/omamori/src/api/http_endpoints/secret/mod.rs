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

pub mod create_generated_secret_v1_0;
pub mod create_secret_v1_0;
pub mod delete_secret_v1_0;
pub mod get_secret_payload_v1_0;
pub mod get_secret_v1_0;
pub mod list_secret_v1_0;

use crate::config;
use crate::database::secret_table;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::quota::get_quota;
use ainari_common::error::AinariError;

async fn check_quota(context: &UserContext) -> Result<(), ErrorResponse> {
    // get number of secrets of the user
    let current_number_of_secrets = match secret_table::count_secrets(context) {
        Ok(number) => number,
        Err(e) => {
            log::error!("Failed to count secrets in database.: {e}");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    // check the maximum number of secrets defined in miko
    let miko_endpoint = &config::CONFIG.miko;
    let max_number_of_secrets = match get_quota(
        miko_endpoint,
        &context.token,
        &context.user_id,
        config::CONFIG.insecure_clients,
    )
    .await
    {
        Ok(body) => body.max_secret as i64,
        Err(AinariError::Unauthorized(msg)) => {
            return Err(ErrorResponse::Unauthorized(msg));
        }
        Err(AinariError::InvalidInput(msg)) => {
            return Err(ErrorResponse::BadRequest(msg));
        }
        Err(AinariError::Error(msg)) => {
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    // check if quota is already exceeded
    if current_number_of_secrets as i64 >= max_number_of_secrets {
        return Err(ErrorResponse::Conflict(
            "Maximum number of secrets exceeded.".to_string(),
        ));
    }

    Ok(())
}
