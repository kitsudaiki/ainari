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

use crate::database::task_table;
use crate::database::virtual_machine_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "task",
    summary = "Get task",
    description = r###"Get information of a task of a virtual_machine from the database."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_task(
    uuids: Path<(Uuid, Uuid)>,
    context: UserContext,
) -> Result<Json<TaskResp>, ErrorResponse> {
    let (virtual_machine_uuid, task_uuid) = uuids.into_inner();

    // check if virtual_machine exist
    virtual_machine_table::get_virtual_machine(&virtual_machine_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("virtual_machine", &virtual_machine_uuid, e))?;

    let task_data = task_table::get_task(&task_uuid, &virtual_machine_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("task", &task_uuid, e))?;

    let resp = TaskResp {
        uuid: task_uuid,
        name: task_data.name,
        task_type: task_data.task_type,
        state: task_data.task_state,
        queued_at: task_data.queued_at,
        started_at: task_data.started_at,
        finished_at: task_data.finished_at,
        messages: task_data.messages,
        created_by: task_data.created_by,
        created_at: task_data.created_at,
    };

    Ok(Json(resp))
}
