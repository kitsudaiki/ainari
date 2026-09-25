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

use crate::core::processing::tasks::TaskVariant;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "task",
    summary = "Create new reboot-task",
    description = r###"Create a new task, which reboots the virtual_machine.

Only a running virtual_machine can be rebooted. A stopped virtual_machine has to be started
instead."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn reboot_virtual_machine(
    virtual_machine_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<CreatedJson<TaskResp>, ErrorResponse> {
    let resp = super::add_power_task(
        &virtual_machine_uuid,
        TaskType::VirtualMachineReboot,
        "Reboot",
        TaskVariant::CloudHypervisorVirtualMachineReboot,
        &context,
    )?;

    Ok(CreatedJson(resp))
}
