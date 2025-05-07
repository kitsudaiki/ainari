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
use actix_web::web::Path;
use apistos::api_operation;
use uuid::Uuid;
use std::str::FromStr;

use crate::api::errors::ErrorResponse;
use crate::api::user_context::UserContext;
use crate::database::task_table;
use crate::database::cluster_table;

use hanami_common::enums;

use super::task_structs::{TaskResp, TaskType};

#[api_operation(
    tag = "task",
    summary = "Get task",
    description = r###"Get information of a task of a cluster from the database."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_task(cluster_uuid: Path<Uuid>, task_uuid: Path<Uuid>, context: UserContext) -> Result<Json<TaskResp>, ErrorResponse> {
    // check if cluster exist
    match cluster_table::get_cluster(&cluster_uuid, &context) {
        Ok(_) => {},
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        },
        Err(enums::DbError::NotFound) => {
            let msg = format!("Cluster with UUID '{cluster_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    let task_data = match task_table::get_task(&task_uuid, &cluster_uuid, &context) {
        Ok(task_data) => task_data,
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        },
        Err(enums::DbError::NotFound) => {
            let msg = format!("Task with UUID '{task_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    let task_type = match TaskType::from_str(task_data.task_type.as_str()) {
        Ok(task_type) => task_type,
        Err(()) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    let resp = TaskResp {
        uuid: task_uuid.clone(),
        name: task_data.name.clone(),
        task_type: task_type,
        created_by: task_data.created_by.clone(),
        created_at: task_data.created_at.clone(),
        updated_by: task_data.updated_by.clone(),
        updated_at: task_data.updated_at.clone(),
    };

    return Ok(Json(resp));
}