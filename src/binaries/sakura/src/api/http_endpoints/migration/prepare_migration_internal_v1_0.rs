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
    summary = "Prepare migration",
    description = r###"Create a new task, which prepares a virtual_machine on this host for its migration to another host.

A running virtual_machine is shut down gracefully, so it writes all its data to its disk.
Afterwards it is frozen in the state `MIGRATING`, so it can not be started or changed anymore,
while the target host pulls its files. Only a running or stopped virtual_machine can be prepared."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn prepare_migration_internal(
    virtual_machine_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<CreatedJson<TaskResp>, ErrorResponse> {
    // check that the virtual_machine exists, before a task for it is created
    virtual_machine_table::get_virtual_machine(&virtual_machine_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("virtual_machine", &virtual_machine_uuid, e))?;

    let description =
        format!("Prepare migration of virtual machine with UUID {virtual_machine_uuid}");
    let info = CloudHypervisorVirtualMachineMigrationInfo {
        vm_uuid: *virtual_machine_uuid,
        description: description.clone(),
        context: context.clone(),
    };

    let resp = super::add_migration_task(
        &virtual_machine_uuid,
        TaskType::MigrationPrepare,
        description,
        TaskVariant::CloudHypervisorVirtualMachineMigrationPrepare(info),
        &context,
    )?;

    Ok(CreatedJson(resp))
}
