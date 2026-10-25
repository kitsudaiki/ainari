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

use actix_web::web::{Json, Query};
use apistos::api_operation;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::backup_structs::*;
use ainari_api_structs::user_context::UserContext;

use crate::database::id_map_table;
use crate::database::job_table::{self, JobEntry};

#[api_operation(
    tag = "job",
    summary = "List jobs",
    description = r###"List the backup- and restore-jobs of the project of the user, the newest first.

The query-parameter `backup_uuid` lists only the jobs of one backup. Admins see the jobs of all
projects."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_job(
    query: Query<JobListQuery>,
    context: UserContext,
) -> Result<Json<JobListResp>, ErrorResponse> {
    let jobs = job_table::list_jobs(&context, query.backup_uuid.as_ref())
        .map_err(|e| map_db_list_error("jobs", e, Some(&context)))?
        .into_iter()
        .map(|entry| to_job_resp(entry, &context))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(JobListResp { jobs }))
}

/// Converts a job of the database into its response, together with the UUIDs, which were
/// changed by the job.
///
/// # Arguments
///
/// * `entry` - The job
/// * `context` - User-context of the request
///
/// # Returns
///
/// The response, or an internal error, if the database contains invalid values.
fn to_job_resp(entry: JobEntry, context: &UserContext) -> Result<JobResp, ErrorResponse> {
    let job_type = entry
        .job_type()
        .map_err(|e| map_internal_error("read type of job", e, Some(context)))?;
    let state = entry
        .state()
        .map_err(|e| map_internal_error("read state of job", e, Some(context)))?;
    let id_map = id_map_table::list_id_map_of_job(&entry.uuid)
        .map_err(|e| map_db_list_error("id-map", e, Some(context)))?
        .into_iter()
        .map(|mapping| IdMapEntryResp {
            resource_type: mapping.resource_type,
            old_uuid: mapping.old_uuid,
            new_uuid: mapping.new_uuid,
        })
        .collect();

    Ok(JobResp {
        uuid: entry.uuid,
        job_type,
        backup_uuid: entry.backup_uuid,
        project_id: entry.project_id,
        state,
        step: entry.step,
        messages: entry.messages,
        id_map,
        created_at: entry.created_at,
        created_by: entry.created_by,
        started_at: entry.started_at,
        finished_at: entry.finished_at,
    })
}
