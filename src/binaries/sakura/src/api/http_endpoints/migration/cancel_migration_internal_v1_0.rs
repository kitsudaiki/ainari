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
use apistos::actix::CreatedJson;
use apistos::api_operation;
use uuid::Uuid;
use validator::Validate;

use crate::core::processing::tasks::{
    CloudHypervisorVirtualMachineMigrationCancelInfo, TaskVariant,
};
use crate::database::virtual_machine_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::migration_structs::*;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "migration",
    summary = "Cancel migration",
    description = r###"Create a new task, which unfreezes a virtual_machine on this host, whose migration failed.

The virtual_machine is marked as stopped again and booted with `boot`, because it was running
before the migration. A virtual_machine, which is not exported, is left as it is."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn cancel_migration_internal(
    virtual_machine_uuid: Path<Uuid>,
    body: Json<MigrationCancelReq>,
    context: UserContext,
) -> Result<CreatedJson<TaskResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    // check that the virtual_machine exists, before a task for it is created
    virtual_machine_table::get_virtual_machine(&virtual_machine_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("virtual_machine", &virtual_machine_uuid, e))?;

    let description =
        format!("Cancel migration of virtual machine with UUID {virtual_machine_uuid}");
    let info = CloudHypervisorVirtualMachineMigrationCancelInfo {
        vm_uuid: *virtual_machine_uuid,
        boot: body.boot,
        description: description.clone(),
        context: context.clone(),
    };

    let resp = super::add_migration_task(
        &virtual_machine_uuid,
        TaskType::MigrationCancel,
        description,
        TaskVariant::CloudHypervisorVirtualMachineMigrationCancel(info),
        &context,
    )?;

    Ok(CreatedJson(resp))
}
