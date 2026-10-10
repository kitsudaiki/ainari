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

use std::collections::BTreeMap;

use actix_web::web::Json;
use actix_web::web::Path;
use apistos::api_operation;

use crate::database::floating_ip_table;
use crate::database::meta_virtual_machine_table;
use crate::database::network_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::project_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "project",
    summary = "Count resources of project (internal)",
    description = r###"Count the virtual machines, networks and floating IP-addresses of a project, which still exist. It is used by miko to check, if a project is empty, before it is deleted. This can only be done by an admin."###,
    error_code = 401,
    error_code = 500
)]
pub async fn get_project_resource_count_internal(
    project_id: Path<String>,
    context: UserContext,
) -> Result<Json<ProjectResourceCountInternalResp>, ErrorResponse> {
    check_admin_context(&context)?;

    let mut resource_counts = BTreeMap::new();

    // count virtual machines
    let number = meta_virtual_machine_table::count_meta_virtual_machines_of_project(&project_id)
        .map_err(|e| {
            log::error!(
                "Failed to count the virtual_machines of project '{project_id}' in database: {e}"
            );
            ErrorResponse::InternalError("Internal Error".to_string())
        })?;
    resource_counts.insert("virtual_machine".to_string(), number);

    // count networks
    let number = network_table::count_networks_of_project(&project_id).map_err(|e| {
        log::error!("Failed to count the networks of project '{project_id}' in database: {e}");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;
    resource_counts.insert("network".to_string(), number);

    // count flowing-ips
    let number = floating_ip_table::count_floating_ips_of_project(&project_id).map_err(|e| {
        log::error!("Failed to count the floating_ips of project '{project_id}' in database: {e}");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;
    resource_counts.insert("floating_ip".to_string(), number);

    Ok(Json(ProjectResourceCountInternalResp { resource_counts }))
}
