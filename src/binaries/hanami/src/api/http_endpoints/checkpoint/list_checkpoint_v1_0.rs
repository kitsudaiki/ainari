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
use apistos::api_operation;
use uuid::Uuid;

use crate::api::errors::ErrorResponse;
use crate::api::user_context::UserContext;
use crate::database::checkpoint_table;

use hanami_structs::checkpoint_structs::{CheckpointBasicResp, CheckpointListResp};

#[api_operation(
    tag = "checkpoint",
    summary = "List checkpoint",
    description = r###"List basic information of all checkpoint from the database."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_checkpoint(context: UserContext) -> Result<Json<CheckpointListResp>, ErrorResponse> {
    let checkpoints = match checkpoint_table::list_checkpoints(&context)
    {
        Ok(checkpoints) => checkpoints,
        Err(e) => {
            log::error!("Failed to get list of checkpoints form database: '{:?}'", e);
            return Err(ErrorResponse::InternalError("".to_string()))
        }
    };

    let mut resp = CheckpointListResp {
        checkpoints: Vec::new(),
    };

    for checkpoint in checkpoints {
        match Uuid::parse_str(&checkpoint.uuid) {
            Ok(uuid) => {
                let obj = CheckpointBasicResp {
                    uuid: uuid,
                    name: checkpoint.name.clone(),
                };
        
                resp.checkpoints.push(obj);
            },
            Err(e) => {
                log::error!("Error while listing checkpoint: '{:?}'", e);
                return Err(ErrorResponse::InternalError("".to_string()));
            }
        }
    }

    Ok(Json(resp))
}
