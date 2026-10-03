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

use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;
use ainari_common::objects::*;

// Define the schema for vm_types table
table! {
    vm_types (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        number_of_cores -> Integer,
        amount_of_memory -> BigInt,
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

/// Represents an entry in the vm_types table.
/// This struct contains all the fields required to create, query, and update VM-type records.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = vm_types)]
pub struct VmTypeEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    pub name: String,
    pub number_of_cores: i32,
    pub amount_of_memory: i64,
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

/// Adds a new VM-type to the database.
///
/// This function creates a new VmTypeEntry with the provided parameters and inserts it into the database.
/// The status is set to "ACTIVE" and timestamps are set to the current time.
///
/// # Arguments
/// * `vm_type_uuid` - The unique identifier for the VM-type
/// * `vm_type_name` - The name of the VM-type
/// * `number_of_cores` - The number of CPU-cores of the VM-type
/// * `amount_of_memory` - The amount of memory of the VM-type
/// * `context` - The user context containing information about the user and project
///
/// # Returns
/// A QueryResult indicating the number of rows affected
pub fn add_new_vm_type(
    vm_type_uuid: &Uuid,
    vm_type_name: &str,
    number_of_cores: i32,
    amount_of_memory: i64,
    context: &UserContext,
) -> QueryResult<usize> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::permission_denied_error());
    }

    let vm_type = VmTypeEntry {
        uuid: *vm_type_uuid,
        name: vm_type_name.to_string().clone(),
        number_of_cores,
        amount_of_memory,
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

    add_vm_type(vm_type)
}

/// Adds a VM-type to the database.
///
/// This is a helper function that performs the actual insertion of a VmTypeEntry into the database.
///
/// # Arguments
/// * `vm_type` - The VmTypeEntry to be inserted
///
/// # Returns
/// A QueryResult indicating the number of rows affected
pub fn add_vm_type(vm_type: VmTypeEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::vm_types::dsl::*;
    diesel::insert_into(vm_types)
        .values(vm_type)
        .execute(&mut *conn)
}

/// Retrieves a VM-type from the database.
///
/// VM-types are global and not bound to a project, so every user can read every active VM-type.
///
/// # Arguments
/// * `vm_type_uuid` - The UUID of the VM-type to retrieve
///
/// # Returns
/// A Result containing the VmTypeEntry if found, or a DbError if not found or an error occurs
pub fn get_vm_type(vm_type_uuid: &Uuid) -> Result<VmTypeEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::vm_types::dsl::*;

    match vm_types
        .filter(uuid.eq(vm_type_uuid.to_string()).and(status.eq("ACTIVE")))
        .select(VmTypeEntry::as_select())
        .first::<VmTypeEntry>(&mut *conn)
    {
        Ok(vm_type) => Ok(vm_type),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Lists all active VM-types.
///
/// VM-types are global and not bound to a project, so every user can read every active VM-type.
///
/// # Returns
/// A QueryResult containing a vector of VmTypeEntry objects
pub fn list_vm_types() -> QueryResult<Vec<VmTypeEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::vm_types::dsl::*;

    vm_types
        .filter(status.eq("ACTIVE"))
        .select(VmTypeEntry::as_select())
        .load(&mut *conn)
}

/// Updates the values of a VM-type in the database.
///
/// The name, the number of cores and the amount of memory are always overwritten, so the caller
/// has to provide the current values for the ones, which should not change.
///
/// # Arguments
/// * `vm_type_uuid` - The UUID of the VM-type to update
/// * `new_name` - The new name of the VM-type
/// * `new_number_of_cores` - The new number of CPU-cores of the VM-type
/// * `new_amount_of_memory` - The new amount of memory of the VM-type
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A Result indicating success or an error
pub fn update_vm_type(
    vm_type_uuid: &Uuid,
    new_name: &str,
    new_number_of_cores: i32,
    new_amount_of_memory: i64,
    context: &UserContext,
) -> Result<(), enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    // Verify the VM-type exists
    get_vm_type(vm_type_uuid)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::vm_types::dsl::*;
    match diesel::update(vm_types.filter(uuid.eq(vm_type_uuid.to_string())))
        .set((
            name.eq(new_name),
            number_of_cores.eq(new_number_of_cores),
            amount_of_memory.eq(new_amount_of_memory),
            updated_at.eq(Utc::now().to_rfc3339()),
            updated_by.eq(context.user_id.clone()),
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

/// Counts the number of vm_types of a whole project.
///
/// Unlike `count_vm_types`, the vm_types of all users of the project are counted, because the quota,
/// which is checked with this number, belongs to the project.
///
/// # Arguments
///
/// * `project` - The ID of the project, which is counted
///
/// # Returns
///
/// A QueryResult containing the count of vm_types as an i64
#[allow(dead_code)]
pub fn count_vm_types_of_project(project: &str) -> QueryResult<i64> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::vm_types::dsl::*;

    vm_types
        .filter(status.eq("ACTIVE"))
        .filter(project_id.eq(project))
        .select(count_star())
        .first::<i64>(&mut *conn)
}

/// Force deletes a VM-type from the database.
///
/// This function marks a VM-type as deleted without checking permissions.
/// It's intended for system-level operations where permission checks are not required.
///
/// # Arguments
/// * `vm_type_uuid` - The UUID of the VM-type to delete
///
/// # Returns
/// A Result indicating success or an error
#[allow(dead_code)]
pub fn force_delete_vm_type(vm_type_uuid: &Uuid) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::vm_types::dsl::*;
    match diesel::update(vm_types.filter(uuid.eq(vm_type_uuid.to_string())))
        .set((
            status.eq("DELETED"),
            deleted_at.eq(Utc::now().to_rfc3339()),
            deleted_by.eq("HOST_INIT"),
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

/// Deletes a VM-type from the database.
///
/// This function marks a VM-type as deleted after verifying that the user has permission to delete it.
/// It first checks if the VM-type exists and if the user has the necessary permissions.
///
/// # Arguments
/// * `vm_type_uuid` - The UUID of the VM-type to delete
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A Result indicating success or an error
pub fn delete_vm_type(vm_type_uuid: &Uuid, context: &UserContext) -> Result<(), enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    // Verify the VM-type exists
    get_vm_type(vm_type_uuid)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::vm_types::dsl::*;
    match diesel::update(vm_types.filter(uuid.eq(vm_type_uuid.to_string())))
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

/// Deletes all VM-types from the database.
///
/// This function marks all active VM-types as deleted without checking permissions.
/// It's intended for system-level operations where permission checks are not required.
///
/// # Returns
/// A Result indicating success or an error
#[allow(dead_code)]
pub fn delete_all_vm_type() -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::vm_types::dsl::*;
    match diesel::update(vm_types.filter(status.eq("ACTIVE")))
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

#[cfg(test)]
mod tests {
    use super::*;
    use ainari_common::enums::ProjectRole;
    use serial_test::serial;

    fn hard_delete_vm_type(vm_type_uuid: &Uuid) {
        use self::vm_types::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ =
            diesel::delete(vm_types.filter(uuid.eq(vm_type_uuid.to_string()))).execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_add_get_vm_type() {
        let uuid1 = Uuid::new_v4();
        let name = "test-vm-type".to_string();
        let number_of_cores = 4;
        let amount_of_memory = 8192;

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();

        let vm_type = VmTypeEntry {
            uuid: uuid1,
            name: name.clone(),
            number_of_cores,
            amount_of_memory,
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

        hard_delete_vm_type(&uuid1);

        add_vm_type(vm_type.clone()).unwrap();
        match get_vm_type(&uuid1) {
            Ok(retrieved_vm_type) => {
                assert_eq!(retrieved_vm_type.uuid, vm_type.uuid);
                assert_eq!(retrieved_vm_type.number_of_cores, vm_type.number_of_cores);
                assert_eq!(retrieved_vm_type.amount_of_memory, vm_type.amount_of_memory);
                assert_eq!(retrieved_vm_type.owner_id, vm_type.owner_id);
                assert_eq!(retrieved_vm_type.project_id, vm_type.project_id);
                assert_eq!(retrieved_vm_type.status, vm_type.status);
                assert_eq!(retrieved_vm_type.created_by, vm_type.created_by);
                assert_eq!(retrieved_vm_type.updated_by, vm_type.updated_by);
                assert_eq!(retrieved_vm_type.deleted_at, vm_type.deleted_at);
                assert_eq!(retrieved_vm_type.deleted_by, vm_type.deleted_by);
            }
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        hard_delete_vm_type(&uuid1);
    }

    #[test]
    #[serial]
    fn test_list_vm_types() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let name = "test-vm-type".to_string();
        let number_of_cores = 4;
        let amount_of_memory = 8192;

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();

        let vm_type1 = VmTypeEntry {
            uuid: uuid1,
            name: name.clone(),
            number_of_cores,
            amount_of_memory,
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

        let vm_type2 = VmTypeEntry {
            uuid: uuid2,
            name: name.clone(),
            number_of_cores,
            amount_of_memory,
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

        hard_delete_vm_type(&uuid1);
        hard_delete_vm_type(&uuid2);

        add_vm_type(vm_type1).unwrap();
        add_vm_type(vm_type2).unwrap();
        // deleted entries are not listed
        let vm_types = list_vm_types().unwrap();
        assert!(vm_types.iter().any(|v| v.uuid == uuid1));
        assert!(!vm_types.iter().any(|v| v.uuid == uuid2));
        hard_delete_vm_type(&uuid1);
        hard_delete_vm_type(&uuid2);
    }

    #[test]
    #[serial]
    fn test_delete_vm_type() {
        let uuid1 = Uuid::new_v4();
        let name = "test-vm-type".to_string();
        let number_of_cores = 4;
        let amount_of_memory = 8192;

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let vm_type = VmTypeEntry {
            uuid: uuid1,
            name: name.clone(),
            number_of_cores,
            amount_of_memory,
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

        hard_delete_vm_type(&uuid1);

        add_vm_type(vm_type.clone()).unwrap();
        let _ = delete_vm_type(&uuid1, &context);
        let result = get_vm_type(&uuid1);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_count_vm_types_of_project() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let name = "test-vm-type".to_string();
        let number_of_cores = 4;
        let amount_of_memory = 8192;

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let vm_type1 = VmTypeEntry {
            uuid: uuid1,
            name: name.clone(),
            number_of_cores,
            amount_of_memory,
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

        let vm_type2 = VmTypeEntry {
            uuid: uuid2,
            name: name.clone(),
            number_of_cores,
            amount_of_memory,
            owner_id: "other-user".to_string(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let vm_type3 = VmTypeEntry {
            uuid: uuid3,
            name: name.clone(),
            number_of_cores,
            amount_of_memory,
            owner_id: owner_id.clone(),
            project_id: "other-project".to_string(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_vm_type(&uuid1);
        hard_delete_vm_type(&uuid2);
        hard_delete_vm_type(&uuid3);

        add_vm_type(vm_type1).unwrap();
        add_vm_type(vm_type2).unwrap();
        add_vm_type(vm_type3).unwrap();

        // the vm_types of all users of the project are counted, but not the ones of other projects
        let number = count_vm_types_of_project(&context.project_id).unwrap();
        assert_eq!(number, 2);

        hard_delete_vm_type(&uuid1);
        hard_delete_vm_type(&uuid2);
        hard_delete_vm_type(&uuid3);
    }

    #[test]
    #[serial]
    fn test_vm_types_visibility_and_update() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let name = "test-vm-type".to_string();
        let number_of_cores = 4;
        let amount_of_memory = 8192;

        let vm_type1 = VmTypeEntry {
            uuid: uuid1,
            name: name.clone(),
            number_of_cores,
            amount_of_memory,
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

        let vm_type2 = VmTypeEntry {
            uuid: uuid2,
            name: name.clone(),
            number_of_cores,
            amount_of_memory,
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

        let vm_type3 = VmTypeEntry {
            uuid: uuid3,
            name: name.clone(),
            number_of_cores,
            amount_of_memory,
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

        hard_delete_vm_type(&uuid1);
        hard_delete_vm_type(&uuid2);
        hard_delete_vm_type(&uuid3);

        add_vm_type(vm_type1).unwrap();
        add_vm_type(vm_type2).unwrap();
        add_vm_type(vm_type3).unwrap();

        // VM-types are global, so normal users see the ones of all projects
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        let vm_types = list_vm_types().unwrap();
        for vm_type_uuid in [uuid1, uuid2, uuid3] {
            assert!(vm_types.iter().any(|v| v.uuid == vm_type_uuid));
        }
        assert_eq!(
            get_vm_type(&uuid3).ok().expect("vm-type not found").uuid,
            uuid3
        );

        // update-test, which changes all values and the update-information
        assert!(update_vm_type(&uuid1, "updated-vm-type", 8, 16384, &context).is_ok());
        let updated = get_vm_type(&uuid1).ok().expect("vm-type not found");
        assert_eq!(updated.name, "updated-vm-type");
        assert_eq!(updated.number_of_cores, 8);
        assert_eq!(updated.amount_of_memory, 16384);
        assert_eq!(updated.updated_by, "test-user-42");

        // update-test with unknown uuid
        let result = update_vm_type(&Uuid::new_v4(), "unknown", 1, 1, &context);
        assert!(matches!(result, Err(enums::DbError::NotFound)));

        // observers are not allowed to update or delete
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Observer.to_string(),
        };
        let result = update_vm_type(&uuid2, "observer-vm-type", 1, 1, &context);
        assert!(matches!(result, Err(enums::DbError::PermissionDenied)));
        let result = delete_vm_type(&uuid2, &context);
        assert!(matches!(result, Err(enums::DbError::PermissionDenied)));
        assert_eq!(
            get_vm_type(&uuid2).ok().expect("vm-type not found").name,
            name
        );

        hard_delete_vm_type(&uuid1);
        hard_delete_vm_type(&uuid2);
        hard_delete_vm_type(&uuid3);
    }
}
