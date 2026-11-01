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

use std::collections::VecDeque;

use super::jobs::Job;

/// Queue of the jobs of one worker-thread.
#[derive(Default, Debug)]
pub struct JobQueue {
    pub queue: VecDeque<Job>,
}

impl JobQueue {
    /// Adds a new job to the end of the queue.
    ///
    /// # Arguments
    ///
    /// * `job` - The job, which is processed after all jobs, which are already queued
    pub fn add(&mut self, job: Job) {
        log::debug!("added job {} to job-queue", job.uuid);
        self.queue.push_back(job);
    }

    /// Removes and returns the job at the front of the queue.
    ///
    /// # Returns
    ///
    /// The next job, or `None`, if the queue is empty.
    pub fn get(&mut self) -> Option<Job> {
        self.queue.pop_front()
    }

    /// Returns the number of jobs, which wait in the queue.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::backup::{BackupJob, BackupScope};
    use crate::core::processing::jobs::JobVariant;
    use ainari_api_structs::user_context::UserContext;
    use ainari_common::secret::Secret;
    use uuid::Uuid;

    fn new_job() -> Job {
        Job {
            uuid: Uuid::new_v4(),
            backup_uuid: Uuid::new_v4(),
            project_id: "test-project".to_string(),
            context: UserContext {
                token: String::new(),
                user_id: "test-user".to_string(),
                project_id: "test-project".to_string(),
                is_admin: false.to_string(),
                project_role: "member".to_string(),
            },
            variant: JobVariant::Backup(BackupJob {
                scope: BackupScope::Project,
                passphrase: Secret::from("passphrase"),
                stop_vms: false,
                include_images: false,
            }),
        }
    }

    #[test]
    fn test_add_and_get() {
        let mut queue = JobQueue::default();
        let (job1, job2) = (new_job(), new_job());
        let (uuid1, uuid2) = (job1.uuid, job2.uuid);

        queue.add(job1);
        queue.add(job2);
        assert_eq!(queue.len(), 2);

        assert_eq!(queue.get().unwrap().uuid, uuid1);
        assert_eq!(queue.get().unwrap().uuid, uuid2);
        assert!(queue.get().is_none());
    }
}
