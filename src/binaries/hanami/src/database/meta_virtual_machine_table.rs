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
use crate::database::host_table::HostResources;

use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;
use ainari_common::objects::*;

// Define the schema for meta_virtual_machines table
table! {
    meta_virtual_machines (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        sakura_host_uuid -> Varchar,
        proxy_uuid -> Varchar,
        owner_id -> Varchar,
        project_id -> Varchar,
        status -> Varchar,
        created_at -> Varchar,
        created_by -> Varchar,
        updated_at -> Varchar,
        updated_by -> Varchar,
        deleted_at -> Nullable<Varchar>,
        deleted_by -> Nullable<Varchar>,
        number_of_cores -> BigInt,
        memory_size -> BigInt,
        disk_size -> BigInt,
    }
}

/// Represents an entry in the meta_virtual_machines table.
/// This struct contains all the fields required to create, query, and update meta virtual_machine records.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = meta_virtual_machines)]
pub struct MetaVirtualMachineEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    pub name: String,
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub sakura_host_uuid: Uuid,
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub proxy_uuid: Uuid,
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
    /// Number of cpu-cores of the virtual_machine
    pub number_of_cores: i64,
    /// Memory of the virtual_machine in MiB
    pub memory_size: i64,
    /// Size of the disk of the virtual_machine in GiB
    pub disk_size: i64,
}

/// Adds a new meta virtual_machine to the database.
///
/// This function creates a new MetaVirtualMachineEntry with the provided parameters and inserts it into the database.
/// The status is set to "ACTIVE" and timestamps are set to the current time.
///
/// # Arguments
/// * `meta_virtual_machine_uuid` - The unique identifier for the meta virtual_machine
/// * `virtual_machine_name` - The name of the meta virtual_machine
/// * `sakura_host_uuid` - The UUID of the Sakura host associated with this virtual_machine
/// * `proxy_uuid` - The UUID of the proxy associated with this virtual_machine
/// * `resources` - Cores, memory in MiB and disk-size in GiB of the virtual_machine
/// * `context` - The user context containing information about the user and project
///
/// # Returns
/// A QueryResult indicating the number of rows affected
pub fn add_new_meta_virtual_machine(
    meta_virtual_machine_uuid: &Uuid,
    virtual_machine_name: &str,
    sakura_host_uuid: &Uuid,
    proxy_uuid: &Uuid,
    resources: &HostResources,
    context: &UserContext,
) -> QueryResult<usize> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::permission_denied_error());
    }

    let meta_virtual_machine = MetaVirtualMachineEntry {
        uuid: *meta_virtual_machine_uuid,
        name: virtual_machine_name.to_string().clone(),
        sakura_host_uuid: *sakura_host_uuid,
        proxy_uuid: *proxy_uuid,
        owner_id: context.user_id.clone(),
        project_id: context.project_id.clone(),
        status: "ACTIVE".to_string(),
        created_at: Utc::now(),
        created_by: context.user_id.clone(),
        updated_at: Utc::now(),
        updated_by: context.user_id.clone(),
        deleted_at: None,
        deleted_by: None,
        number_of_cores: resources.number_of_cores,
        memory_size: resources.memory_size,
        disk_size: resources.disk_space,
    };

    add_meta_virtual_machine(meta_virtual_machine.clone())
}

/// Adds a meta virtual_machine to the database.
///
/// This is a helper function that performs the actual insertion of a MetaVirtualMachineEntry into the database.
///
/// # Arguments
/// * `meta_virtual_machine` - The MetaVirtualMachineEntry to be inserted
///
/// # Returns
/// A QueryResult indicating the number of rows affected
pub fn add_meta_virtual_machine(
    meta_virtual_machine: MetaVirtualMachineEntry,
) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::meta_virtual_machines::dsl::*;
    diesel::insert_into(meta_virtual_machines)
        .values(meta_virtual_machine)
        .execute(&mut *conn)
}

/// Retrieves a meta virtual_machine from the database.
///
/// This function queries the database for a meta virtual_machine with the specified UUID and checks the user's permissions.
/// Only active virtual_machines are returned, and the query is filtered based on the user's role and project membership.
///
/// # Arguments
/// * `meta_virtual_machine_uuid` - The UUID of the meta virtual_machine to retrieve
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A Result containing the MetaVirtualMachineEntry if found, or a DbError if not found or an error occurs
pub fn get_meta_virtual_machine(
    meta_virtual_machine_uuid: &Uuid,
    context: &UserContext,
) -> Result<MetaVirtualMachineEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::meta_virtual_machines::dsl::*;

    let mut query = meta_virtual_machines
        .filter(
            uuid.eq(meta_virtual_machine_uuid.to_string())
                .and(status.eq("ACTIVE")),
        )
        .into_boxed();

    // Apply permission-based filtering
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
    }

    match query
        .select(MetaVirtualMachineEntry::as_select())
        .first::<MetaVirtualMachineEntry>(&mut *conn)
    {
        Ok(meta_virtual_machine) => Ok(meta_virtual_machine),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Lists all meta virtual_machines that the user has access to.
///
/// This function retrieves all active meta virtual_machines and applies permission-based filtering.
/// The results are filtered based on the user's role and project membership.
///
/// # Arguments
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A QueryResult containing a vector of MetaVirtualMachineEntry objects
#[allow(dead_code)]
pub fn list_meta_virtual_machines(
    context: &UserContext,
) -> QueryResult<Vec<MetaVirtualMachineEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::meta_virtual_machines::dsl::*;

    let mut query = meta_virtual_machines
        .filter(status.eq("ACTIVE"))
        .into_boxed();

    // Apply permission-based filtering
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
    }

    query
        .select(MetaVirtualMachineEntry::as_select())
        .load(&mut *conn)
}

/// Counts the number of meta virtual_machines that the user has access to.
///
/// This function counts all active meta virtual_machines and applies permission-based filtering.
/// The count is filtered based on the user's role and project membership.
///
/// # Arguments
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A QueryResult containing the count of meta virtual_machines as an i64
pub fn count_meta_virtual_machines(context: &UserContext) -> QueryResult<i64> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::meta_virtual_machines::dsl::*;

    let mut query = meta_virtual_machines
        .filter(status.eq("ACTIVE"))
        .into_boxed();

    // Apply permission-based filtering
    query = query.filter(project_id.eq(context.project_id.clone()));
    query = query.filter(owner_id.eq(context.user_id.clone()));

    query.select(count_star()).first::<i64>(&mut *conn)
}

/// Counts the number of virtual machines of a whole project.
///
/// Unlike `count_meta_virtual_machines`, the virtual machines of all users of the project are counted, because the quota,
/// which is checked with this number, belongs to the project.
///
/// # Arguments
///
/// * `project` - The ID of the project, which is counted
///
/// # Returns
///
/// A QueryResult containing the count of virtual machines as an i64
pub fn count_meta_virtual_machines_of_project(project: &str) -> QueryResult<i64> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::meta_virtual_machines::dsl::*;

    meta_virtual_machines
        .filter(status.eq("ACTIVE"))
        .filter(project_id.eq(project))
        .select(count_star())
        .first::<i64>(&mut *conn)
}

/// Moves a meta virtual_machine to another sakura-host, after it was migrated there.
///
/// There is no permission-based filtering, because only an admin migrates virtual_machines.
///
/// # Arguments
/// * `meta_virtual_machine_uuid` - The UUID of the meta virtual_machine
/// * `new_sakura_host_uuid` - The UUID of the sakura-host, which runs the virtual_machine now
/// * `context` - The user context of the admin, who migrated the virtual_machine
///
/// # Returns
/// A Result indicating success or an error
pub fn set_host_of_meta_virtual_machine(
    meta_virtual_machine_uuid: &Uuid,
    new_sakura_host_uuid: &Uuid,
    context: &UserContext,
) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::meta_virtual_machines::dsl::*;
    match diesel::update(
        meta_virtual_machines.filter(
            uuid.eq(meta_virtual_machine_uuid.to_string())
                .and(status.eq("ACTIVE")),
        ),
    )
    .set((
        sakura_host_uuid.eq(new_sakura_host_uuid.to_string()),
        updated_at.eq(Utc::now().to_rfc3339()),
        updated_by.eq(context.user_id.clone()),
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

/// Deletes all active meta virtual_machines of a sakura-host, which don't exist on the host
/// anymore.
///
/// It is used to sync the meta virtual_machines with the virtual_machines, which the host reports
/// at its registration, so it doesn't check any permissions.
///
/// # Arguments
/// * `host_uuid` - The UUID of the sakura-host
/// * `existing_uuids` - The UUIDs of the virtual_machines, which exist on the host
///
/// # Returns
/// A QueryResult containing the deleted meta virtual_machines
pub fn delete_missing_meta_virtual_machines_of_host(
    host_uuid: &Uuid,
    existing_uuids: &[Uuid],
) -> QueryResult<Vec<MetaVirtualMachineEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::meta_virtual_machines::dsl::*;

    let existing_uuids: Vec<String> = existing_uuids.iter().map(Uuid::to_string).collect();
    conn.transaction(|conn| {
        let missing = meta_virtual_machines
            .filter(
                sakura_host_uuid
                    .eq(host_uuid.to_string())
                    .and(status.eq("ACTIVE"))
                    .and(uuid.ne_all(&existing_uuids)),
            )
            .select(MetaVirtualMachineEntry::as_select())
            .load::<MetaVirtualMachineEntry>(conn)?;
        if missing.is_empty() {
            return Ok(missing);
        }

        let missing_uuids: Vec<String> = missing.iter().map(|vm| vm.uuid.to_string()).collect();
        diesel::update(meta_virtual_machines.filter(uuid.eq_any(&missing_uuids)))
            .set((
                status.eq("DELETED"),
                deleted_at.eq(Utc::now().to_rfc3339()),
                deleted_by.eq("HOST_INIT"),
            ))
            .execute(conn)?;
        Ok(missing)
    })
}

/// Deletes a meta virtual_machine from the database.
///
/// This function marks a meta virtual_machine as deleted after verifying that the user has permission to delete it.
/// It first checks if the virtual_machine exists and if the user has the necessary permissions.
///
/// # Arguments
/// * `meta_virtual_machine_uuid` - The UUID of the meta virtual_machine to delete
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A Result indicating success or an error
pub fn delete_meta_virtual_machine(
    meta_virtual_machine_uuid: &Uuid,
    context: &UserContext,
) -> Result<(), enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    // Verify the meta virtual_machine exists and the user has permission to delete it
    get_meta_virtual_machine(meta_virtual_machine_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::meta_virtual_machines::dsl::*;
    match diesel::update(
        meta_virtual_machines.filter(uuid.eq(meta_virtual_machine_uuid.to_string())),
    )
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

/// Deletes all meta virtual_machines from the database.
///
/// This function marks all active meta virtual_machines as deleted without checking permissions.
/// It's intended for system-level operations where permission checks are not required.
///
/// # Returns
/// A Result indicating success or an error
#[allow(dead_code)]
pub fn delete_all_meta_virtual_machine() -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::meta_virtual_machines::dsl::*;
    match diesel::update(meta_virtual_machines.filter(status.eq("ACTIVE")))
        .set((
            status.eq("DELETED"),
            deleted_at.eq(Utc::now().to_rfc3339()),
            deleted_by.eq("AINARI_START"),
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

/// Searches a virtual_machine by its UUID within all projects, also if it is already deleted.
///
/// It is used to check, if a predefined UUID is still free, before a virtual_machine with this
/// UUID is reserved.
///
/// # Arguments
/// * `virtual_machine_uuid` - The UUID of the virtual_machine
///
/// # Returns
/// A QueryResult containing project and status of the virtual_machine, or None, if the UUID was
/// never used
pub fn find_meta_virtual_machine(
    virtual_machine_uuid: &Uuid,
) -> QueryResult<Option<(String, String)>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::meta_virtual_machines::dsl::*;

    meta_virtual_machines
        .filter(uuid.eq(virtual_machine_uuid.to_string()))
        .select((project_id, status))
        .first::<(String, String)>(&mut *conn)
        .optional()
}

/// Removes the entry of a deleted virtual_machine, so its UUID can be used again, for example to
/// restore it from a backup. Active virtual_machines are never removed.
///
/// # Arguments
/// * `virtual_machine_uuid` - The UUID of the deleted virtual_machine
///
/// # Returns
/// A QueryResult with the number of removed entries
pub fn remove_deleted_meta_virtual_machine(virtual_machine_uuid: &Uuid) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::meta_virtual_machines::dsl::*;

    diesel::delete(
        meta_virtual_machines.filter(
            uuid.eq(virtual_machine_uuid.to_string())
                .and(status.eq("DELETED")),
        ),
    )
    .execute(&mut *conn)
}

/// Lists all active meta virtual machines of a project.
///
/// Unlike the listing for the users, it is done for the project of the path and not the one of
/// the token, so an admin can export any project.
///
/// # Arguments
/// * `project` - The ID of the project
///
/// # Returns
/// A QueryResult containing the meta virtual machines of the project
pub fn list_meta_virtual_machines_of_project(
    project: &str,
) -> QueryResult<Vec<MetaVirtualMachineEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::meta_virtual_machines::dsl::*;

    meta_virtual_machines
        .filter(status.eq("ACTIVE"))
        .filter(project_id.eq(project))
        .select(MetaVirtualMachineEntry::as_select())
        .load(&mut *conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ainari_common::enums::ProjectRole;
    use serial_test::serial;

    fn hard_delete_meta_virtual_machine(meta_virtual_machine_uuid: &Uuid) {
        use self::meta_virtual_machines::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(
            meta_virtual_machines.filter(uuid.eq(meta_virtual_machine_uuid.to_string())),
        )
        .execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_add_get_meta_virtual_machine() {
        let uuid1 = Uuid::new_v4();
        let name = "test-virtual_machine".to_string();
        let sakura_host_uuid1 = Uuid::new_v4();
        let proxy_uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let meta_virtual_machine = MetaVirtualMachineEntry {
            uuid: uuid1,
            name: name.clone(),
            sakura_host_uuid: sakura_host_uuid1,
            proxy_uuid: proxy_uuid1,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
            number_of_cores: 2,
            memory_size: 2048,
            disk_size: 10,
        };

        hard_delete_meta_virtual_machine(&uuid1);

        add_meta_virtual_machine(meta_virtual_machine.clone()).unwrap();
        match get_meta_virtual_machine(&uuid1, &context) {
            Ok(retrieved_meta_virtual_machine) => {
                assert_eq!(
                    retrieved_meta_virtual_machine.uuid,
                    meta_virtual_machine.uuid
                );
                assert_eq!(retrieved_meta_virtual_machine.number_of_cores, 2);
                assert_eq!(retrieved_meta_virtual_machine.memory_size, 2048);
                assert_eq!(retrieved_meta_virtual_machine.disk_size, 10);
                assert_eq!(
                    retrieved_meta_virtual_machine.proxy_uuid,
                    meta_virtual_machine.proxy_uuid
                );
                assert_eq!(
                    retrieved_meta_virtual_machine.sakura_host_uuid,
                    meta_virtual_machine.sakura_host_uuid
                );
                assert_eq!(
                    retrieved_meta_virtual_machine.owner_id,
                    meta_virtual_machine.owner_id
                );
                assert_eq!(
                    retrieved_meta_virtual_machine.project_id,
                    meta_virtual_machine.project_id
                );
                assert_eq!(
                    retrieved_meta_virtual_machine.status,
                    meta_virtual_machine.status
                );
                assert_eq!(
                    retrieved_meta_virtual_machine.created_by,
                    meta_virtual_machine.created_by
                );
                assert_eq!(
                    retrieved_meta_virtual_machine.updated_by,
                    meta_virtual_machine.updated_by
                );
                assert_eq!(
                    retrieved_meta_virtual_machine.deleted_at,
                    meta_virtual_machine.deleted_at
                );
                assert_eq!(
                    retrieved_meta_virtual_machine.deleted_by,
                    meta_virtual_machine.deleted_by
                );
            }
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        hard_delete_meta_virtual_machine(&uuid1);
    }

    #[test]
    #[serial]
    fn test_list_meta_virtual_machines() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let name = "test-virtual_machine".to_string();
        let sakura_host_uuid1 = Uuid::new_v4();
        let proxy_uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let meta_virtual_machine1 = MetaVirtualMachineEntry {
            uuid: uuid1,
            name: name.clone(),
            sakura_host_uuid: sakura_host_uuid1,
            proxy_uuid: proxy_uuid1,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
            number_of_cores: 2,
            memory_size: 2048,
            disk_size: 10,
        };

        let meta_virtual_machine2 = MetaVirtualMachineEntry {
            uuid: uuid2,
            name: name.clone(),
            sakura_host_uuid: sakura_host_uuid1,
            proxy_uuid: proxy_uuid1,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "DELETED".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
            number_of_cores: 2,
            memory_size: 2048,
            disk_size: 10,
        };

        hard_delete_meta_virtual_machine(&uuid1);
        hard_delete_meta_virtual_machine(&uuid2);

        add_meta_virtual_machine(meta_virtual_machine1).unwrap();
        add_meta_virtual_machine(meta_virtual_machine2).unwrap();
        let meta_virtual_machines = list_meta_virtual_machines(&context).unwrap();
        assert_eq!(meta_virtual_machines.len(), 1);
        hard_delete_meta_virtual_machine(&uuid1);
        hard_delete_meta_virtual_machine(&uuid2);
    }

    #[test]
    #[serial]
    fn test_set_host_of_meta_virtual_machine() {
        let uuid1 = Uuid::new_v4();
        let old_host_uuid = Uuid::new_v4();
        let new_host_uuid = Uuid::new_v4();
        let context = UserContext {
            token: "".to_string(),
            user_id: "admin".to_string(),
            project_id: "admin-project".to_string(),
            is_admin: true.to_string(),
            project_role: ProjectRole::Admin.to_string(),
        };

        hard_delete_meta_virtual_machine(&uuid1);
        add_meta_virtual_machine(MetaVirtualMachineEntry {
            uuid: uuid1,
            name: "test-virtual_machine".to_string(),
            sakura_host_uuid: old_host_uuid,
            proxy_uuid: Uuid::new_v4(),
            owner_id: "test-user".to_string(),
            project_id: "test-project".to_string(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "test-user".to_string(),
            updated_at: Utc::now(),
            updated_by: "test-user".to_string(),
            deleted_at: None,
            deleted_by: None,
            number_of_cores: 2,
            memory_size: 2048,
            disk_size: 10,
        })
        .unwrap();

        assert!(set_host_of_meta_virtual_machine(&uuid1, &new_host_uuid, &context).is_ok());
        let Ok(retrieved) = get_meta_virtual_machine(&uuid1, &context) else {
            panic!("meta virtual_machine not found");
        };
        assert_eq!(retrieved.sakura_host_uuid, new_host_uuid);
        // the owner stays the same, only the admin is noted as the last one, who changed it
        assert_eq!(retrieved.owner_id, "test-user");
        assert_eq!(retrieved.updated_by, "admin");

        // a deleted virtual_machine is not moved anymore
        let deleted = delete_missing_meta_virtual_machines_of_host(&new_host_uuid, &[]).unwrap();
        assert_eq!(deleted.len(), 1);
        assert!(matches!(
            set_host_of_meta_virtual_machine(&uuid1, &old_host_uuid, &context),
            Err(enums::DbError::NotFound)
        ));

        hard_delete_meta_virtual_machine(&uuid1);
    }

    #[test]
    #[serial]
    fn test_delete_missing_meta_virtual_machines_of_host() {
        let (kept, missing, other) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let (host_uuid, other_host_uuid) = (Uuid::new_v4(), Uuid::new_v4());
        let entry = |vm_uuid: Uuid, vm_host_uuid: Uuid| MetaVirtualMachineEntry {
            uuid: vm_uuid,
            name: "test-virtual_machine".to_string(),
            sakura_host_uuid: vm_host_uuid,
            proxy_uuid: Uuid::new_v4(),
            owner_id: "test-user".to_string(),
            project_id: "test-project".to_string(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "test-user".to_string(),
            updated_at: Utc::now(),
            updated_by: "test-user".to_string(),
            deleted_at: None,
            deleted_by: None,
            number_of_cores: 2,
            memory_size: 2048,
            disk_size: 10,
        };
        for (vm_uuid, vm_host_uuid) in [
            (kept, host_uuid),
            (missing, host_uuid),
            (other, other_host_uuid),
        ] {
            hard_delete_meta_virtual_machine(&vm_uuid);
            add_meta_virtual_machine(entry(vm_uuid, vm_host_uuid)).unwrap();
        }

        // only the virtual_machine of the host, which is not reported by the host, is deleted
        let deleted = delete_missing_meta_virtual_machines_of_host(&host_uuid, &[kept]).unwrap();
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0].uuid, missing);
        assert_eq!(deleted[0].number_of_cores, 2);
        assert_eq!(
            find_meta_virtual_machine(&missing).unwrap().unwrap().1,
            "DELETED"
        );
        assert_eq!(
            find_meta_virtual_machine(&kept).unwrap().unwrap().1,
            "ACTIVE"
        );
        assert_eq!(
            find_meta_virtual_machine(&other).unwrap().unwrap().1,
            "ACTIVE"
        );

        // an already deleted virtual_machine is not returned again
        let deleted = delete_missing_meta_virtual_machines_of_host(&host_uuid, &[kept]).unwrap();
        assert!(deleted.is_empty());

        for vm_uuid in [kept, missing, other] {
            hard_delete_meta_virtual_machine(&vm_uuid);
        }
    }

    #[test]
    #[serial]
    fn test_delete_meta_virtual_machine() {
        let uuid1 = Uuid::new_v4();
        let name = "test-virtual_machine".to_string();
        let sakura_host_uuid1 = Uuid::new_v4();
        let proxy_uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let meta_virtual_machine = MetaVirtualMachineEntry {
            uuid: uuid1,
            name: name.clone(),
            sakura_host_uuid: sakura_host_uuid1,
            proxy_uuid: proxy_uuid1,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
            number_of_cores: 2,
            memory_size: 2048,
            disk_size: 10,
        };

        hard_delete_meta_virtual_machine(&uuid1);

        add_meta_virtual_machine(meta_virtual_machine).unwrap();
        let _ = delete_meta_virtual_machine(&uuid1, &context);
        let result = get_meta_virtual_machine(&uuid1, &context);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_count_meta_virtual_machines() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let name = "test-virtual_machine".to_string();
        let sakura_host_uuid1 = Uuid::new_v4();
        let proxy_uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let meta_virtual_machine1 = MetaVirtualMachineEntry {
            uuid: uuid1,
            name: name.clone(),
            sakura_host_uuid: sakura_host_uuid1,
            proxy_uuid: proxy_uuid1,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
            number_of_cores: 2,
            memory_size: 2048,
            disk_size: 10,
        };

        let meta_virtual_machine2 = MetaVirtualMachineEntry {
            uuid: uuid2,
            name: name.clone(),
            sakura_host_uuid: sakura_host_uuid1,
            proxy_uuid: proxy_uuid1,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
            number_of_cores: 2,
            memory_size: 2048,
            disk_size: 10,
        };

        let meta_virtual_machine3 = MetaVirtualMachineEntry {
            uuid: uuid3,
            name: name.clone(),
            sakura_host_uuid: sakura_host_uuid1,
            proxy_uuid: proxy_uuid1,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
            number_of_cores: 2,
            memory_size: 2048,
            disk_size: 10,
        };

        hard_delete_meta_virtual_machine(&uuid1);
        hard_delete_meta_virtual_machine(&uuid2);
        hard_delete_meta_virtual_machine(&uuid3);

        add_meta_virtual_machine(meta_virtual_machine1).unwrap();
        add_meta_virtual_machine(meta_virtual_machine2).unwrap();
        add_meta_virtual_machine(meta_virtual_machine3).unwrap();

        let number = count_meta_virtual_machines(&context).unwrap();
        assert_eq!(number, 3);

        hard_delete_meta_virtual_machine(&uuid1);
        hard_delete_meta_virtual_machine(&uuid2);
        hard_delete_meta_virtual_machine(&uuid3);
    }

    #[test]
    #[serial]
    fn test_meta_virtual_machines_permissions() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let name = "test-virtual_machine".to_string();
        let sakura_host_uuid1 = Uuid::new_v4();
        let proxy_uuid1 = Uuid::new_v4();

        let meta_virtual_machine1 = MetaVirtualMachineEntry {
            uuid: uuid1,
            name: name.clone(),
            sakura_host_uuid: sakura_host_uuid1,
            proxy_uuid: proxy_uuid1,
            owner_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
            number_of_cores: 2,
            memory_size: 2048,
            disk_size: 10,
        };

        let meta_virtual_machine2 = MetaVirtualMachineEntry {
            uuid: uuid2,
            name: name.clone(),
            sakura_host_uuid: sakura_host_uuid1,
            proxy_uuid: proxy_uuid1,
            owner_id: "test-user-43".to_string(),
            project_id: "test_permissions_1".to_string(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
            number_of_cores: 2,
            memory_size: 2048,
            disk_size: 10,
        };

        let meta_virtual_machine3 = MetaVirtualMachineEntry {
            uuid: uuid3,
            name: name.clone(),
            sakura_host_uuid: sakura_host_uuid1,
            proxy_uuid: proxy_uuid1,
            owner_id: "test-user-44".to_string(),
            project_id: "test_permissions_2".to_string(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
            number_of_cores: 2,
            memory_size: 2048,
            disk_size: 10,
        };

        hard_delete_meta_virtual_machine(&uuid1);
        hard_delete_meta_virtual_machine(&uuid2);
        hard_delete_meta_virtual_machine(&uuid3);

        add_meta_virtual_machine(meta_virtual_machine1).unwrap();
        add_meta_virtual_machine(meta_virtual_machine2).unwrap();
        add_meta_virtual_machine(meta_virtual_machine3).unwrap();

        // list-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        let meta_virtual_machines = list_meta_virtual_machines(&context).unwrap();
        // members see all entries of their project, also the ones of other users
        assert_eq!(meta_virtual_machines.len(), 2);

        // list-test project-admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Admin.to_string(),
        };
        let meta_virtual_machines = list_meta_virtual_machines(&context).unwrap();
        assert_eq!(meta_virtual_machines.len(), 2);

        // list-test admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: true.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        let meta_virtual_machines = list_meta_virtual_machines(&context).unwrap();
        assert_eq!(meta_virtual_machines.len(), 3);

        // get-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        match get_meta_virtual_machine(&uuid1, &context) {
            Ok(retrieved_meta_virtual_machine) => {
                assert_eq!(retrieved_meta_virtual_machine.uuid, uuid1);
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
        if get_meta_virtual_machine(&uuid3, &context).is_ok() {
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
        if delete_meta_virtual_machine(&uuid3, &context).is_ok() {
            assert_eq!(true, false);
        };

        hard_delete_meta_virtual_machine(&uuid1);
        hard_delete_meta_virtual_machine(&uuid2);
        hard_delete_meta_virtual_machine(&uuid3);
    }
}
