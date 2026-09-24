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
use apistos::actix::NoContent;
use apistos::api_operation;
use uuid::Uuid;

use crate::core::processing::tasks::{
    CloudHypervisorVirtualMachineDeleteInfo, Task, TaskMeta, TaskVariant,
};
use crate::database::virtual_machine_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "virtual_machine",
    summary = "Delete virtual_machine",
    description = r###"Create a new task, which deletes the virtual_machine.

The task stops the virtual_machine, removes all of its files from the host and marks it as
deleted in the database afterwards. So the virtual_machine can still be read, until the task
is finished."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_virtual_machine_internal(
    virtual_machine_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    // check that the virtual_machine exists, before a task for it is created
    let virtual_machine_data =
        virtual_machine_table::get_virtual_machine(&virtual_machine_uuid, &context).map_err(
            |e| map_db_uuid_get_delete_error("virtual_machine", &virtual_machine_uuid, e),
        )?;

    let task_uuid = Uuid::new_v4();
    let task_type = TaskType::VirtualMachineDelete;

    // prepare task-info
    let task_description = format!(
        "Delete virtual machine with UUID {}",
        virtual_machine_data.uuid
    );
    let info = CloudHypervisorVirtualMachineDeleteInfo {
        vm_uuid: virtual_machine_data.uuid,
        description: task_description.clone(),
        context: context.clone(),
    };

    // create new task, which is processed by the same worker-thread as the task, which created
    // the virtual_machine, so the deletion can not overtake the creation
    let task = Task {
        uuid: task_uuid,
        resouce_uuid: *virtual_machine_uuid,
        resource_type: TaskResourceType::VirtualMachine,
        description: task_description,
        info: TaskVariant::CloudHypervisorVirtualMachineDelete(info),
        meta: TaskMeta::new(),
    };
    super::super::task::add_task(task, &task_type, &context)
        .inspect_err(|e| log::error!("Creating a delete-task failed with error: {e}"))?;

    Ok(NoContent)
}
