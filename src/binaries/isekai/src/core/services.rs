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

use std::time::Duration;
use uuid::Uuid;

use ainari_api_structs::task_structs::{TaskResp, TaskState};
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::get_endpoints;
use ainari_clients::project::{lock_project, unlock_project};
use ainari_clients::task::get_task;
use ainari_common::config::{Endpoint, Endpoints};
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::config;

/// Time between two requests of the state of a task of sakura
const TASK_POLL_INTERVAL: Duration = Duration::from_secs(2);
/// Maximum time, which a task of sakura may take, like the saving of a large root-disk
const TASK_TIMEOUT: Duration = Duration::from_secs(12 * 60 * 60);

/// Connection to the other components, which isekai orchestrates.
///
/// All calls are done with the token of the user, who has started the backup or restore, and
/// with the internal API-key, so they are not blocked by the lock of the project.
pub struct Services {
    /// Endpoints of the components, like miko provides them
    pub endpoints: Endpoints,
    /// Endpoint of miko with its internal address
    pub miko: Endpoint,
    /// Token of the user
    pub token: String,
}

impl Services {
    /// Requests the endpoints of the components from miko.
    ///
    /// # Arguments
    ///
    /// * `context` - Context of the user, whose token is used for all calls
    ///
    /// # Returns
    ///
    /// The connection to the components.
    pub async fn connect(context: &UserContext) -> Result<Self, AinariError> {
        let endpoints = get_endpoints(
            &config::CONFIG.miko.public(),
            config::CONFIG.skip_tls_verification,
        )
        .await?;
        Ok(Services {
            endpoints,
            miko: config::CONFIG.miko.endpoint(),
            token: context.token.clone(),
        })
    }

    /// Internal API-key of the deployment.
    pub fn key(&self) -> &Secret {
        &config::INTERNAL_API_KEY
    }

    /// True, if the certificates of the other components are not checked.
    pub fn insecure(&self) -> bool {
        config::CONFIG.skip_tls_verification
    }

    /// Locks a project for a job, so its users can only read it.
    ///
    /// # Arguments
    ///
    /// * `project_id` - The project
    /// * `job_uuid` - The job, which is the only one, which can remove the lock again
    /// * `reason` - Description of the job for the users
    ///
    /// # Returns
    ///
    /// Ok, if the project was locked, `Conflict`, if it is already locked.
    pub async fn lock(
        &self,
        project_id: &str,
        job_uuid: &Uuid,
        reason: &str,
    ) -> Result<(), AinariError> {
        lock_project(
            &self.miko,
            &self.token,
            self.key(),
            project_id,
            job_uuid,
            reason,
            self.insecure(),
        )
        .await
        .map(|_| ())
    }

    /// Removes the lock of a project, which was set by a job.
    ///
    /// # Arguments
    ///
    /// * `project_id` - The project
    /// * `job_uuid` - The job, which has locked the project
    ///
    /// # Returns
    ///
    /// Ok, if the project is unlocked.
    pub async fn unlock(&self, project_id: &str, job_uuid: &Uuid) -> Result<(), AinariError> {
        unlock_project(
            &self.miko,
            &self.token,
            self.key(),
            project_id,
            job_uuid,
            self.insecure(),
        )
        .await
        .map(|_| ())
    }

    /// Waits, until a task of sakura has ended.
    ///
    /// # Arguments
    ///
    /// * `sakura_address` - Internal address of the sakura-host, which processes the task
    /// * `task` - The task
    ///
    /// # Returns
    ///
    /// Ok, if the task was finished successfully, or an error with the messages of the task.
    pub async fn wait_for_task(
        &self,
        sakura_address: &str,
        task: &TaskResp,
    ) -> Result<(), AinariError> {
        let start = std::time::Instant::now();
        loop {
            let current =
                get_task(sakura_address, &self.token, &task.uuid, self.insecure()).await?;
            match current.state {
                TaskState::Finished => return Ok(()),
                TaskState::Error | TaskState::Aborted => {
                    return Err(AinariError::internal(format!(
                        "Task '{}' ({}) failed: {}",
                        task.uuid,
                        task.description,
                        current.messages.join("; ")
                    )));
                }
                TaskState::Created | TaskState::Queued | TaskState::Active => {}
            }
            if start.elapsed() > TASK_TIMEOUT {
                return Err(AinariError::internal(format!(
                    "Task '{}' ({}) didn't end in time",
                    task.uuid, task.description
                )));
            }
            tokio::time::sleep(TASK_POLL_INTERVAL).await;
        }
    }
}
