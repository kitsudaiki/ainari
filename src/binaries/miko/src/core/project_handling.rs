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

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::project::get_project_resource_count;

use crate::config;
use crate::database::project_table;
use crate::database::quota_table;
use crate::database::user_project_mapping_table;

/// Lists the resources, which still exist within a project.
///
/// Miko doesn't know the resources itself, so the components, which hold resources limited by
/// the quota, are asked for the number of resources of the project. If one of them is not
/// reachable, an error is returned, so a project is never deleted without a complete check.
///
/// # Arguments
///
/// * `project_id` - The ID of the project, whose resources are listed
/// * `context` - The user context, whose token is used for the requests
///
/// # Returns
///
/// A list like `["2 virtual_machine", "1 secret"]`, which is empty, if the project contains no
/// resources anymore.
pub async fn list_resources_of_project(
    project_id: &str,
    context: &UserContext,
) -> Result<Vec<String>, ErrorResponse> {
    let endpoints = &config::CONFIG.endpoints;
    let mut resources = Vec::new();

    for endpoint in [&endpoints.hanami, &endpoints.ryokan, &endpoints.omamori] {
        let resp = get_project_resource_count(
            endpoint,
            &context.token,
            &config::INTERNAL_API_KEY,
            project_id,
            config::CONFIG.skip_tls_verification,
        )
        .await
        .map_err(map_ainari_error_to_api_response)?;

        for (resource_type, number) in resp.resource_counts {
            if number > 0 {
                resources.push(format!("{number} {resource_type}"));
            }
        }
    }

    Ok(resources)
}

/// Deletes a project together with its quota and the assignments of all of its users.
///
/// The resources of the project are not checked here, so this has to be done by the caller
/// with `list_resources_of_project` before.
///
/// # Arguments
///
/// * `project_id` - The ID of the project, which should be deleted
/// * `context` - The user context containing authentication information
///
/// # Returns
///
/// Ok(()) if the project was deleted, or an ErrorResponse if one of the deletions failed.
pub fn delete_project_completely(
    project_id: &String,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    project_table::delete_project(project_id, context)
        .map_err(|e| map_db_id_get_delete_error("project", project_id, e))?;

    quota_table::delete_quota(project_id, context)
        .map_err(|e| map_db_id_get_delete_error("quota", project_id, e))?;

    user_project_mapping_table::delete_mappings_of_project(project_id)
        .map_err(|e| map_db_id_get_delete_error("user-project-mapping", project_id, e))?;

    Ok(())
}
