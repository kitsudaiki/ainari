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

use crate::database::backup_table;
use crate::database::db_handle;
use crate::database::image_table;

use ainari_api_structs::user_context::UserContext;
use ainari_common::database::DbConnection;
use ainari_common::enums;
use ainari_common::objects::*;

// Define the schema for the hosts table
table! {
    hosts (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        address -> Varchar,
        is_backup_target -> Bool,
        created_at -> Varchar,
        created_by -> Varchar,
        updated_at -> Varchar,
        updated_by -> Varchar,
    }
}

/// Represents a single host entry in the database.
///
/// This struct maps to the `hosts` table in the database and contains
/// all fields necessary to identify, track, and manage a host.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = hosts)]
pub struct HostEntry {
    /// Unique identifier for the host
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    /// Human-readable name of the host
    pub name: String,
    /// Network address of the host
    pub address: String,
    /// True, if the host only stores backups and never normal images or snapshots
    pub is_backup_target: bool,
    /// Timestamp when the host was created
    #[diesel(serialize_as = DbDateTime, deserialize_as = DbDateTime)]
    pub created_at: DateTime<Utc>,
    /// User ID who created the host
    pub created_by: String,
    /// Timestamp when the host was last updated
    #[diesel(serialize_as = DbDateTime, deserialize_as = DbDateTime)]
    pub updated_at: DateTime<Utc>,
    /// User ID who last updated the host
    pub updated_by: String,
}

/// Adds a new host to the database with default values.
///
/// This function creates a new HostEntry with the provided parameters and
/// sets default values for fields like the timestamps.
///
/// # Arguments
///
/// * `host_uuid` - The unique identifier for the new host
/// * `host_name` - The human-readable name of the host
/// * `host_address` - The network address of the host
/// * `context` - The user context containing the ID of the user creating the host
///
/// # Returns
///
/// * `QueryResult<usize>` - The number of rows affected by the insert operation
pub fn add_new_host(
    host_uuid: &Uuid,
    host_name: &str,
    host_address: &str,
    context: &UserContext,
) -> QueryResult<usize> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::permission_denied_error());
    }

    let host = HostEntry {
        uuid: *host_uuid,
        name: host_name.to_owned(),
        address: host_address.to_owned(),
        is_backup_target: false,
        created_at: Utc::now(),
        created_by: context.user_id.clone(),
        updated_at: Utc::now(),
        updated_by: context.user_id.clone(),
    };

    add_host(host.clone())
}

/// Adds an existing HostEntry to the database.
///
/// This is a lower-level function that takes a fully constructed HostEntry
/// and inserts it into the database.
///
/// # Arguments
///
/// * `host` - The HostEntry to insert into the database
///
/// # Returns
///
/// * `QueryResult<usize>` - The number of rows affected by the insert operation
pub fn add_host(host: HostEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;
    diesel::insert_into(hosts).values(host).execute(&mut *conn)
}

/// Retrieves a host from the database by its address.
///
/// An already deregistered host is removed, so it is reported as not found.
///
/// # Arguments
///
/// * `host_address` - Address of the host to retrieve
/// * `_` - The user context (unused in this function but maintained for consistency)
///
/// # Returns
///
/// * `Ok(HostEntry)` with the host on success
/// * `Err(enums::DbError)` if there is no host with that address
pub fn get_host_by_address(
    host_address: &String,
    _: &UserContext,
) -> Result<HostEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;

    let query = hosts
        .filter(address.eq(host_address.to_string()))
        .into_boxed();

    match query
        .select(HostEntry::as_select())
        .first::<HostEntry>(&mut *conn)
    {
        Ok(host) => Ok(host),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Retrieves an onsen-host from the database.
///
/// An already deregistered host is removed, so it is reported as not found. The
/// hosts are part of the infrastructure and not owned by a user, so no ownership-filter is applied
/// and the user-context is only taken for consistency with the other tables.
///
/// # Arguments
///
/// * `host_uuid` - The UUID of the host to retrieve
///
/// # Returns
///
/// * `Result<HostEntry, enums::DbError>` - The requested host or an error
pub fn get_host(host_uuid: &Uuid, _: &UserContext) -> Result<HostEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;

    let query = hosts.filter(uuid.eq(host_uuid.to_string())).into_boxed();

    match query
        .select(HostEntry::as_select())
        .first::<HostEntry>(&mut *conn)
    {
        Ok(host) => Ok(host),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Retrieves a list of all hosts from the database.
///
/// # Arguments
///
/// * `_` - The user context (unused in this function but maintained for consistency)
///
/// # Returns
///
/// * `QueryResult<Vec<HostEntry>>` - A vector of all hosts
pub fn list_hosts(_: &UserContext) -> QueryResult<Vec<HostEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;

    let query = hosts.into_boxed();

    query.select(HostEntry::as_select()).load(&mut *conn)
}

/// Marks an onsen-host as backup-target or removes the mark (admin function).
///
/// A host can only be marked, while no image or snapshot is stored on it, and only be unmarked,
/// while no backup is registered on it. The check and the update run in one transaction, so no
/// image or backup can be added to the host in between. Backups, which were copied onto the host,
/// but are not registered, are checked by the caller over the onsen.
///
/// # Arguments
///
/// * `host_uuid` - The UUID of the host
/// * `backup_target` - True to mark the host as backup-target, false to remove the mark
/// * `context` - The user context containing the ID of the user performing the change
///
/// # Returns
///
/// * `Ok(true)` if the host was updated
/// * `Ok(false)` if the host can not be marked, because images are stored on it, or can not be
///   unmarked, because backups are registered on it
/// * `Err(enums::DbError)` if the host doesn't exist or the update failed
pub fn set_backup_target(
    host_uuid: &Uuid,
    backup_target: bool,
    context: &UserContext,
) -> Result<bool, enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;

    let host = *host_uuid;
    let host_uuid = host_uuid.to_string();

    let result = conn.transaction::<_, diesel::result::Error, _>(|conn| {
        let host_address = hosts
            .filter(uuid.eq(&host_uuid))
            .select(address)
            .first::<String>(conn)?;

        if backup_target && image_table::count_images_on_onsen(conn, &host_address)? > 0 {
            return Ok(false);
        }
        if !backup_target && backup_table::count_backups_on_host(conn, &host)? > 0 {
            return Ok(false);
        }

        diesel::update(hosts.filter(uuid.eq(&host_uuid)))
            .set((
                is_backup_target.eq(backup_target),
                updated_at.eq(Utc::now().to_rfc3339()),
                updated_by.eq(context.user_id.clone()),
            ))
            .execute(conn)?;
        Ok(true)
    });

    match result {
        Ok(changed) => Ok(changed),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Checks, if an onsen-address belongs to a backup-target.
///
/// It takes the already locked connection, so it can be used within the transaction of another
/// table.
///
/// # Arguments
///
/// * `conn` - The locked database-connection
/// * `onsen_address` - Address of the onsen-host
///
/// # Returns
///
/// * `QueryResult<bool>` - True, if a host with this address is a backup-target
pub fn is_backup_target_address(conn: &mut DbConnection, onsen_address: &str) -> QueryResult<bool> {
    use self::hosts::dsl::*;

    hosts
        .filter(address.eq(onsen_address).and(is_backup_target.eq(true)))
        .select(count_star())
        .first::<i64>(conn)
        .map(|count| count > 0)
}

/// Checks, if a host is a backup-target.
///
/// It takes the already locked connection, so it can be used within the transaction of another
/// table.
///
/// # Arguments
///
/// * `conn` - The locked database-connection
/// * `host` - UUID of the onsen-host
///
/// # Returns
///
/// * `QueryResult<bool>` - True, if the host exists and is a backup-target
pub fn is_backup_target_host(conn: &mut DbConnection, host: &Uuid) -> QueryResult<bool> {
    use self::hosts::dsl::*;

    hosts
        .filter(uuid.eq(host.to_string()).and(is_backup_target.eq(true)))
        .select(count_star())
        .first::<i64>(conn)
        .map(|count| count > 0)
}

/// Deletes a host from the database (admin function).
///
/// It first verifies the host exists.
///
/// # Arguments
///
/// * `host_uuid` - The UUID of the host to delete
/// * `context` - The user context containing the ID of the user performing the deletion
///
/// # Returns
///
/// * `Result<(), enums::DbError>` - Ok if successful, or an error
pub fn delete_host_admin(host_uuid: &Uuid, context: &UserContext) -> Result<(), enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    get_host(host_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;
    match diesel::delete(hosts.filter(uuid.eq(host_uuid.to_string()))).execute(&mut *conn) {
        Ok(_) => Ok(()),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Deletes all hosts from the database (admin function).
///
/// This is typically used during application startup or shutdown.
///
/// # Returns
///
/// * `Result<(), enums::DbError>` - Ok if successful, or an error
#[allow(dead_code)]
pub fn delete_all_host() -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;
    match diesel::delete(hosts).execute(&mut *conn) {
        Ok(_) => Ok(()),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ainari_common::enums::ProjectRole;
    use serial_test::serial;

    fn hard_delete_host(host_uuid: &Uuid) {
        use self::hosts::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(hosts.filter(uuid.eq(host_uuid.to_string()))).execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_add_get_host() {
        let uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let host = HostEntry {
            uuid: uuid1,
            name: "Alice".to_string(),
            address: "http://127.0.0.1:11420".to_string(),
            is_backup_target: false,
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
        };

        hard_delete_host(&uuid1);

        add_host(host.clone()).unwrap();
        match get_host(&uuid1, &context) {
            Ok(retrieved_host) => {
                assert_eq!(retrieved_host.uuid, host.uuid);
                assert_eq!(retrieved_host.name, host.name);
                assert_eq!(retrieved_host.created_by, host.created_by);
                assert_eq!(retrieved_host.updated_by, host.updated_by);
            }
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        hard_delete_host(&uuid1);
    }

    #[test]
    #[serial]
    fn test_list_hosts() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let host1 = HostEntry {
            uuid: uuid1,
            name: "Alice".to_string(),
            address: "http://127.0.0.1:11420".to_string(),
            is_backup_target: false,
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
        };

        let host2 = HostEntry {
            uuid: uuid2,
            name: "Bob".to_string(),
            address: "http://127.0.0.1:11420".to_string(),
            is_backup_target: false,
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
        };

        hard_delete_host(&uuid1);
        hard_delete_host(&uuid2);

        add_host(host1).unwrap();
        add_host(host2).unwrap();
        assert!(delete_host_admin(&uuid2, &context).is_ok());
        let hosts = list_hosts(&context).unwrap();
        assert_eq!(hosts.len(), 1);
        hard_delete_host(&uuid1);
        hard_delete_host(&uuid2);
    }

    #[test]
    #[serial]
    fn test_delete_host() {
        let uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let host = HostEntry {
            uuid: uuid1,
            name: "Alice".to_string(),
            address: "http://127.0.0.1:11420".to_string(),
            is_backup_target: false,
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
        };

        hard_delete_host(&uuid1);

        add_host(host).unwrap();
        let _ = delete_host_admin(&uuid1, &context);
        let result = get_host(&uuid1, &context);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_hosts_permissions() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();

        let host1 = HostEntry {
            uuid: uuid1,
            name: "Alice".to_string(),
            address: "http://127.0.0.1:11420".to_string(),
            is_backup_target: false,
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
        };

        let host2 = HostEntry {
            uuid: uuid2,
            name: "Bob".to_string(),
            address: "http://127.0.0.1:11420".to_string(),
            is_backup_target: false,
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
        };

        let host3 = HostEntry {
            uuid: uuid3,
            name: "Poi".to_string(),
            address: "http://127.0.0.1:11420".to_string(),
            is_backup_target: false,
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
        };

        hard_delete_host(&uuid1);
        hard_delete_host(&uuid2);
        hard_delete_host(&uuid3);

        add_host(host1).unwrap();
        add_host(host2).unwrap();
        add_host(host3).unwrap();

        // list-test
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: true.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        let hosts = list_hosts(&context).unwrap();
        assert_eq!(hosts.len(), 3);

        // get-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        match get_host(&uuid1, &context) {
            Ok(retrieved_host) => {
                assert_eq!(retrieved_host.uuid, uuid1);
            }
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        hard_delete_host(&uuid1);
        hard_delete_host(&uuid2);
        hard_delete_host(&uuid3);
    }

    fn admin_context() -> UserContext {
        UserContext {
            token: "".to_string(),
            user_id: "test-admin".to_string(),
            project_id: "test-project".to_string(),
            is_admin: true.to_string(),
            project_role: ProjectRole::Member.to_string(),
        }
    }

    /// Adds a host with a unique address, which is not a backup-target.
    fn add_test_host() -> HostEntry {
        let host = HostEntry {
            uuid: Uuid::new_v4(),
            name: "onsen".to_string(),
            address: format!("http://{}:50051", Uuid::new_v4()),
            is_backup_target: false,
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
        };
        add_host(host.clone()).unwrap();
        host
    }

    fn is_marked(host_uuid: &Uuid) -> bool {
        match get_host(host_uuid, &admin_context()) {
            Ok(host) => host.is_backup_target,
            Err(_) => panic!("host not found"),
        }
    }

    fn add_test_image(onsen_address: &str) -> QueryResult<Uuid> {
        let image_uuid = Uuid::new_v4();
        image_table::add_new_image(
            &image_uuid,
            "image",
            onsen_address,
            &format!("images/{image_uuid}"),
            "qcow2",
            4096,
            &Uuid::new_v4(),
            false,
            &ainari_api_structs::image_structs::ImageStatus::Ready,
            &admin_context(),
        )
        .map(|_| image_uuid)
    }

    fn hard_delete_images_on(address: &str) {
        use crate::database::image_table::images::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(images.filter(onsen_address.eq(address))).execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_set_backup_target() {
        let context = admin_context();
        let host = add_test_host();

        // mark and unmark an empty host
        assert!(matches!(
            set_backup_target(&host.uuid, true, &context),
            Ok(true)
        ));
        assert!(is_marked(&host.uuid));
        assert!(
            list_hosts(&context)
                .unwrap()
                .iter()
                .any(|h| h.uuid == host.uuid && h.is_backup_target)
        );
        assert!(matches!(
            set_backup_target(&host.uuid, false, &context),
            Ok(true)
        ));
        assert!(!is_marked(&host.uuid));

        // unknown or deleted host
        assert!(matches!(
            set_backup_target(&Uuid::new_v4(), true, &context),
            Err(enums::DbError::NotFound)
        ));

        // observers are not allowed to change the host
        let observer = UserContext {
            is_admin: false.to_string(),
            project_role: ProjectRole::Observer.to_string(),
            ..context.clone()
        };
        assert!(matches!(
            set_backup_target(&host.uuid, true, &observer),
            Err(enums::DbError::PermissionDenied)
        ));

        hard_delete_host(&host.uuid);
    }

    #[test]
    #[serial]
    fn test_set_backup_target_with_images() {
        let context = admin_context();
        let host = add_test_host();
        let image_uuid = add_test_image(&host.address).unwrap();

        // a host with an image can not become a backup-target
        assert!(matches!(
            set_backup_target(&host.uuid, true, &context),
            Ok(false)
        ));
        assert!(!is_marked(&host.uuid));

        // after the image was deleted, it can
        assert!(image_table::delete_image(&image_uuid, &context).is_ok());
        assert!(matches!(
            set_backup_target(&host.uuid, true, &context),
            Ok(true)
        ));

        hard_delete_images_on(&host.address);
        hard_delete_host(&host.uuid);
    }

    #[test]
    #[serial]
    fn test_unset_backup_target_with_backups() {
        let context = admin_context();
        let host = add_test_host();
        assert!(matches!(
            set_backup_target(&host.uuid, true, &context),
            Ok(true)
        ));
        {
            let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
            assert!(is_backup_target_host(&mut conn, &host.uuid).unwrap());
            assert!(!is_backup_target_host(&mut conn, &Uuid::new_v4()).unwrap());
        }

        // a host with a registered backup can not be unmarked
        let backup_uuid = Uuid::new_v4();
        backup_table::add_new_backup(&backup_uuid, &host.uuid, "project", None, &context).unwrap();
        assert!(matches!(
            set_backup_target(&host.uuid, false, &context),
            Ok(false)
        ));
        assert!(is_marked(&host.uuid));

        // after the backup was deleted, it can
        use ainari_api_structs::backup_structs::BackupStatus;
        assert!(
            backup_table::change_backup_status(
                &backup_uuid,
                &[BackupStatus::Creating],
                BackupStatus::Deleting,
                None,
                &context,
            )
            .is_ok()
        );
        assert!(backup_table::delete_backup(&backup_uuid, &context).is_ok());
        assert!(matches!(
            set_backup_target(&host.uuid, false, &context),
            Ok(true)
        ));
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        assert!(!is_backup_target_host(&mut conn, &host.uuid).unwrap());
        {
            use crate::database::backup_table::backups::dsl::*;
            let _ = diesel::delete(backups.filter(uuid.eq(backup_uuid.to_string())))
                .execute(&mut *conn);
        }
        drop(conn);

        hard_delete_host(&host.uuid);
    }

    #[test]
    #[serial]
    fn test_no_images_on_backup_target() {
        let context = admin_context();
        let host = add_test_host();
        assert!(matches!(
            set_backup_target(&host.uuid, true, &context),
            Ok(true)
        ));

        {
            let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
            assert!(is_backup_target_address(&mut conn, &host.address).unwrap());
            assert!(!is_backup_target_address(&mut conn, "http://unknown:50051").unwrap());
        }

        // adding an image on a backup-target is a hard error, even if the host was selected
        // before it was marked
        assert!(add_test_image(&host.address).is_err());
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        assert_eq!(
            image_table::count_images_on_onsen(&mut conn, &host.address).unwrap(),
            0
        );
        drop(conn);

        // a deleted host is no backup-target anymore
        assert!(delete_host_admin(&host.uuid, &context).is_ok());
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        assert!(!is_backup_target_address(&mut conn, &host.address).unwrap());
        drop(conn);

        hard_delete_images_on(&host.address);
        hard_delete_host(&host.uuid);
    }
}
