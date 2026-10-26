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
use diesel::dsl::count_star;
use diesel::prelude::*;
use uuid::Uuid;

use crate::database::db_handle;
use crate::database::host_table;
use crate::onsen_functions::BACKUP_DIRECTORY;

use ainari_api_structs::backup_structs::BackupStatus;
use ainari_api_structs::user_context::UserContext;
use ainari_common::database::DbConnection;
use ainari_common::enums;
use ainari_common::objects::*;

// Define the schema
table! {
    backups (uuid) {
        uuid -> Varchar,
        host_uuid -> Varchar,
        directory_path -> Text,
        backup_status -> Varchar,
        file_count -> Integer,
        total_size -> BigInt,
        manifest_sha256 -> Nullable<Varchar>,
        owner_id -> Varchar,
        project_id -> Varchar,
        created_at -> Varchar,
        created_by -> Varchar,
        updated_at -> Varchar,
        updated_by -> Varchar,
    }
}

/// Represents the package of a backup in the database.
///
/// The package is one directory on a backup-target. Its single files are not tracked, only the
/// package as a whole.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = backups)]
pub struct BackupEntry {
    /// Unique identifier of the backup, which is also the name of its directory
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    /// Backup-target, which stores the package
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub host_uuid: Uuid,
    /// Path of the package relative to the storage-location of the onsen
    pub directory_path: String,
    /// State of the package (see `BackupStatus`)
    pub backup_status: String,
    /// Number of files in the package, set when the package is sealed
    pub file_count: i32,
    /// Size of all files of the package in bytes, set when the package is sealed
    pub total_size: i64,
    /// Hex-encoded sha256 of the manifest, set when the package is sealed
    pub manifest_sha256: Option<String>,
    /// ID of the user who owns this backup
    pub owner_id: String,
    /// ID of the project this backup belongs to
    pub project_id: String,
    /// Timestamp when the backup was created
    #[diesel(serialize_as = DbDateTime, deserialize_as = DbDateTime)]
    pub created_at: DateTime<Utc>,
    /// ID of the user who created the backup
    pub created_by: String,
    /// Timestamp when the backup was last updated
    #[diesel(serialize_as = DbDateTime, deserialize_as = DbDateTime)]
    pub updated_at: DateTime<Utc>,
    /// ID of the user who last updated the backup
    pub updated_by: String,
}

/// Summary of the files of a complete package, which is stored, when the package is sealed.
#[derive(Debug, PartialEq, Clone)]
pub struct PackageInfo {
    /// Number of files in the package
    pub file_count: i32,
    /// Size of all files of the package in bytes
    pub total_size: i64,
    /// Hex-encoded sha256 of the manifest of the package
    pub manifest_sha256: String,
}

/// Result of adding a new backup.
#[derive(Debug, PartialEq)]
pub enum BackupAddResult {
    /// The backup was added
    Added,
    /// The host is no active backup-target, so it can not store backups
    NoBackupTarget,
    /// A backup with the same UUID was already registered before
    AlreadyExists,
}

/// Result of a change of the state of a package.
#[derive(Debug, PartialEq)]
pub enum StatusChange {
    /// The state was changed
    Changed,
    /// The package is not in one of the expected states, so it was not changed
    WrongStatus,
}

/// Returns the path of the directory of a backup relative to the storage-location of the onsen.
///
/// # Arguments
///
/// * `backup_uuid` - UUID of the backup
///
/// # Returns
///
/// The path `backups/<backup_uuid>`
pub fn backup_directory_path(backup_uuid: &Uuid) -> String {
    format!("{BACKUP_DIRECTORY}/{backup_uuid}")
}

/// Adds a new backup to the database.
///
/// Without `package` the backup starts as `CREATING`, so files can be written into it. With
/// `package` it is registered as already complete and sealed package with the state `READY`,
/// which is used for packages, which were copied onto the backup-target from outside.
///
/// # Arguments
///
/// * `backup_uuid` - The unique identifier for the new backup
/// * `host_uuid` - The backup-target, which stores the package
/// * `project` - The project, which the backup belongs to
/// * `package` - Summary of the files of an already complete package
/// * `context` - The user context containing authentication information
///
/// # Returns
///
/// * `QueryResult<BackupAddResult>` - If the backup was added, or why it was not added
pub fn add_new_backup(
    backup_uuid: &Uuid,
    host_uuid: &Uuid,
    project: &str,
    package: Option<&PackageInfo>,
    context: &UserContext,
) -> QueryResult<BackupAddResult> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::permission_denied_error());
    }

    let backup = BackupEntry {
        uuid: *backup_uuid,
        host_uuid: *host_uuid,
        directory_path: backup_directory_path(backup_uuid),
        backup_status: match package {
            Some(_) => BackupStatus::Ready.to_string(),
            None => BackupStatus::Creating.to_string(),
        },
        file_count: package.map_or(0, |p| p.file_count),
        total_size: package.map_or(0, |p| p.total_size),
        manifest_sha256: package.map(|p| p.manifest_sha256.clone()),
        owner_id: context.user_id.clone(),
        project_id: project.to_owned(),
        created_at: Utc::now(),
        created_by: context.user_id.clone(),
        updated_at: Utc::now(),
        updated_by: context.user_id.clone(),
    };

    add_backup(backup)
}

/// Adds a backup to the database.
///
/// The host is checked within the same transaction like the marking of the host as
/// backup-target, so a backup can never be stored on a host, which was unmarked in between.
///
/// # Arguments
///
/// * `backup` - The `BackupEntry` to insert
///
/// # Returns
///
/// * `QueryResult<BackupAddResult>` - If the backup was added, or why it was not added
pub fn add_backup(backup: BackupEntry) -> QueryResult<BackupAddResult> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::backups::dsl::*;

    conn.transaction(|conn| {
        if !host_table::is_backup_target_host(conn, &backup.host_uuid)? {
            return Ok(BackupAddResult::NoBackupTarget);
        }
        // the UUID names the directory, so it can only be used by one backup
        let existing = backups
            .filter(uuid.eq(backup.uuid.to_string()))
            .select(count_star())
            .first::<i64>(conn)?;
        if existing > 0 {
            return Ok(BackupAddResult::AlreadyExists);
        }
        diesel::insert_into(backups).values(backup).execute(conn)?;
        Ok(BackupAddResult::Added)
    })
}

/// Checks, if a backup with this UUID is registered.
///
/// # Arguments
///
/// * `backup_uuid` - UUID of the backup
///
/// # Returns
///
/// * `QueryResult<bool>` - True, if the UUID is known
pub fn is_registered(backup_uuid: &Uuid) -> QueryResult<bool> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::backups::dsl::*;

    backups
        .filter(uuid.eq(backup_uuid.to_string()))
        .select(count_star())
        .first::<i64>(&mut *conn)
        .map(|count| count > 0)
}

/// Retrieves a backup from the database.
///
/// Users, which are not admin, only see the backups of their own project.
///
/// # Arguments
///
/// * `backup_uuid` - The UUID of the backup to retrieve
/// * `context` - The user context containing authentication information
///
/// # Returns
///
/// * `Result<BackupEntry, enums::DbError>` - The requested backup or an error
pub fn get_backup(
    backup_uuid: &Uuid,
    context: &UserContext,
) -> Result<BackupEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::backups::dsl::*;

    let mut query = backups
        .filter(uuid.eq(backup_uuid.to_string()))
        .into_boxed();

    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
    }

    match query
        .select(BackupEntry::as_select())
        .first::<BackupEntry>(&mut *conn)
    {
        Ok(backup) => Ok(backup),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Lists all backups accessible to the user.
///
/// # Arguments
///
/// * `context` - The user context containing authentication information
/// * `project` - Optional project to list only its backups
///
/// # Returns
///
/// * `QueryResult<Vec<BackupEntry>>` - The accessible backups
pub fn list_backups(context: &UserContext, project: Option<&str>) -> QueryResult<Vec<BackupEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::backups::dsl::*;

    let mut query = backups.into_boxed();

    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
    }
    if let Some(project) = project {
        query = query.filter(project_id.eq(project.to_owned()));
    }

    query.select(BackupEntry::as_select()).load(&mut *conn)
}

/// Counts the backups, which are stored on a backup-target.
///
/// It takes the already locked connection, so it can be used within the transaction of another
/// table.
///
/// # Arguments
///
/// * `conn` - The locked database-connection
/// * `host` - UUID of the backup-target
///
/// # Returns
///
/// * `QueryResult<i64>` - The number of backups on the host
pub fn count_backups_on_host(conn: &mut DbConnection, host: &Uuid) -> QueryResult<i64> {
    use self::backups::dsl::*;

    backups
        .filter(host_uuid.eq(host.to_string()))
        .select(count_star())
        .first::<i64>(conn)
}

/// Changes the state of a package, if it is in one of the expected states.
///
/// The check of the current state and the change are done by one conditional update, so of two
/// concurrent changes, like sealing and deleting the same package, only one can succeed.
///
/// # Arguments
///
/// * `backup_uuid` - The UUID of the backup
/// * `expected` - The states, which allow the change
/// * `new_status` - The new state of the package
/// * `package` - Summary of the files, which is stored together with the new state
/// * `context` - The user context containing authentication information
///
/// # Returns
///
/// * `Result<StatusChange, enums::DbError>` - If the state was changed, or an error
pub fn change_backup_status(
    backup_uuid: &Uuid,
    expected: &[BackupStatus],
    new_status: BackupStatus,
    package: Option<&PackageInfo>,
    context: &UserContext,
) -> Result<StatusChange, enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    // First verify that the backup exists and is accessible to the user
    get_backup(backup_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::backups::dsl::*;

    let expected: Vec<&str> = expected.iter().map(BackupStatus::as_str).collect();
    let target = backups.filter(
        uuid.eq(backup_uuid.to_string())
            .and(backup_status.eq_any(expected)),
    );
    let result = match package {
        Some(package) => diesel::update(target)
            .set((
                backup_status.eq(new_status.as_str()),
                file_count.eq(package.file_count),
                total_size.eq(package.total_size),
                manifest_sha256.eq(Some(package.manifest_sha256.clone())),
                updated_at.eq(Utc::now().to_rfc3339()),
                updated_by.eq(context.user_id.clone()),
            ))
            .execute(&mut *conn),
        None => diesel::update(target)
            .set((
                backup_status.eq(new_status.as_str()),
                updated_at.eq(Utc::now().to_rfc3339()),
                updated_by.eq(context.user_id.clone()),
            ))
            .execute(&mut *conn),
    };

    match result {
        Ok(0) => Ok(StatusChange::WrongStatus),
        Ok(_) => Ok(StatusChange::Changed),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Deletes a backup from the database, after its package was deleted from the backup-target.
///
/// Only a package in the state `DELETING` can be deleted, so a backup can't disappear from the
/// database, while its files still exist.
///
/// # Arguments
///
/// * `backup_uuid` - The UUID of the backup to delete
/// * `context` - The user context containing authentication information
///
/// # Returns
///
/// * `Result<StatusChange, enums::DbError>` - If the backup was deleted, or an error
pub fn delete_backup(
    backup_uuid: &Uuid,
    context: &UserContext,
) -> Result<StatusChange, enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    // First verify that the backup exists and is accessible to the user
    get_backup(backup_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::backups::dsl::*;

    match diesel::delete(
        backups.filter(
            uuid.eq(backup_uuid.to_string())
                .and(backup_status.eq(BackupStatus::Deleting.as_str())),
        ),
    )
    .execute(&mut *conn)
    {
        Ok(0) => Ok(StatusChange::WrongStatus),
        Ok(_) => Ok(StatusChange::Changed),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::host_table::HostEntry;
    use ainari_common::enums::ProjectRole;
    use serial_test::serial;

    fn context(project: &str, is_admin: bool, role: ProjectRole) -> UserContext {
        UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: project.to_string(),
            is_admin: is_admin.to_string(),
            project_role: role.to_string(),
        }
    }

    /// Unwraps the result of a database-function, whose error has no `Debug`
    fn ok<T>(result: Result<T, enums::DbError>) -> T {
        match result {
            Ok(value) => value,
            Err(_) => panic!("unexpected database-error"),
        }
    }

    fn member() -> UserContext {
        context("test-backup-project", false, ProjectRole::Member)
    }

    fn add_test_host(is_backup_target: bool) -> HostEntry {
        let host = HostEntry {
            uuid: Uuid::new_v4(),
            name: "onsen".to_string(),
            address: format!("http://{}:50051", Uuid::new_v4()),
            is_backup_target,
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
        };
        host_table::add_host(host.clone()).unwrap();
        host
    }

    fn hard_delete(backup_uuid: &Uuid, host: &Uuid) {
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        {
            use self::backups::dsl::*;
            let _ = diesel::delete(backups.filter(uuid.eq(backup_uuid.to_string())))
                .execute(&mut *conn);
        }
        use crate::database::host_table::hosts::dsl::*;
        let _ = diesel::delete(hosts.filter(uuid.eq(host.to_string()))).execute(&mut *conn);
    }

    fn status_of(backup_uuid: &Uuid) -> String {
        ok(get_backup(backup_uuid, &member())).backup_status
    }

    fn package() -> PackageInfo {
        PackageInfo {
            file_count: 7,
            total_size: 1 << 33,
            manifest_sha256: "ab".repeat(32),
        }
    }

    #[test]
    #[serial]
    fn test_add_get_backup() {
        let host = add_test_host(true);
        let backup_uuid = Uuid::new_v4();
        let ctx = member();

        assert_eq!(
            add_new_backup(&backup_uuid, &host.uuid, &ctx.project_id, None, &ctx).unwrap(),
            BackupAddResult::Added
        );
        let backup = ok(get_backup(&backup_uuid, &ctx));
        assert_eq!(backup.host_uuid, host.uuid);
        assert_eq!(backup.directory_path, format!("backups/{backup_uuid}"));
        assert_eq!(backup.backup_status, "CREATING");
        assert_eq!(backup.file_count, 0);
        assert_eq!(backup.total_size, 0);
        assert_eq!(backup.manifest_sha256, None);
        assert_eq!(backup.project_id, ctx.project_id);
        assert_eq!(backup.owner_id, ctx.user_id);

        // the same UUID can not be used twice
        assert_eq!(
            add_new_backup(&backup_uuid, &host.uuid, &ctx.project_id, None, &ctx).unwrap(),
            BackupAddResult::AlreadyExists
        );
        assert!(is_registered(&backup_uuid).unwrap());
        assert!(!is_registered(&Uuid::new_v4()).unwrap());

        hard_delete(&backup_uuid, &host.uuid);
    }

    #[test]
    #[serial]
    fn test_add_registered_backup() {
        let host = add_test_host(true);
        let backup_uuid = Uuid::new_v4();
        let ctx = member();

        // an already complete package is directly sealed
        let package = package();
        assert_eq!(
            add_new_backup(&backup_uuid, &host.uuid, "other", Some(&package), &ctx).unwrap(),
            BackupAddResult::Added
        );
        let admin = context("test-backup-project", true, ProjectRole::Member);
        let backup = ok(get_backup(&backup_uuid, &admin));
        assert_eq!(backup.backup_status, "READY");
        assert_eq!(backup.file_count, package.file_count);
        assert_eq!(backup.total_size, package.total_size);
        assert_eq!(backup.manifest_sha256, Some(package.manifest_sha256));
        assert_eq!(backup.project_id, "other");

        hard_delete(&backup_uuid, &host.uuid);
    }

    #[test]
    #[serial]
    fn test_backups_only_on_backup_targets() {
        let ctx = member();
        let image_host = add_test_host(false);
        let backup_uuid = Uuid::new_v4();

        assert_eq!(
            add_new_backup(&backup_uuid, &image_host.uuid, &ctx.project_id, None, &ctx).unwrap(),
            BackupAddResult::NoBackupTarget
        );
        assert_eq!(
            add_new_backup(&backup_uuid, &Uuid::new_v4(), &ctx.project_id, None, &ctx).unwrap(),
            BackupAddResult::NoBackupTarget
        );
        assert!(!is_registered(&backup_uuid).unwrap());

        // a deleted backup-target can not store backups anymore
        let backup_host = add_test_host(true);
        ok(host_table::delete_host_admin(&backup_host.uuid, &ctx));
        assert_eq!(
            add_new_backup(&backup_uuid, &backup_host.uuid, &ctx.project_id, None, &ctx).unwrap(),
            BackupAddResult::NoBackupTarget
        );

        hard_delete(&backup_uuid, &image_host.uuid);
        hard_delete(&backup_uuid, &backup_host.uuid);
    }

    #[test]
    #[serial]
    fn test_backup_lifecycle() {
        let host = add_test_host(true);
        let backup_uuid = Uuid::new_v4();
        let ctx = member();
        add_new_backup(&backup_uuid, &host.uuid, &ctx.project_id, None, &ctx).unwrap();

        // a package, which is still created, can not be deleted from the database
        assert_eq!(
            ok(delete_backup(&backup_uuid, &ctx)),
            StatusChange::WrongStatus
        );

        // seal the package
        let package = package();
        assert_eq!(
            ok(change_backup_status(
                &backup_uuid,
                &[BackupStatus::Creating],
                BackupStatus::Ready,
                Some(&package),
                &ctx
            )),
            StatusChange::Changed
        );
        let backup = ok(get_backup(&backup_uuid, &ctx));
        assert_eq!(backup.backup_status, "READY");
        assert_eq!(backup.file_count, package.file_count);
        assert_eq!(backup.manifest_sha256, Some(package.manifest_sha256));

        // a sealed package can not be sealed again
        assert_eq!(
            ok(change_backup_status(
                &backup_uuid,
                &[BackupStatus::Creating],
                BackupStatus::Ready,
                None,
                &ctx
            )),
            StatusChange::WrongStatus
        );

        // delete the package
        assert_eq!(
            ok(change_backup_status(
                &backup_uuid,
                &[BackupStatus::Ready],
                BackupStatus::Deleting,
                None,
                &ctx
            )),
            StatusChange::Changed
        );
        assert_eq!(status_of(&backup_uuid), "DELETING");
        assert_eq!(ok(delete_backup(&backup_uuid, &ctx)), StatusChange::Changed);
        assert!(matches!(
            get_backup(&backup_uuid, &ctx),
            Err(enums::DbError::NotFound)
        ));
        assert!(matches!(
            change_backup_status(
                &backup_uuid,
                &[BackupStatus::Deleting],
                BackupStatus::Error,
                None,
                &ctx
            ),
            Err(enums::DbError::NotFound)
        ));
        // the UUID is free again after the deletion
        assert!(!is_registered(&backup_uuid).unwrap());

        hard_delete(&backup_uuid, &host.uuid);
    }

    #[test]
    #[serial]
    fn test_count_backups_on_host() {
        let host = add_test_host(true);
        let backup_uuid = Uuid::new_v4();
        let ctx = member();
        let count = || {
            let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
            count_backups_on_host(&mut conn, &host.uuid).unwrap()
        };

        assert_eq!(count(), 0);
        add_new_backup(&backup_uuid, &host.uuid, &ctx.project_id, None, &ctx).unwrap();
        assert_eq!(count(), 1);

        // a package in deletion still blocks the host, only a deleted one doesn't
        ok(change_backup_status(
            &backup_uuid,
            &[BackupStatus::Creating],
            BackupStatus::Deleting,
            None,
            &ctx,
        ));
        assert_eq!(count(), 1);
        ok(delete_backup(&backup_uuid, &ctx));
        assert_eq!(count(), 0);

        hard_delete(&backup_uuid, &host.uuid);
    }

    #[test]
    #[serial]
    fn test_backups_permissions() {
        let host = add_test_host(true);
        let own_uuid = Uuid::new_v4();
        let other_uuid = Uuid::new_v4();
        let ctx = member();
        let admin = context("test-backup-project", true, ProjectRole::Member);
        add_new_backup(&own_uuid, &host.uuid, &ctx.project_id, None, &ctx).unwrap();
        add_new_backup(&other_uuid, &host.uuid, "test-backup-other", None, &admin).unwrap();

        // members only see the backups of their own project
        let own = list_backups(&ctx, None).unwrap();
        assert!(own.iter().any(|b| b.uuid == own_uuid));
        assert!(!own.iter().any(|b| b.uuid == other_uuid));
        assert!(matches!(
            get_backup(&other_uuid, &ctx),
            Err(enums::DbError::NotFound)
        ));
        assert!(
            list_backups(&ctx, Some("test-backup-other"))
                .unwrap()
                .is_empty()
        );
        assert!(matches!(
            change_backup_status(
                &other_uuid,
                &[BackupStatus::Creating],
                BackupStatus::Deleting,
                None,
                &ctx
            ),
            Err(enums::DbError::NotFound)
        ));

        // admins see all backups and can filter them by project
        let other = list_backups(&admin, Some("test-backup-other")).unwrap();
        assert_eq!(other.len(), 1);
        assert_eq!(other[0].uuid, other_uuid);
        assert!(get_backup(&other_uuid, &admin).is_ok());

        // observers can only read
        let observer = context("test-backup-project", false, ProjectRole::Observer);
        assert!(get_backup(&own_uuid, &observer).is_ok());
        assert!(add_new_backup(&Uuid::new_v4(), &host.uuid, "x", None, &observer).is_err());
        assert!(matches!(
            change_backup_status(
                &own_uuid,
                &[BackupStatus::Creating],
                BackupStatus::Deleting,
                None,
                &observer
            ),
            Err(enums::DbError::PermissionDenied)
        ));
        assert!(matches!(
            delete_backup(&own_uuid, &observer),
            Err(enums::DbError::PermissionDenied)
        ));

        hard_delete(&own_uuid, &host.uuid);
        hard_delete(&other_uuid, &host.uuid);
    }
}
