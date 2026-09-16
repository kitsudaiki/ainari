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
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use uuid::Uuid;

use crate::database::db_handle;

use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;
use ainari_common::objects::*;

// Define the schema for the virtual_machines table
table! {
    virtual_machines (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        is_created -> Bool,
        number_of_cores -> BigInt,
        size_of_memory -> BigInt,
        size_of_disk -> BigInt,
        image_uuid -> Varchar,
        seed_uuid -> Varchar,
        public_key_uuid -> Varchar,
        ip_addresses -> Varchar,
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

/// Represents a single entry in the virtual_machines table
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = virtual_machines)]
pub struct VirtualMachineEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    pub name: String,
    pub is_created: bool,
    pub number_of_cores: i64,
    pub size_of_memory: i64,
    pub size_of_disk: i64,
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub image_uuid: Uuid,
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub seed_uuid: Uuid,
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub public_key_uuid: Uuid,
    #[diesel(serialize_as = DbVecString, deserialize_as = DbVecString)]
    pub ip_addresses: Vec<String>,
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

/// Initializes the virtual_machines table in the database if it doesn't already exist
///
/// # Returns
/// * `Ok(())` if the table was created or already exists
/// * An error if there was a problem creating the table
pub fn init_virtual_machine_table() -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS virtual_machines (
        uuid VARCHAR(40) PRIMARY KEY,
        name VARCHAR(256),
        is_created BOOL,
        number_of_cores INTEGER,
        size_of_memory INTEGER,
        size_of_disk INTEGER,
        image_uuid VARCHAR(40),
        seed_uuid VARCHAR(40),
        public_key_uuid VARCHAR(40),
        ip_addresses TEXT,
        owner_id VARCHAR(256),
        project_id VARCHAR(256),
        status VARCHAR(8),
        created_at VARCHAR(64),
        created_by VARCHAR(256),
        updated_at VARCHAR(64),
        updated_by VARCHAR(256),
        deleted_at VARCHAR(64),
        deleted_by VARCHAR(256)
    );",
    )?;

    Ok(())
}

/// Adds a new virtual_machine to the database with the provided information
///
/// # Arguments
/// * `virtual_machine_uuid` - Unique identifier for the new virtual_machine
/// * `virtual_machine_name` - Name for the new virtual_machine
/// * `virtual_machine_template` - Template content for the new virtual_machine
/// * `inputs` - Vector of input specifications
/// * `outputs` - Vector of output specifications
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(usize)` with the number of rows inserted on success
/// * `Err` with an appropriate error on failure
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub fn add_new_virtual_machine(
    virtual_machine_uuid: &Uuid,
    virtual_machine_name: &str,
    number_of_cores: i64,
    size_of_memory: i64,
    size_of_disk: i64,
    image_uuid: &Uuid,
    seed_uuid: &Uuid,
    public_key_uuid: &Uuid,
    ip_addresses: &[String],
    context: &UserContext,
) -> QueryResult<usize> {
    // Create the new virtual_machine entry
    let virtual_machine = VirtualMachineEntry {
        uuid: *virtual_machine_uuid,
        name: virtual_machine_name.to_owned(),
        is_created: false,
        number_of_cores,
        size_of_memory,
        size_of_disk,
        image_uuid: *image_uuid,
        seed_uuid: *seed_uuid,
        public_key_uuid: *public_key_uuid,
        ip_addresses: ip_addresses.to_vec(),
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

    // Insert the virtual_machine into the database
    add_virtual_machine(virtual_machine)
}

/// Adds a virtual_machine entry to the database
///
/// # Arguments
/// * `virtual_machine` - The virtual_machine entry to insert
///
/// # Returns
/// * `Ok(usize)` with the number of rows inserted on success
/// * `Err` with an appropriate error on failure
#[allow(dead_code)]
pub fn add_virtual_machine(virtual_machine: VirtualMachineEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::virtual_machines::dsl::*;
    diesel::insert_into(virtual_machines)
        .values(virtual_machine)
        .execute(&mut *conn)
}

/// Retrieves a specific virtual_machine from the database
///
/// # Arguments
/// * `virtual_machine_uuid` - Unique identifier of the virtual_machine to retrieve
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(VirtualMachineEntry)` with the virtual_machine on success
/// * `Err(enums::DbError)` with an appropriate error on failure
pub fn get_virtual_machine(
    virtual_machine_uuid: &Uuid,
    context: &UserContext,
) -> Result<VirtualMachineEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::virtual_machines::dsl::*;

    // Build the query with appropriate filters based on user permissions
    let mut query = virtual_machines
        .filter(
            uuid.eq(virtual_machine_uuid.to_string())
                .and(status.eq("ACTIVE")),
        )
        .into_boxed();

    // Apply project and ownership filters for non-admin users
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    // Execute the query and return the result
    match query
        .select(VirtualMachineEntry::as_select())
        .first::<VirtualMachineEntry>(&mut *conn)
    {
        Ok(virtual_machine) => Ok(virtual_machine),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Lists all deleted virtual_machines from the database
///
/// # Returns
/// * `Ok(Vec<VirtualMachineEntry>)` with the list of deleted virtual_machines on success
/// * `Err` with an appropriate error on failure
pub fn list_deleted_virtual_machines() -> QueryResult<Vec<VirtualMachineEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::virtual_machines::dsl::*;

    // Create a query to find all virtual_machines with "DELETED" status
    let query = virtual_machines.filter(status.eq("DELETED")).into_boxed();

    // Execute the query and return the results
    query
        .select(VirtualMachineEntry::as_select())
        .load(&mut *conn)
}

/// Lists all active virtual_machines from the database, applying appropriate filters based on user permissions
///
/// # Arguments
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(Vec<VirtualMachineEntry>)` with the list of virtual_machines on success
/// * `Err` with an appropriate error on failure
pub fn list_virtual_machines(context: &UserContext) -> QueryResult<Vec<VirtualMachineEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::virtual_machines::dsl::*;

    // Build the query with appropriate filters based on user permissions
    let mut query = virtual_machines.filter(status.eq("ACTIVE")).into_boxed();

    // Apply project and ownership filters for non-admin users
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    // Execute the query and return the results
    query
        .select(VirtualMachineEntry::as_select())
        .load(&mut *conn)
}

/// Marks a specific virtual_machine as deleted in the database
///
/// # Arguments
/// * `virtual_machine_uuid` - Unique identifier of the virtual_machine to delete
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(enums::DbError)` with an appropriate error on failure
pub fn delete_virtual_machine(
    virtual_machine_uuid: &Uuid,
    context: &UserContext,
) -> Result<(), enums::DbError> {
    // First verify that the virtual_machine exists and the user has permission to delete it
    get_virtual_machine(virtual_machine_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::virtual_machines::dsl::*;

    // Update the virtual_machine's status to "DELETED" and set the deletion timestamp and user
    match diesel::update(virtual_machines.filter(uuid.eq(virtual_machine_uuid.to_string())))
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

/// Marks all active virtual_machines as deleted in the database
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(enums::DbError)` with an appropriate error on failure
pub fn delete_all_virtual_machine() -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::virtual_machines::dsl::*;

    // Update all active virtual_machines to have "DELETED" status with a system user as the deleter
    match diesel::update(virtual_machines.filter(status.eq("ACTIVE")))
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
    use serial_test::serial;

    fn hard_delete_virtual_machine(virtual_machine_uuid: &Uuid) {
        use self::virtual_machines::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(virtual_machines.filter(uuid.eq(virtual_machine_uuid.to_string())))
            .execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_add_get_virtual_machine() {
        let _ = init_virtual_machine_table();
        let uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let virtual_machine = VirtualMachineEntry {
            uuid: uuid1,
            name: "Alice".to_string(),
            is_created: false,
            number_of_cores: 2,
            size_of_memory: 4096,
            size_of_disk: 1024,
            image_uuid: Uuid::new_v4(),
            seed_uuid: Uuid::new_v4(),
            public_key_uuid: Uuid::new_v4(),
            ip_addresses: vec!["192.168.1.1".to_string()],
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

        hard_delete_virtual_machine(&uuid1);

        add_virtual_machine(virtual_machine.clone()).unwrap();
        match get_virtual_machine(&uuid1, &context) {
            Ok(retrieved_virtual_machine) => {
                assert_eq!(retrieved_virtual_machine.uuid, virtual_machine.uuid);
                assert_eq!(retrieved_virtual_machine.name, virtual_machine.name);
                assert_eq!(
                    retrieved_virtual_machine.is_created,
                    virtual_machine.is_created
                );
                assert_eq!(
                    retrieved_virtual_machine.number_of_cores,
                    virtual_machine.number_of_cores
                );
                assert_eq!(
                    retrieved_virtual_machine.size_of_memory,
                    virtual_machine.size_of_memory
                );
                assert_eq!(
                    retrieved_virtual_machine.size_of_disk,
                    virtual_machine.size_of_disk
                );
                assert_eq!(
                    retrieved_virtual_machine.image_uuid,
                    virtual_machine.image_uuid
                );
                assert_eq!(
                    retrieved_virtual_machine.seed_uuid,
                    virtual_machine.seed_uuid
                );
                assert_eq!(
                    retrieved_virtual_machine.public_key_uuid,
                    virtual_machine.public_key_uuid
                );
                assert_eq!(
                    retrieved_virtual_machine.ip_addresses,
                    virtual_machine.ip_addresses
                );
                assert_eq!(retrieved_virtual_machine.owner_id, virtual_machine.owner_id);
                assert_eq!(
                    retrieved_virtual_machine.project_id,
                    virtual_machine.project_id
                );
                assert_eq!(retrieved_virtual_machine.status, virtual_machine.status);
                assert_eq!(
                    retrieved_virtual_machine.created_by,
                    virtual_machine.created_by
                );
                assert_eq!(
                    retrieved_virtual_machine.updated_by,
                    virtual_machine.updated_by
                );
                assert_eq!(
                    retrieved_virtual_machine.deleted_at,
                    virtual_machine.deleted_at
                );
                assert_eq!(
                    retrieved_virtual_machine.deleted_by,
                    virtual_machine.deleted_by
                );
            }
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        hard_delete_virtual_machine(&uuid1);
    }

    #[test]
    #[serial]
    fn test_list_virtual_machines() {
        let _ = init_virtual_machine_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let virtual_machine1 = VirtualMachineEntry {
            uuid: uuid1,
            name: "Alice".to_string(),
            is_created: false,
            number_of_cores: 2,
            size_of_memory: 4096,
            size_of_disk: 1024,
            image_uuid: Uuid::new_v4(),
            seed_uuid: Uuid::new_v4(),
            public_key_uuid: Uuid::new_v4(),
            ip_addresses: vec!["192.168.1.1".to_string()],
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

        let virtual_machine2 = VirtualMachineEntry {
            uuid: uuid2,
            name: "Bob".to_string(),
            is_created: false,
            number_of_cores: 2,
            size_of_memory: 4096,
            size_of_disk: 1024,
            image_uuid: Uuid::new_v4(),
            seed_uuid: Uuid::new_v4(),
            public_key_uuid: Uuid::new_v4(),
            ip_addresses: vec!["192.168.1.1".to_string()],
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "DELETED".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: Some(Utc::now()),
            deleted_by: Some("admin".to_string()),
        };

        hard_delete_virtual_machine(&uuid1);
        hard_delete_virtual_machine(&uuid2);

        add_virtual_machine(virtual_machine1).unwrap();
        add_virtual_machine(virtual_machine2).unwrap();
        let virtual_machines = list_virtual_machines(&context).unwrap();
        assert_eq!(virtual_machines.len(), 1);
        hard_delete_virtual_machine(&uuid1);
        hard_delete_virtual_machine(&uuid2);
    }

    #[test]
    #[serial]
    fn test_delete_virtual_machine() {
        let _ = init_virtual_machine_table();
        let uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let virtual_machine = VirtualMachineEntry {
            uuid: uuid1,
            name: "Alice".to_string(),
            is_created: false,
            number_of_cores: 2,
            size_of_memory: 4096,
            size_of_disk: 1024,
            image_uuid: Uuid::new_v4(),
            seed_uuid: Uuid::new_v4(),
            public_key_uuid: Uuid::new_v4(),
            ip_addresses: vec!["192.168.1.1".to_string()],
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

        hard_delete_virtual_machine(&uuid1);

        add_virtual_machine(virtual_machine).unwrap();
        let _ = delete_virtual_machine(&uuid1, &context);
        let result = get_virtual_machine(&uuid1, &context);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_virtual_machines_permissions() {
        let _ = init_virtual_machine_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();

        let virtual_machine1 = VirtualMachineEntry {
            uuid: uuid1,
            name: "Alice".to_string(),
            is_created: false,
            number_of_cores: 1,
            size_of_memory: 1024,
            size_of_disk: 20480,
            image_uuid: Uuid::new_v4(),
            seed_uuid: Uuid::new_v4(),
            public_key_uuid: Uuid::new_v4(),
            ip_addresses: Vec::new(),
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

        let virtual_machine2 = VirtualMachineEntry {
            uuid: uuid2,
            name: "Bob".to_string(),
            is_created: false,
            number_of_cores: 1,
            size_of_memory: 1024,
            size_of_disk: 20480,
            image_uuid: Uuid::new_v4(),
            seed_uuid: Uuid::new_v4(),
            public_key_uuid: Uuid::new_v4(),
            ip_addresses: Vec::new(),
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

        let virtual_machine3 = VirtualMachineEntry {
            uuid: uuid3,
            name: "Poi".to_string(),
            is_created: false,
            number_of_cores: 1,
            size_of_memory: 1024,
            size_of_disk: 20480,
            image_uuid: Uuid::new_v4(),
            seed_uuid: Uuid::new_v4(),
            public_key_uuid: Uuid::new_v4(),
            ip_addresses: Vec::new(),
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

        hard_delete_virtual_machine(&uuid1);
        hard_delete_virtual_machine(&uuid2);
        hard_delete_virtual_machine(&uuid3);

        add_virtual_machine(virtual_machine1).unwrap();
        add_virtual_machine(virtual_machine2).unwrap();
        add_virtual_machine(virtual_machine3).unwrap();

        // list-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        let virtual_machines = list_virtual_machines(&context).unwrap();
        assert_eq!(virtual_machines.len(), 1);

        // list-test project-admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: true.to_string(),
        };
        let virtual_machines = list_virtual_machines(&context).unwrap();
        assert_eq!(virtual_machines.len(), 2);

        // list-test admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: true.to_string(),
            is_project_admin: false.to_string(),
        };
        let virtual_machines = list_virtual_machines(&context).unwrap();
        assert_eq!(virtual_machines.len(), 3);

        // get-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        match get_virtual_machine(&uuid1, &context) {
            Ok(retrieved_virtual_machine) => {
                assert_eq!(retrieved_virtual_machine.uuid, uuid1.clone());
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
            is_project_admin: false.to_string(),
        };
        if get_virtual_machine(&uuid3, &context).is_ok() {
            assert_eq!(true, false);
        };

        // delete-test normal user false uuid
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        if delete_virtual_machine(&uuid3, &context).is_ok() {
            assert_eq!(true, false);
        };

        hard_delete_virtual_machine(&uuid1);
        hard_delete_virtual_machine(&uuid2);
        hard_delete_virtual_machine(&uuid3);
    }
}
