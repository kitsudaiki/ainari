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

use ainari_api_structs::backup_structs::{BackupKind, BackupStatus};
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;
use ainari_common::objects::*;

table! {
    backups (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        backup_kind -> Varchar,
        project_id -> Varchar,
        virtual_machine_uuid -> Nullable<Varchar>,
        backup_state -> Varchar,
        ainari_version -> Varchar,
        secret_uuids -> Text,
        owner_id -> Varchar,
        status -> Varchar,
        created_at -> Varchar,
        created_by -> Varchar,
        updated_at -> Varchar,
        updated_by -> Varchar,
        deleted_at -> Nullable<Varchar>,
        deleted_by -> Nullable<Varchar>,
    }
}

/// Represents an entry in the backups table.
///
/// The host and the size of a backup are not stored here, because they belong to the
/// backup-resource with the same UUID in ryokan, which stores the files of the backup.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = backups)]
pub struct BackupEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    pub name: String,
    /// `PROJECT` or `VIRTUAL_MACHINE`
    pub backup_kind: String,
    pub project_id: String,
    /// saved virtual_machine of a backup of the kind `VIRTUAL_MACHINE`
    #[diesel(serialize_as = DbOptUuid, deserialize_as = DbOptUuid)]
    pub virtual_machine_uuid: Option<Uuid>,
    /// `CREATING`, `READY`, `DELETING` or `ERROR`
    pub backup_state: String,
    /// version of ainari, which has created the backup
    pub ainari_version: String,
    /// secrets in omamori, which encrypt the disks of the backup and which are deleted together
    /// with the backup
    #[diesel(serialize_as = DbVecString, deserialize_as = DbVecString)]
    pub secret_uuids: Vec<String>,
    pub owner_id: String,
    pub status: String,
    #[diesel(serialize_as = DbDateTime, deserialize_as = DbDateTime)]
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    #[diesel(serialize_as = DbDateTime, deserialize_as = DbDateTime)]
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
    #[diesel(serialize_as = DbOptDateTime, deserialize_as = DbOptDateTime)]
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<String>,
}

impl BackupEntry {
    /// Kind of the backup.
    ///
    /// # Returns
    ///
    /// The kind, or an error, if the database contains an invalid value.
    pub fn kind(&self) -> Result<BackupKind, String> {
        self.backup_kind.parse()
    }

    /// State of the backup.
    ///
    /// # Returns
    ///
    /// The state, or an error, if the database contains an invalid value.
    pub fn state(&self) -> Result<BackupStatus, String> {
        self.backup_state.parse()
    }

    /// Secrets of the disks of the backup.
    ///
    /// # Returns
    ///
    /// The UUIDs of the secrets, which are valid UUIDs.
    pub fn secrets(&self) -> Vec<Uuid> {
        self.secret_uuids
            .iter()
            .filter_map(|secret| Uuid::parse_str(secret).ok())
            .collect()
    }
}

/// Adds a new backup in the state `CREATING` to the database.
///
/// # Arguments
///
/// * `backup_uuid` - UUID of the backup, which is also used by the backup-resource in ryokan
/// * `name` - Name of the backup
/// * `kind` - Content of the backup
/// * `virtual_machine_uuid` - Saved virtual_machine of a backup of the kind `VIRTUAL_MACHINE`
/// * `context` - The user context, whose project is saved
///
/// # Returns
///
/// A QueryResult indicating the number of rows affected
pub fn add_new_backup(
    backup_uuid: &Uuid,
    name: &str,
    kind: BackupKind,
    virtual_machine_uuid: Option<Uuid>,
    context: &UserContext,
) -> QueryResult<usize> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::permission_denied_error());
    }

    let now = Utc::now();
    add_backup(BackupEntry {
        uuid: *backup_uuid,
        name: name.to_string(),
        backup_kind: kind.to_string(),
        project_id: context.project_id.clone(),
        virtual_machine_uuid,
        backup_state: BackupStatus::Creating.to_string(),
        ainari_version: ainari_version(),
        secret_uuids: Vec::new(),
        owner_id: context.user_id.clone(),
        status: "ACTIVE".to_string(),
        created_at: now,
        created_by: context.user_id.clone(),
        updated_at: now,
        updated_by: context.user_id.clone(),
        deleted_at: None,
        deleted_by: None,
    })
}

/// Adds a backup, which was created by another deployment and is imported, in the state `READY`.
///
/// An entry of a deleted backup with the same UUID is replaced, so a package can be imported again,
/// after its entry was deleted.
///
/// # Arguments
///
/// * `backup_uuid` - UUID of the backup, which is also used by the backup-resource in ryokan
/// * `backup_name` - Name of the backup
/// * `kind` - Content of the backup
/// * `project` - Project of the backup
/// * `vm_uuid` - Saved virtual_machine of a backup of the kind `VIRTUAL_MACHINE`
/// * `version` - Version of ainari, which has created the backup
/// * `context` - The user context of the admin, who imports the backup
///
/// # Returns
///
/// A QueryResult indicating the number of rows affected
pub fn add_imported_backup(
    backup_uuid: &Uuid,
    backup_name: &str,
    kind: BackupKind,
    project: &str,
    vm_uuid: Option<Uuid>,
    version: &str,
    context: &UserContext,
) -> QueryResult<usize> {
    let now = Utc::now();
    let entry = BackupEntry {
        uuid: *backup_uuid,
        name: backup_name.to_string(),
        backup_kind: kind.to_string(),
        project_id: project.to_string(),
        virtual_machine_uuid: vm_uuid,
        backup_state: BackupStatus::Ready.to_string(),
        ainari_version: version.to_string(),
        // the secrets of the disks are only part of the backup itself
        secret_uuids: Vec::new(),
        owner_id: context.user_id.clone(),
        status: "ACTIVE".to_string(),
        created_at: now,
        created_by: context.user_id.clone(),
        updated_at: now,
        updated_by: context.user_id.clone(),
        deleted_at: None,
        deleted_by: None,
    };
    replace_deleted_backup(entry)
}

/// Adds a backup and removes an entry of a deleted backup with the same UUID before.
///
/// # Arguments
///
/// * `entry` - The backup to insert
///
/// # Returns
///
/// A QueryResult indicating the number of rows affected
fn replace_deleted_backup(entry: BackupEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::backups::dsl::*;
    let backup_uuid = entry.uuid;
    conn.transaction(|conn| {
        diesel::delete(
            backups
                .filter(uuid.eq(backup_uuid.to_string()))
                .filter(status.ne("ACTIVE")),
        )
        .execute(conn)?;
        diesel::insert_into(backups).values(entry).execute(conn)
    })
}

/// Checks, if a backup is known, which is not deleted, independent of its project.
///
/// # Arguments
///
/// * `backup_uuid` - The UUID of the backup
///
/// # Returns
///
/// True, if the backup exists
pub fn is_known(backup_uuid: &Uuid) -> QueryResult<bool> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::backups::dsl::*;

    backups
        .filter(uuid.eq(backup_uuid.to_string()))
        .filter(status.eq("ACTIVE"))
        .select(uuid)
        .first::<String>(&mut *conn)
        .optional()
        .map(|entry| entry.is_some())
}

/// Version of ainari, which is stored with each backup.
///
/// # Returns
///
/// The version, like it is reported by the version-endpoint.
pub fn ainari_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Adds a backup to the database.
///
/// # Arguments
///
/// * `entry` - The backup to insert
///
/// # Returns
///
/// A QueryResult indicating the number of rows affected
pub fn add_backup(entry: BackupEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::backups::dsl::*;
    diesel::insert_into(backups)
        .values(entry)
        .execute(&mut *conn)
}

/// Retrieves a backup from the database.
///
/// Only backups, which are not deleted, are returned. Users, which are not admin, only see the
/// backups of their own project.
///
/// # Arguments
///
/// * `backup_uuid` - The UUID of the backup
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
///
/// A Result containing the BackupEntry, or a DbError if not found or an error occurs
pub fn get_backup(
    backup_uuid: &Uuid,
    context: &UserContext,
) -> Result<BackupEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::backups::dsl::*;

    let mut query = backups
        .filter(uuid.eq(backup_uuid.to_string()).and(status.eq("ACTIVE")))
        .into_boxed();

    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
    }

    match query
        .select(BackupEntry::as_select())
        .first::<BackupEntry>(&mut *conn)
    {
        Ok(entry) => Ok(entry),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Lists the backups, which the user has access to.
///
/// # Arguments
///
/// * `context` - The user context containing information about the user and their permissions
/// * `filter_project_id` - List only the backups of this project
///
/// # Returns
///
/// A QueryResult containing the backups
pub fn list_backups(
    context: &UserContext,
    filter_project_id: Option<&str>,
) -> QueryResult<Vec<BackupEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::backups::dsl::*;

    let mut query = backups.filter(status.eq("ACTIVE")).into_boxed();

    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
    }
    if let Some(filter) = filter_project_id {
        query = query.filter(project_id.eq(filter.to_string()));
    }

    query
        .order(created_at.desc())
        .select(BackupEntry::as_select())
        .load(&mut *conn)
}

/// Changes the state of a backup.
///
/// # Arguments
///
/// * `backup_uuid` - The UUID of the backup
/// * `new_state` - The new state
///
/// # Returns
///
/// Ok, if the backup was updated, `NotFound`, if it doesn't exist
pub fn update_backup_state(
    backup_uuid: &Uuid,
    new_state: BackupStatus,
) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::backups::dsl::*;

    match diesel::update(backups.filter(uuid.eq(backup_uuid.to_string())))
        .set((
            backup_state.eq(new_state.to_string()),
            updated_at.eq(Utc::now().to_rfc3339()),
        ))
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

/// Changes the state of a backup only, if it has an expected state.
///
/// Of two concurrent changes of the same backup, like a deletion and the end of its job, only one
/// can succeed.
///
/// # Arguments
///
/// * `backup_uuid` - The UUID of the backup
/// * `expected` - The state, which the backup must have
/// * `new_state` - The new state
///
/// # Returns
///
/// True, if the state was changed, false, if the backup had another state
pub fn change_backup_state(
    backup_uuid: &Uuid,
    expected: BackupStatus,
    new_state: BackupStatus,
) -> Result<bool, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::backups::dsl::*;

    match diesel::update(
        backups
            .filter(uuid.eq(backup_uuid.to_string()))
            .filter(backup_state.eq(expected.to_string())),
    )
    .set((
        backup_state.eq(new_state.to_string()),
        updated_at.eq(Utc::now().to_rfc3339()),
    ))
    .execute(&mut *conn)
    {
        Ok(count) => Ok(count > 0),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Adds the secret of a disk to a backup, so it is deleted together with the backup.
///
/// # Arguments
///
/// * `backup_uuid` - The UUID of the backup
/// * `secret_uuid` - The secret, which encrypts a disk of the backup
///
/// # Returns
///
/// Ok, if the secret was added, `NotFound`, if the backup doesn't exist
pub fn add_backup_secret(backup_uuid: &Uuid, secret_uuid: &Uuid) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::backups::dsl::*;

    let result = conn.transaction::<_, diesel::result::Error, _>(|conn| {
        let entry = backups
            .filter(uuid.eq(backup_uuid.to_string()))
            .select(BackupEntry::as_select())
            .first::<BackupEntry>(conn)?;
        let mut secrets = entry.secret_uuids;
        secrets.push(secret_uuid.to_string());
        diesel::update(backups.filter(uuid.eq(backup_uuid.to_string())))
            .set(secret_uuids.eq(DbVecString::from(secrets).0))
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

/// Marks a backup as deleted.
///
/// # Arguments
///
/// * `backup_uuid` - The UUID of the backup
/// * `context` - The user context of the user, who deletes the backup
///
/// # Returns
///
/// Ok, if the backup was deleted, `NotFound`, if it doesn't exist
pub fn delete_backup(backup_uuid: &Uuid, context: &UserContext) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::backups::dsl::*;

    match diesel::update(
        backups
            .filter(uuid.eq(backup_uuid.to_string()))
            .filter(status.eq("ACTIVE")),
    )
    .set((
        status.eq("DELETED"),
        deleted_at.eq(Utc::now().to_rfc3339()),
        deleted_by.eq(context.user_id.clone()),
    ))
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

/// Marks all backups, which were still created, as failed.
///
/// The jobs don't survive a restart of isekai, so these backups are never finished.
///
/// # Returns
///
/// The number of backups, which were marked as failed
pub fn fail_unfinished_backups() -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::backups::dsl::*;

    diesel::update(
        backups
            .filter(status.eq("ACTIVE"))
            .filter(backup_state.eq(BackupStatus::Creating.to_string())),
    )
    .set((
        backup_state.eq(BackupStatus::Error.to_string()),
        updated_at.eq(Utc::now().to_rfc3339()),
    ))
    .execute(&mut *conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ainari_common::enums::ProjectRole;
    use serial_test::serial;

    fn hard_delete_backup(backup_uuid: &Uuid) {
        use self::backups::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ =
            diesel::delete(backups.filter(uuid.eq(backup_uuid.to_string()))).execute(&mut *conn);
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

    fn expect_entry(result: Result<BackupEntry, enums::DbError>) -> BackupEntry {
        match result {
            Ok(entry) => entry,
            Err(_) => panic!("backup was not found"),
        }
    }

    #[test]
    #[serial]
    fn test_add_get_backup() {
        let backup_uuid = Uuid::new_v4();
        let vm_uuid = Uuid::new_v4();
        let context = new_context("test-project", false, ProjectRole::Member);
        hard_delete_backup(&backup_uuid);

        add_new_backup(
            &backup_uuid,
            "backup",
            BackupKind::VirtualMachine,
            Some(vm_uuid),
            &context,
        )
        .unwrap();
        let entry = expect_entry(get_backup(&backup_uuid, &context));
        assert_eq!(entry.name, "backup");
        assert_eq!(entry.kind(), Ok(BackupKind::VirtualMachine));
        assert_eq!(entry.state(), Ok(BackupStatus::Creating));
        assert_eq!(entry.virtual_machine_uuid, Some(vm_uuid));
        assert_eq!(entry.project_id, "test-project");
        assert!(entry.secrets().is_empty());

        // invisible for other projects, but not for admins
        let other = new_context("other-project", false, ProjectRole::Admin);
        assert!(matches!(
            get_backup(&backup_uuid, &other),
            Err(enums::DbError::NotFound)
        ));
        let admin = new_context("other-project", true, ProjectRole::Member);
        expect_entry(get_backup(&backup_uuid, &admin));

        hard_delete_backup(&backup_uuid);
    }

    #[test]
    #[serial]
    fn test_observer_can_not_add_backup() {
        let backup_uuid = Uuid::new_v4();
        let observer = new_context("test-project", false, ProjectRole::Observer);
        let result = add_new_backup(&backup_uuid, "backup", BackupKind::Project, None, &observer);
        assert!(matches!(result, Err(e) if enums::is_permission_denied(&e)));
    }

    #[test]
    #[serial]
    fn test_states_and_secrets() {
        let backup_uuid = Uuid::new_v4();
        let context = new_context("test-project", false, ProjectRole::Member);
        hard_delete_backup(&backup_uuid);
        add_new_backup(&backup_uuid, "backup", BackupKind::Project, None, &context).unwrap();

        let (secret1, secret2) = (Uuid::new_v4(), Uuid::new_v4());
        assert!(add_backup_secret(&backup_uuid, &secret1).is_ok());
        assert!(add_backup_secret(&backup_uuid, &secret2).is_ok());
        let entry = expect_entry(get_backup(&backup_uuid, &context));
        assert_eq!(entry.secrets(), vec![secret1, secret2]);

        // only the expected state is changed
        assert!(matches!(
            change_backup_state(&backup_uuid, BackupStatus::Ready, BackupStatus::Deleting),
            Ok(false)
        ));
        assert!(matches!(
            change_backup_state(&backup_uuid, BackupStatus::Creating, BackupStatus::Ready),
            Ok(true)
        ));
        let entry = expect_entry(get_backup(&backup_uuid, &context));
        assert_eq!(entry.state(), Ok(BackupStatus::Ready));

        assert!(update_backup_state(&backup_uuid, BackupStatus::Error).is_ok());
        let entry = expect_entry(get_backup(&backup_uuid, &context));
        assert_eq!(entry.state(), Ok(BackupStatus::Error));

        hard_delete_backup(&backup_uuid);
    }

    #[test]
    #[serial]
    fn test_list_and_delete_backups() {
        let (uuid1, uuid2) = (Uuid::new_v4(), Uuid::new_v4());
        let context = new_context("test-list-project", false, ProjectRole::Member);
        let other = new_context("test-list-other", false, ProjectRole::Member);
        hard_delete_backup(&uuid1);
        hard_delete_backup(&uuid2);

        add_new_backup(&uuid1, "backup-1", BackupKind::Project, None, &context).unwrap();
        add_new_backup(&uuid2, "backup-2", BackupKind::Project, None, &other).unwrap();

        let listed = list_backups(&context, None).unwrap();
        assert!(listed.iter().any(|entry| entry.uuid == uuid1));
        assert!(listed.iter().all(|entry| entry.uuid != uuid2));

        // the filter of a user can't extend the visible projects
        let listed = list_backups(&context, Some("test-list-other")).unwrap();
        assert!(listed.is_empty());

        assert!(delete_backup(&uuid1, &context).is_ok());
        assert!(matches!(
            get_backup(&uuid1, &context),
            Err(enums::DbError::NotFound)
        ));
        assert!(matches!(
            delete_backup(&uuid1, &context),
            Err(enums::DbError::NotFound)
        ));

        hard_delete_backup(&uuid1);
        hard_delete_backup(&uuid2);
    }

    #[test]
    #[serial]
    fn test_add_imported_backup() {
        let backup_uuid = Uuid::new_v4();
        let admin = new_context("admin-project", true, ProjectRole::Member);
        hard_delete_backup(&backup_uuid);
        assert!(!is_known(&backup_uuid).unwrap());

        add_imported_backup(
            &backup_uuid,
            "imported",
            BackupKind::Project,
            "old-project",
            None,
            "0.21.0",
            &admin,
        )
        .unwrap();
        assert!(is_known(&backup_uuid).unwrap());
        let entry = expect_entry(get_backup(&backup_uuid, &admin));
        assert_eq!(entry.state(), Ok(BackupStatus::Ready));
        assert_eq!(entry.project_id, "old-project");
        assert_eq!(entry.ainari_version, "0.21.0");

        // an active backup can't be imported twice, but a deleted one again
        let again = |name| {
            add_imported_backup(
                &backup_uuid,
                name,
                BackupKind::Project,
                "old-project",
                None,
                "0.21.0",
                &admin,
            )
        };
        assert!(again("twice").is_err());
        assert!(delete_backup(&backup_uuid, &admin).is_ok());
        assert!(!is_known(&backup_uuid).unwrap());
        again("again").unwrap();
        assert_eq!(expect_entry(get_backup(&backup_uuid, &admin)).name, "again");

        hard_delete_backup(&backup_uuid);
    }

    #[test]
    #[serial]
    fn test_fail_unfinished_backups() {
        let (creating, ready) = (Uuid::new_v4(), Uuid::new_v4());
        let context = new_context("test-project", false, ProjectRole::Member);
        hard_delete_backup(&creating);
        hard_delete_backup(&ready);
        add_new_backup(&creating, "backup", BackupKind::Project, None, &context).unwrap();
        add_new_backup(&ready, "backup", BackupKind::Project, None, &context).unwrap();
        assert!(update_backup_state(&ready, BackupStatus::Ready).is_ok());

        assert!(fail_unfinished_backups().unwrap() >= 1);
        let entry = expect_entry(get_backup(&creating, &context));
        assert_eq!(entry.state(), Ok(BackupStatus::Error));
        let entry = expect_entry(get_backup(&ready, &context));
        assert_eq!(entry.state(), Ok(BackupStatus::Ready));

        hard_delete_backup(&creating);
        hard_delete_backup(&ready);
    }
}
