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

use actix_web::web::Path;
use apistos::actix::CreatedJson;
use apistos::api_operation;
use uuid::Uuid;

use crate::core::processing::tasks::{CloudHypervisorVirtualMachineMigrationInfo, TaskVariant};
use crate::database::virtual_machine_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "migration",
    summary = "Remove migrated virtual_machine",
    description = r###"Create a new task, which removes the copy of a migrated virtual_machine from this host.

This is the source host after a successful migration, or the target host after a failed one.
Only a virtual_machine, which is exported or whose import failed, is removed. Unlike a normal
deletion, no deleted entry is kept, because the virtual_machine still exists on the other host."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn remove_migrated_virtual_machine_internal(
    virtual_machine_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<CreatedJson<TaskResp>, ErrorResponse> {
    // check that the virtual_machine exists, before a task for it is created
    virtual_machine_table::get_virtual_machine(&virtual_machine_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("virtual_machine", &virtual_machine_uuid, e))?;

    let description = format!("Remove migrated virtual machine with UUID {virtual_machine_uuid}");
    let info = CloudHypervisorVirtualMachineMigrationInfo {
        vm_uuid: *virtual_machine_uuid,
        description: description.clone(),
        context: context.clone(),
    };

    let resp = super::add_migration_task(
        &virtual_machine_uuid,
        TaskType::MigrationRemove,
        description,
        TaskVariant::CloudHypervisorVirtualMachineMigrationRemove(info),
        &context,
    )?;

    Ok(CreatedJson(resp))
}
