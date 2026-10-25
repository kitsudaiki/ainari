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

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::database::db_handle;

use ainari_api_structs::backup_structs::{JobState, JobType};
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;
use ainari_common::objects::*;

table! {
    jobs (uuid) {
        uuid -> Varchar,
        job_type -> Varchar,
        backup_uuid -> Varchar,
        project_id -> Varchar,
        job_state -> Varchar,
        step -> Varchar,
        messages -> Text,
        options -> Text,
        owner_id -> Varchar,
        created_at -> Varchar,
        created_by -> Varchar,
        started_at -> Nullable<Varchar>,
        finished_at -> Nullable<Varchar>,
    }
}

/// Represents an entry in the jobs table.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = jobs)]
pub struct JobEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    /// `BACKUP` or `RESTORE`
    pub job_type: String,
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub backup_uuid: Uuid,
    /// project, which is saved or restored by the job
    pub project_id: String,
    /// `QUEUED`, `ACTIVE`, `FINISHED` or `ERROR`
    pub job_state: String,
    /// step, which is processed right now, or the last step of a finished job
    pub step: String,
    #[diesel(serialize_as = DbVecString, deserialize_as = DbVecString)]
    pub messages: Vec<String>,
    /// options of the job as json, without the passphrase
    pub options: String,
    pub owner_id: String,
    #[diesel(serialize_as = DbDateTime, deserialize_as = DbDateTime)]
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    #[diesel(serialize_as = DbOptDateTime, deserialize_as = DbOptDateTime)]
    pub started_at: Option<DateTime<Utc>>,
    #[diesel(serialize_as = DbOptDateTime, deserialize_as = DbOptDateTime)]
    pub finished_at: Option<DateTime<Utc>>,
}

impl JobEntry {
    /// Type of the job.
    ///
    /// # Returns
    ///
    /// The type, or an error, if the database contains an invalid value.
    pub fn job_type(&self) -> Result<JobType, String> {
        self.job_type.parse()
    }

    /// State of the job.
    ///
    /// # Returns
    ///
    /// The state, or an error, if the database contains an invalid value.
    pub fn state(&self) -> Result<JobState, String> {
        self.job_state.parse()
    }
}

/// Adds a new job in the state `QUEUED` to the database.
///
/// # Arguments
///
/// * `job_uuid` - UUID of the job, which also locks the project
/// * `job_type` - Type of the job
/// * `backup_uuid` - Backup, which is created or restored by the job
/// * `project_id` - Project, which is saved or restored by the job
/// * `options` - Options of the job as json, which must not contain the passphrase
/// * `context` - The user context of the user, who has started the job
///
/// # Returns
///
/// A QueryResult indicating the number of rows affected
pub fn add_new_job(
    job_uuid: &Uuid,
    job_type: JobType,
    backup_uuid: &Uuid,
    project_id: &str,
    options: &str,
    context: &UserContext,
) -> QueryResult<usize> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::permission_denied_error());
    }

    let entry = JobEntry {
        uuid: *job_uuid,
        job_type: job_type.to_string(),
        backup_uuid: *backup_uuid,
        project_id: project_id.to_string(),
        job_state: JobState::Queued.to_string(),
        step: "queued".to_string(),
        messages: Vec::new(),
        options: options.to_string(),
        owner_id: context.user_id.clone(),
        created_at: Utc::now(),
        created_by: context.user_id.clone(),
        started_at: None,
        finished_at: None,
    };
    add_job(entry)
}

/// Adds a job to the database.
///
/// # Arguments
///
/// * `entry` - The job to insert
///
/// # Returns
///
/// A QueryResult indicating the number of rows affected
fn add_job(entry: JobEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::jobs::dsl::*;
    diesel::insert_into(jobs).values(entry).execute(&mut *conn)
}

/// Retrieves a job from the database.
///
/// Users, which are not admin, only see the jobs of their own project.
///
/// # Arguments
///
/// * `job_uuid` - The UUID of the job
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
///
/// A Result containing the JobEntry, or a DbError if not found or an error occurs
pub fn get_job(job_uuid: &Uuid, context: &UserContext) -> Result<JobEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::jobs::dsl::*;

    let mut query = jobs.filter(uuid.eq(job_uuid.to_string())).into_boxed();
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
    }

    match query
        .select(JobEntry::as_select())
        .first::<JobEntry>(&mut *conn)
    {
        Ok(entry) => Ok(entry),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Lists the jobs, which the user has access to, the newest first.
///
/// # Arguments
///
/// * `context` - The user context containing information about the user and their permissions
/// * `filter_backup_uuid` - List only the jobs of this backup
///
/// # Returns
///
/// A QueryResult containing the jobs
pub fn list_jobs(
    context: &UserContext,
    filter_backup_uuid: Option<&Uuid>,
) -> QueryResult<Vec<JobEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::jobs::dsl::*;

    let mut query = jobs.into_boxed();
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
    }
    if let Some(filter) = filter_backup_uuid {
        query = query.filter(backup_uuid.eq(filter.to_string()));
    }

    query
        .order(created_at.desc())
        .select(JobEntry::as_select())
        .load(&mut *conn)
}

/// Lists all jobs of a backup, the newest first, independent of the user.
///
/// # Arguments
///
/// * `filter_backup_uuid` - The backup
///
/// # Returns
///
/// A QueryResult containing the jobs of the backup
pub fn list_jobs_of_backup(filter_backup_uuid: &Uuid) -> QueryResult<Vec<JobEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::jobs::dsl::*;

    jobs.filter(backup_uuid.eq(filter_backup_uuid.to_string()))
        .order(created_at.desc())
        .select(JobEntry::as_select())
        .load(&mut *conn)
}

/// Changes the state of a job and sets the time of its start or end.
///
/// # Arguments
///
/// * `job_uuid` - The UUID of the job
/// * `new_state` - The new state
///
/// # Returns
///
/// Ok, if the job was updated, `NotFound`, if it doesn't exist
pub fn update_job_state(job_uuid: &Uuid, new_state: JobState) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::jobs::dsl::*;

    let target = jobs.filter(uuid.eq(job_uuid.to_string()));
    let now = Utc::now().to_rfc3339();
    let result = match new_state {
        JobState::Queued => diesel::update(target)
            .set(job_state.eq(new_state.to_string()))
            .execute(&mut *conn),
        JobState::Active => diesel::update(target)
            .set((job_state.eq(new_state.to_string()), started_at.eq(now)))
            .execute(&mut *conn),
        JobState::Finished | JobState::Error => diesel::update(target)
            .set((job_state.eq(new_state.to_string()), finished_at.eq(now)))
            .execute(&mut *conn),
    };

    match result {
        Ok(0) => Err(enums::DbError::NotFound),
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Sets the step of a job, which is processed right now.
///
/// # Arguments
///
/// * `job_uuid` - The UUID of the job
/// * `new_step` - Description of the step
///
/// # Returns
///
/// Ok, if the job was updated, `NotFound`, if it doesn't exist
pub fn set_job_step(job_uuid: &Uuid, new_step: &str) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::jobs::dsl::*;

    match diesel::update(jobs.filter(uuid.eq(job_uuid.to_string())))
        .set(step.eq(new_step.to_string()))
        .execute(&mut *conn)
    {
        Ok(0) => Err(enums::DbError::NotFound),
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Adds a message to a job, like the reason of its failure.
///
/// # Arguments
///
/// * `job_uuid` - The UUID of the job
/// * `new_message` - The message
///
/// # Returns
///
/// Ok, if the message was added, `NotFound`, if the job doesn't exist
pub fn add_message_to_job(job_uuid: &Uuid, new_message: &str) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::jobs::dsl::*;

    let result = conn.transaction::<_, diesel::result::Error, _>(|conn| {
        let entry = jobs
            .filter(uuid.eq(job_uuid.to_string()))
            .select(JobEntry::as_select())
            .first::<JobEntry>(conn)?;
        let mut all_messages = entry.messages;
        all_messages.push(new_message.to_string());
        diesel::update(jobs.filter(uuid.eq(job_uuid.to_string())))
            .set(messages.eq(DbVecString::from(all_messages).0))
            .execute(conn)
    });

    match result {
        Ok(_) => Ok(()),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Marks all jobs, which were queued or active, as failed.
///
/// The queues of the jobs and the passphrases of the backups only exist in the memory of isekai,
/// so these jobs are never processed after a restart.
///
/// # Arguments
///
/// * `message` - Reason, which is added to the messages of the jobs
///
/// # Returns
///
/// The jobs, which were marked as failed, with their state before
pub fn fail_unfinished_jobs(message: &str) -> QueryResult<Vec<JobEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::jobs::dsl::*;

    conn.transaction::<_, diesel::result::Error, _>(|conn| {
        let unfinished = jobs
            .filter(
                job_state
                    .eq(JobState::Queued.to_string())
                    .or(job_state.eq(JobState::Active.to_string())),
            )
            .select(JobEntry::as_select())
            .load::<JobEntry>(conn)?;

        let now = Utc::now().to_rfc3339();
        for job in &unfinished {
            let mut all_messages = job.messages.clone();
            all_messages.push(message.to_string());
            diesel::update(jobs.filter(uuid.eq(job.uuid.to_string())))
                .set((
                    job_state.eq(JobState::Error.to_string()),
                    finished_at.eq(now.clone()),
                    messages.eq(DbVecString::from(all_messages).0),
                ))
                .execute(conn)?;
        }
        Ok(unfinished)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ainari_common::enums::ProjectRole;
    use serial_test::serial;

    fn hard_delete_job(job_uuid: &Uuid) {
        use self::jobs::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(jobs.filter(uuid.eq(job_uuid.to_string()))).execute(&mut *conn);
    }

    fn new_context(project_id: &str, is_admin: bool, project_role: ProjectRole) -> UserContext {
        UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: project_id.to_string(),
            is_admin: is_admin.to_string(),
            project_role: project_role.to_string(),
        }
    }

    fn expect_entry(result: Result<JobEntry, enums::DbError>) -> JobEntry {
        match result {
            Ok(entry) => entry,
            Err(_) => panic!("job was not found"),
        }
    }

    #[test]
    #[serial]
    fn test_job_lifecycle() {
        let (job_uuid, backup_uuid) = (Uuid::new_v4(), Uuid::new_v4());
        let context = new_context("test-project", false, ProjectRole::Member);
        hard_delete_job(&job_uuid);

        add_new_job(
            &job_uuid,
            JobType::Backup,
            &backup_uuid,
            "test-project",
            "{}",
            &context,
        )
        .unwrap();
        let entry = expect_entry(get_job(&job_uuid, &context));
        assert_eq!(entry.job_type(), Ok(JobType::Backup));
        assert_eq!(entry.state(), Ok(JobState::Queued));
        assert!(entry.started_at.is_none());

        assert!(update_job_state(&job_uuid, JobState::Active).is_ok());
        assert!(set_job_step(&job_uuid, "save disks").is_ok());
        assert!(add_message_to_job(&job_uuid, "first").is_ok());
        assert!(add_message_to_job(&job_uuid, "second").is_ok());
        let entry = expect_entry(get_job(&job_uuid, &context));
        assert_eq!(entry.state(), Ok(JobState::Active));
        assert_eq!(entry.step, "save disks");
        assert_eq!(entry.messages, vec!["first", "second"]);
        assert!(entry.started_at.is_some());

        assert!(update_job_state(&job_uuid, JobState::Finished).is_ok());
        let entry = expect_entry(get_job(&job_uuid, &context));
        assert!(entry.finished_at.is_some());

        // jobs of other projects are only visible for admins
        let other = new_context("other-project", false, ProjectRole::Admin);
        assert!(get_job(&job_uuid, &other).is_err());
        let admin = new_context("other-project", true, ProjectRole::Member);
        expect_entry(get_job(&job_uuid, &admin));

        let listed = list_jobs(&context, Some(&backup_uuid)).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(list_jobs_of_backup(&backup_uuid).unwrap().len(), 1);

        hard_delete_job(&job_uuid);
    }

    #[test]
    #[serial]
    fn test_fail_unfinished_jobs() {
        let (queued, active, finished) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let backup_uuid = Uuid::new_v4();
        let context = new_context("test-project", false, ProjectRole::Member);
        for job in [&queued, &active, &finished] {
            hard_delete_job(job);
            add_new_job(
                job,
                JobType::Restore,
                &backup_uuid,
                "test-project",
                "{}",
                &context,
            )
            .unwrap();
        }
        assert!(update_job_state(&active, JobState::Active).is_ok());
        assert!(update_job_state(&finished, JobState::Finished).is_ok());

        let failed = fail_unfinished_jobs("restart").unwrap();
        let failed: Vec<Uuid> = failed.iter().map(|job| job.uuid).collect();
        assert!(failed.contains(&queued) && failed.contains(&active));
        assert!(!failed.contains(&finished));

        let entry = expect_entry(get_job(&active, &context));
        assert_eq!(entry.state(), Ok(JobState::Error));
        assert_eq!(entry.messages, vec!["restart"]);
        let entry = expect_entry(get_job(&finished, &context));
        assert_eq!(entry.state(), Ok(JobState::Finished));

        for job in [&queued, &active, &finished] {
            hard_delete_job(job);
        }
    }

    #[test]
    #[serial]
    fn test_observer_can_not_add_job() {
        let observer = new_context("test-project", false, ProjectRole::Observer);
        let result = add_new_job(
            &Uuid::new_v4(),
            JobType::Backup,
            &Uuid::new_v4(),
            "test-project",
            "{}",
            &observer,
        );
        assert!(matches!(result, Err(e) if enums::is_permission_denied(&e)));
    }
}
