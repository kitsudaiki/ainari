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

use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;
use ainari_common::objects::*;

// Define the schema for the proxys table
table! {
    proxys (uuid) {
        uuid -> Varchar,
        target_address -> Varchar,
        virtual_machine_uuid -> Varchar,
        owner_id -> Varchar,
        project_id -> Varchar,
        status -> Varchar,
        created_at -> Varchar,
        created_by -> Varchar,
        updated_at -> Varchar,
        updated_by -> Varchar,
        deleted_at -> Nullable<Varchar>,
        deleted_by -> Nullable<Varchar>,
    }
}

/// Represents a single entry in the proxys table.
/// Contains all fields necessary to store and retrieve proxy server information from the database.
///
/// # Fields
/// * `uuid` - Unique identifier for the proxy, which is part of its address as well
/// * `target_address` - Address the proxy forwards traffic to
/// * `virtual_machine_uuid` - UUID of the virtual_machine this proxy is associated with
/// * `owner_id` - User ID of the proxy owner
/// * `project_id` - Project ID the proxy belongs to
/// * `status` - Current status of the proxy (ACTIVE, DELETED, etc.)
/// * `created_at` - Timestamp when the proxy was created
/// * `created_by` - User ID who created the proxy
/// * `updated_at` - Timestamp when the proxy was last updated
/// * `updated_by` - User ID who last updated the proxy
/// * `deleted_at` - Timestamp when the proxy was deleted (nullable)
/// * `deleted_by` - User ID who deleted the proxy (nullable)
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = proxys)]
pub struct ProxyEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    pub target_address: String,
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub virtual_machine_uuid: Uuid,
    pub owner_id: String,
    pub project_id: String,
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

/// Adds a new proxy entry to the database with default values for a newly created proxy.
///
/// A proxy has the UUID of its virtual_machine. The UUID of a deleted virtual_machine can be used
/// again by its project, like by the restore of a backup, so a deleted proxy of the same project
/// with this UUID is replaced by the new one.
///
/// # Arguments
/// * `proxy_uuid` - Unique identifier for the new proxy
/// * `target_address` - Address the proxy will forward traffic to
/// * `virtual_machine_uuid` - UUID of the virtual_machine this proxy is associated with
/// * `context` - User context containing ownership and project information
///
/// # Returns
/// * `QueryResult<usize>` indicating the number of rows affected
pub fn add_new_proxy(
    proxy_uuid: &Uuid,
    target_address: &str,
    virtual_machine_uuid: &Uuid,
    context: &UserContext,
) -> QueryResult<usize> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::permission_denied_error());
    }

    // Create a new ProxyEntry with the provided parameters and current timestamps
    let proxy = ProxyEntry {
        uuid: *proxy_uuid,
        target_address: target_address.to_owned(),
        virtual_machine_uuid: *virtual_machine_uuid,
        owner_id: context.user_id.clone(),
        project_id: context.project_id.clone(),
        status: "ACTIVE".to_string(),
        created_at: Utc::now(),
        created_by: context.user_id.clone(),
        updated_at: Utc::now(),
        updated_by: context.user_id.clone(),
        deleted_at: None,
        deleted_by: None,
    };

    add_proxy_replacing_deleted(proxy)
}

/// Adds a proxy entry to the database and replaces a deleted proxy of the same project with the
/// same UUID within the same transaction.
///
/// # Arguments
/// * `proxy` - The ProxyEntry to be added
///
/// # Returns
/// * `QueryResult<usize>` indicating the number of added rows
fn add_proxy_replacing_deleted(proxy: ProxyEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::proxys::dsl::*;
    conn.transaction(|conn| {
        diesel::delete(
            proxys.filter(
                uuid.eq(proxy.uuid.to_string())
                    .and(status.eq("DELETED"))
                    .and(project_id.eq(proxy.project_id.clone())),
            ),
        )
        .execute(conn)?;
        diesel::insert_into(proxys).values(proxy).execute(conn)
    })
}

/// Retrieves a proxy entry from the database based on its UUID and user context.
///
/// # Arguments
/// * `proxy_uuid` - UUID of the proxy to retrieve
/// * `context` - User context used to apply appropriate access control filters
///
/// # Returns
/// * `Result<ProxyEntry, enums::DbError>` containing the proxy if found, or an appropriate error
pub fn get_proxy(proxy_uuid: &Uuid, context: &UserContext) -> Result<ProxyEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::proxys::dsl::*;

    // Start building the query with basic filtering for active proxies
    let mut query = proxys
        .filter(uuid.eq(proxy_uuid.to_string()).and(status.eq("ACTIVE")))
        .into_boxed();

    // Apply additional filters based on user permissions
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
    }

    match query
        .select(ProxyEntry::as_select())
        .first::<ProxyEntry>(&mut *conn)
    {
        Ok(proxy) => Ok(proxy),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Checks, if the UUID can't be used for a new proxy of a project.
///
/// The UUID is used by an active proxy or by a proxy of another project, also if that one is
/// already marked as deleted. A deleted proxy of the same project doesn't block its UUID, because
/// it is replaced by the new proxy (see `add_new_proxy`).
///
/// # Arguments
/// * `proxy_uuid` - UUID of the proxy
/// * `context` - User context with the project of the new proxy
///
/// # Returns
/// * `QueryResult<bool>` true, if the UUID can't be used for a new proxy
pub fn proxy_uuid_in_use(proxy_uuid: &Uuid, context: &UserContext) -> QueryResult<bool> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::proxys::dsl::*;

    diesel::select(diesel::dsl::exists(
        proxys.filter(
            uuid.eq(proxy_uuid.to_string()).and(
                status
                    .eq("ACTIVE")
                    .or(project_id.ne(context.project_id.clone())),
            ),
        ),
    ))
    .get_result(&mut *conn)
}

/// Lists all proxy entries from the database that the user has access to.
///
/// # Arguments
/// * `context` - User context used to apply appropriate access control filters
///
/// # Returns
/// * `QueryResult<Vec<ProxyEntry>>` containing all proxies the user can access
pub fn list_proxys(context: &UserContext) -> QueryResult<Vec<ProxyEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::proxys::dsl::*;

    // Start building the query with basic filtering for active proxies
    let mut query = proxys.filter(status.eq("ACTIVE")).into_boxed();

    // Apply additional filters based on user permissions
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
    }

    query.select(ProxyEntry::as_select()).load(&mut *conn)
}

/// Changes the target address of a proxy.
///
/// # Arguments
/// * `proxy_uuid` - UUID of the proxy
/// * `new_target_address` - The new address, which the proxy forwards to
/// * `context` - User context for permission checking and audit
///
/// # Returns
/// * `Ok(ProxyEntry)` with the updated proxy
/// * `Err(DbError)` if the proxy doesn't exist, the user is not allowed to change it or the
///   database failed
pub fn update_proxy_target(
    proxy_uuid: &Uuid,
    new_target_address: &str,
    context: &UserContext,
) -> Result<ProxyEntry, enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    // Verify the proxy exists and the user has permission to change it
    get_proxy(proxy_uuid, context)?;

    {
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        use self::proxys::dsl::*;
        if let Err(e) = diesel::update(proxys.filter(uuid.eq(proxy_uuid.to_string())))
            .set((
                target_address.eq(new_target_address),
                updated_at.eq(Utc::now().to_rfc3339()),
                updated_by.eq(context.user_id.clone()),
            ))
            .execute(&mut *conn)
        {
            log::error!("Database-error: {e:?}");
            return Err(enums::DbError::InternalError);
        }
    }

    get_proxy(proxy_uuid, context)
}

/// Marks a proxy as deleted in the database.
///
/// # Arguments
/// * `proxy_uuid` - UUID of the proxy to delete
/// * `context` - User context used for access control and to record who performed the deletion
///
/// # Returns
/// * `Result<(), enums::DbError>` indicating success or failure
pub fn delete_proxy(proxy_uuid: &Uuid, context: &UserContext) -> Result<(), enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    // Verify the proxy exists and the user has permission to delete it
    get_proxy(proxy_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::proxys::dsl::*;
    match diesel::update(proxys.filter(uuid.eq(proxy_uuid.to_string())))
        .set((
            status.eq("DELETED"),
            deleted_at.eq(Utc::now().to_rfc3339()),
            deleted_by.eq(context.user_id.clone()),
        ))
        .execute(&mut *conn)
    {
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

    /// Adds a proxy entry to the database, like it is, without replacing a deleted one.
    fn add_proxy(proxy: ProxyEntry) -> QueryResult<usize> {
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        use self::proxys::dsl::*;
        diesel::insert_into(proxys)
            .values(proxy)
            .execute(&mut *conn)
    }

    fn hard_delete_proxy(proxy_uuid: &Uuid) {
        use self::proxys::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(proxys.filter(uuid.eq(proxy_uuid.to_string()))).execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_add_get_proxy() {
        let proxy_uuid1 = Uuid::new_v4();
        let target_address1: String = "127.0.0.1:443".to_string();
        let virtual_machine_uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let proxy = ProxyEntry {
            uuid: proxy_uuid1,
            target_address: target_address1.clone(),
            virtual_machine_uuid: virtual_machine_uuid1,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_proxy(&proxy_uuid1);

        add_proxy(proxy.clone()).unwrap();
        match get_proxy(&proxy_uuid1, &context) {
            Ok(retrieved_proxy) => {
                assert_eq!(retrieved_proxy.uuid, proxy.uuid);
                assert_eq!(retrieved_proxy.target_address, proxy.target_address);
                assert_eq!(
                    retrieved_proxy.virtual_machine_uuid,
                    proxy.virtual_machine_uuid
                );
                assert_eq!(retrieved_proxy.status, proxy.status);
                assert_eq!(retrieved_proxy.created_by, proxy.created_by);
                assert_eq!(retrieved_proxy.updated_by, proxy.updated_by);
                assert_eq!(retrieved_proxy.deleted_at, proxy.deleted_at);
                assert_eq!(retrieved_proxy.deleted_by, proxy.deleted_by);
            }
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        hard_delete_proxy(&proxy_uuid1);
    }

    #[test]
    #[serial]
    fn test_proxy_uuid_in_use() {
        let proxy_uuid = Uuid::new_v4();
        let member = UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: "test-project".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let other_project = UserContext {
            project_id: "other-project".to_string(),
            ..member.clone()
        };

        hard_delete_proxy(&proxy_uuid);
        assert!(!proxy_uuid_in_use(&proxy_uuid, &member).unwrap());

        add_new_proxy(&proxy_uuid, "10.0.0.1:443", &proxy_uuid, &member).unwrap();
        assert!(proxy_uuid_in_use(&proxy_uuid, &member).unwrap());
        assert!(proxy_uuid_in_use(&proxy_uuid, &other_project).unwrap());

        // a deleted proxy only blocks its uuid for other projects
        assert!(delete_proxy(&proxy_uuid, &member).is_ok());
        assert!(!proxy_uuid_in_use(&proxy_uuid, &member).unwrap());
        assert!(proxy_uuid_in_use(&proxy_uuid, &other_project).unwrap());

        // the virtual_machine of the deleted proxy is restored with its uuid, so the deleted
        // proxy is replaced
        add_new_proxy(&proxy_uuid, "10.0.0.2:443", &proxy_uuid, &member).unwrap();
        let proxy = get_proxy(&proxy_uuid, &member).ok().unwrap();
        assert_eq!(proxy.status, "ACTIVE");
        assert_eq!(proxy.target_address, "10.0.0.2:443");

        // an active proxy is never replaced
        assert!(add_new_proxy(&proxy_uuid, "10.0.0.3:443", &proxy_uuid, &member).is_err());

        hard_delete_proxy(&proxy_uuid);
    }

    #[test]
    #[serial]
    fn test_update_proxy_target() {
        let proxy_uuid = Uuid::new_v4();
        let member = UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: "test-project".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        let observer = UserContext {
            project_role: ProjectRole::Observer.to_string(),
            ..member.clone()
        };
        let other_project = UserContext {
            project_id: "other-project".to_string(),
            ..member.clone()
        };

        hard_delete_proxy(&proxy_uuid);
        add_new_proxy(&proxy_uuid, "10.0.0.1:443", &Uuid::new_v4(), &member).unwrap();

        // the proxy stays, only the target changes
        let updated = update_proxy_target(&proxy_uuid, "10.0.0.2:443", &member)
            .unwrap_or_else(|_| panic!("update of proxy failed"));
        assert_eq!(updated.target_address, "10.0.0.2:443");
        assert_eq!(updated.uuid, proxy_uuid);

        assert!(matches!(
            update_proxy_target(&proxy_uuid, "10.0.0.3:443", &observer),
            Err(enums::DbError::PermissionDenied)
        ));
        assert!(matches!(
            update_proxy_target(&proxy_uuid, "10.0.0.3:443", &other_project),
            Err(enums::DbError::NotFound)
        ));
        assert!(matches!(
            update_proxy_target(&Uuid::new_v4(), "10.0.0.3:443", &member),
            Err(enums::DbError::NotFound)
        ));
        let stored = get_proxy(&proxy_uuid, &member).unwrap_or_else(|_| panic!("proxy is gone"));
        assert_eq!(stored.target_address, "10.0.0.2:443");

        hard_delete_proxy(&proxy_uuid);
    }

    #[test]
    #[serial]
    fn test_list_proxys() {
        let proxy_uuid1 = Uuid::new_v4();
        let proxy_uuid2 = Uuid::new_v4();
        let target_address1: String = "127.0.0.1:443".to_string();
        let virtual_machine_uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let proxy1 = ProxyEntry {
            uuid: proxy_uuid1,
            target_address: target_address1.clone(),
            virtual_machine_uuid: virtual_machine_uuid1,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let proxy2 = ProxyEntry {
            uuid: proxy_uuid2,
            target_address: target_address1.clone(),
            virtual_machine_uuid: virtual_machine_uuid1,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "DELETED".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_proxy(&proxy_uuid1);
        hard_delete_proxy(&proxy_uuid2);

        add_proxy(proxy1).unwrap();
        add_proxy(proxy2).unwrap();
        let proxys = list_proxys(&context).unwrap();
        assert_eq!(proxys.len(), 1);
        hard_delete_proxy(&proxy_uuid1);
        hard_delete_proxy(&proxy_uuid2);
    }

    #[test]
    #[serial]
    fn test_delete_proxy() {
        let proxy_uuid1 = Uuid::new_v4();
        let target_address1: String = "127.0.0.1:443".to_string();
        let virtual_machine_uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let proxy = ProxyEntry {
            uuid: proxy_uuid1,
            target_address: target_address1.clone(),
            virtual_machine_uuid: virtual_machine_uuid1,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_proxy(&proxy_uuid1);

        add_proxy(proxy).unwrap();
        let _ = delete_proxy(&proxy_uuid1, &context);
        let result = get_proxy(&proxy_uuid1, &context);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_proxys_permissions() {
        let proxy_uuid1 = Uuid::new_v4();
        let proxy_uuid2 = Uuid::new_v4();
        let proxy_uuid3 = Uuid::new_v4();
        let target_address1: String = "127.0.0.1:443".to_string();
        let virtual_machine_uuid1 = Uuid::new_v4();

        let proxy1 = ProxyEntry {
            uuid: proxy_uuid1,
            target_address: target_address1.clone(),
            virtual_machine_uuid: virtual_machine_uuid1,
            owner_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let proxy2 = ProxyEntry {
            uuid: proxy_uuid2,
            target_address: target_address1.clone(),
            virtual_machine_uuid: virtual_machine_uuid1,
            owner_id: "test-user-43".to_string(),
            project_id: "test_permissions_1".to_string(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let proxy3 = ProxyEntry {
            uuid: proxy_uuid3,
            target_address: target_address1.clone(),
            virtual_machine_uuid: virtual_machine_uuid1,
            owner_id: "test-user-44".to_string(),
            project_id: "test_permissions_2".to_string(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_proxy(&proxy_uuid1);
        hard_delete_proxy(&proxy_uuid2);
        hard_delete_proxy(&proxy_uuid3);

        add_proxy(proxy1).unwrap();
        add_proxy(proxy2).unwrap();
        add_proxy(proxy3).unwrap();

        // list-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        let proxys = list_proxys(&context).unwrap();
        // members see all entries of their project, also the ones of other users
        assert_eq!(proxys.len(), 2);

        // list-test project-admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Admin.to_string(),
        };
        let proxys = list_proxys(&context).unwrap();
        assert_eq!(proxys.len(), 2);

        // list-test admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: true.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        let proxys = list_proxys(&context).unwrap();
        assert_eq!(proxys.len(), 3);

        // get-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        match get_proxy(&proxy_uuid1, &context) {
            Ok(retrieved_proxy) => {
                assert_eq!(retrieved_proxy.uuid, proxy_uuid1);
            }
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        // get-test normal user false uuid
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        if get_proxy(&proxy_uuid3, &context).is_ok() {
            assert_eq!(true, false);
        };

        // delete-test normal user false uuid
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        if delete_proxy(&proxy_uuid3, &context).is_ok() {
            assert_eq!(true, false);
        };

        hard_delete_proxy(&proxy_uuid1);
        hard_delete_proxy(&proxy_uuid2);
        hard_delete_proxy(&proxy_uuid3);
    }
}
