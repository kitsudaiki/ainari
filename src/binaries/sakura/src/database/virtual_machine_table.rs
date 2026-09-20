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

use std::net::Ipv4Addr;

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
        number_of_cores -> Integer,
        memory_size -> BigInt,
        image_uuid -> Varchar,
        public_key_uuid -> Varchar,
        network_uuid -> Varchar,
        internal_ip -> Varchar,
        root_disk_path -> Nullable<Varchar>,
        seed_path -> Varchar,
        tap_name -> Varchar,
        mac_address -> Varchar,
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
    pub number_of_cores: i32,
    pub memory_size: i64,
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub image_uuid: Uuid,
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub public_key_uuid: Uuid,
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub network_uuid: Uuid,
    #[diesel(serialize_as = DbIpv4Addr, deserialize_as = DbIpv4Addr)]
    pub internal_ip: Ipv4Addr,
    pub root_disk_path: Option<String>,
    pub seed_path: String,
    pub tap_name: String,
    pub mac_address: String,
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
        memory_size INTEGER,
        image_uuid VARCHAR(40),
        public_key_uuid VARCHAR(40),
        network_uuid VARCHAR(40),
        internal_ip VARCHAR(40),
        root_disk_path VARCHAR(1024),
        seed_path VARCHAR(1024),
        tap_name VARCHAR(256),
        mac_address VARCHAR(32),
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

/// Values of a new virtual_machine, which are provided by the caller of `add_new_virtual_machine`
#[derive(Debug, Clone)]
pub struct NewVirtualMachine {
    /// Unique identifier for the new virtual_machine
    pub uuid: Uuid,
    /// Name for the new virtual_machine
    pub name: String,
    /// Number of cpu-cores of the new virtual_machine
    pub number_of_cores: i32,
    /// Amount of memory in bytes of the new virtual_machine
    pub memory_size: i64,
    /// Unique identifier of the image of the new virtual_machine
    pub image_uuid: Uuid,
    /// Unique identifier of the public-key of the new virtual_machine
    pub public_key_uuid: Uuid,
    /// Unique identifier of the network of the new virtual_machine
    pub network_uuid: Uuid,
    /// Internal address of the new virtual_machine
    pub internal_ip: Ipv4Addr,
    /// Optional path to the root-disk-image of the new virtual_machine
    pub root_disk_path: Option<String>,
    /// Path to the cloud-init seed-image of the new virtual_machine
    pub seed_path: String,
    /// Name of the TAP-device of the new virtual_machine
    pub tap_name: String,
    /// MAC-address of the network-interface of the new virtual_machine
    pub mac_address: String,
}

/// Adds a new virtual_machine to the database with the provided information
///
/// # Arguments
/// * `new_virtual_machine` - Values of the new virtual_machine
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(usize)` with the number of rows inserted on success
/// * `Err` with an appropriate error on failure
pub fn add_new_virtual_machine(
    new_virtual_machine: NewVirtualMachine,
    context: &UserContext,
) -> QueryResult<usize> {
    // Create the new virtual_machine entry
    let virtual_machine = VirtualMachineEntry {
        uuid: new_virtual_machine.uuid,
        name: new_virtual_machine.name,
        is_created: false,
        number_of_cores: new_virtual_machine.number_of_cores,
        memory_size: new_virtual_machine.memory_size,
        image_uuid: new_virtual_machine.image_uuid,
        public_key_uuid: new_virtual_machine.public_key_uuid,
        network_uuid: new_virtual_machine.network_uuid,
        internal_ip: new_virtual_machine.internal_ip,
        root_disk_path: new_virtual_machine.root_disk_path,
        seed_path: new_virtual_machine.seed_path,
        tap_name: new_virtual_machine.tap_name,
        mac_address: new_virtual_machine.mac_address,
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

/// Sets the image and the public-key of a reserved virtual_machine
///
/// A virtual_machine is reserved without an image and without a public-key. Both are chosen, when
/// the virtual_machine is really created, and are stored here, so the task, which creates it, can
/// read them from the database.
///
/// # Arguments
/// * `virtual_machine_uuid` - Unique identifier of the virtual_machine to update
/// * `new_image_uuid` - Unique identifier of the image of the virtual_machine
/// * `new_public_key_uuid` - Unique identifier of the public-key of the virtual_machine
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(enums::DbError)` with an appropriate error on failure
pub fn set_virtual_machine_image(
    virtual_machine_uuid: &Uuid,
    new_image_uuid: &Uuid,
    new_public_key_uuid: &Uuid,
    context: &UserContext,
) -> Result<(), enums::DbError> {
    // First verify that the virtual_machine exists and the user has permission to update it
    get_virtual_machine(virtual_machine_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::virtual_machines::dsl::*;

    match diesel::update(virtual_machines.filter(uuid.eq(virtual_machine_uuid.to_string())))
        .set((
            image_uuid.eq(new_image_uuid.to_string()),
            public_key_uuid.eq(new_public_key_uuid.to_string()),
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

/// Updates the image, public-key, seed-image and root-disk of an existing virtual_machine
/// and marks it as created
///
/// # Arguments
/// * `virtual_machine_uuid` - Unique identifier of the virtual_machine to update
/// * `new_image_uuid` - Unique identifier of the new image of the virtual_machine
/// * `new_public_key_uuid` - Unique identifier of the new public-key of the virtual_machine
/// * `new_seed_path` - New path to the cloud-init seed-image of the virtual_machine
/// * `new_root_disk_path` - New optional path to the root-disk-image of the virtual_machine
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(enums::DbError)` with an appropriate error on failure
#[allow(dead_code)]
pub fn update_virtual_machine(
    virtual_machine_uuid: &Uuid,
    new_image_uuid: &Uuid,
    new_public_key_uuid: &Uuid,
    new_seed_path: &str,
    new_root_disk_path: Option<String>,
    context: &UserContext,
) -> Result<(), enums::DbError> {
    // First verify that the virtual_machine exists and the user has permission to update it
    get_virtual_machine(virtual_machine_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::virtual_machines::dsl::*;

    // Update the values and set the update timestamp and user
    match diesel::update(virtual_machines.filter(uuid.eq(virtual_machine_uuid.to_string())))
        .set((
            is_created.eq(true),
            image_uuid.eq(new_image_uuid.to_string()),
            public_key_uuid.eq(new_public_key_uuid.to_string()),
            seed_path.eq(new_seed_path),
            root_disk_path.eq(new_root_disk_path),
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
            memory_size: 4096,
            image_uuid: Uuid::new_v4(),
            public_key_uuid: Uuid::new_v4(),
            network_uuid: Uuid::new_v4(),
            internal_ip: Ipv4Addr::new(192, 168, 100, 2),
            root_disk_path: Some("/tmp/ubuntu-24.04.raw".to_string()),
            seed_path: "/tmp/seed.iso".to_string(),
            tap_name: "tap-vm".to_string(),
            mac_address: "02:00:00:00:00:42".to_string(),
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
                    retrieved_virtual_machine.memory_size,
                    virtual_machine.memory_size
                );
                assert_eq!(
                    retrieved_virtual_machine.image_uuid,
                    virtual_machine.image_uuid
                );
                assert_eq!(
                    retrieved_virtual_machine.public_key_uuid,
                    virtual_machine.public_key_uuid
                );
                assert_eq!(
                    retrieved_virtual_machine.network_uuid,
                    virtual_machine.network_uuid
                );
                assert_eq!(
                    retrieved_virtual_machine.internal_ip,
                    virtual_machine.internal_ip
                );
                assert_eq!(
                    retrieved_virtual_machine.root_disk_path,
                    virtual_machine.root_disk_path
                );
                assert_eq!(
                    retrieved_virtual_machine.seed_path,
                    virtual_machine.seed_path
                );
                assert_eq!(retrieved_virtual_machine.tap_name, virtual_machine.tap_name);
                assert_eq!(
                    retrieved_virtual_machine.mac_address,
                    virtual_machine.mac_address
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
            memory_size: 4096,
            image_uuid: Uuid::new_v4(),
            public_key_uuid: Uuid::new_v4(),
            network_uuid: Uuid::new_v4(),
            internal_ip: Ipv4Addr::new(192, 168, 100, 2),
            root_disk_path: Some("/tmp/ubuntu-24.04.raw".to_string()),
            seed_path: "/tmp/seed.iso".to_string(),
            tap_name: "tap-vm".to_string(),
            mac_address: "02:00:00:00:00:42".to_string(),
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
            memory_size: 4096,
            image_uuid: Uuid::new_v4(),
            public_key_uuid: Uuid::new_v4(),
            network_uuid: Uuid::new_v4(),
            internal_ip: Ipv4Addr::new(192, 168, 100, 2),
            root_disk_path: Some("/tmp/ubuntu-24.04.raw".to_string()),
            seed_path: "/tmp/seed.iso".to_string(),
            tap_name: "tap-vm".to_string(),
            mac_address: "02:00:00:00:00:42".to_string(),
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
            memory_size: 4096,
            image_uuid: Uuid::new_v4(),
            public_key_uuid: Uuid::new_v4(),
            network_uuid: Uuid::new_v4(),
            internal_ip: Ipv4Addr::new(192, 168, 100, 2),
            root_disk_path: Some("/tmp/ubuntu-24.04.raw".to_string()),
            seed_path: "/tmp/seed.iso".to_string(),
            tap_name: "tap-vm".to_string(),
            mac_address: "02:00:00:00:00:42".to_string(),
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
            memory_size: 1024,
            image_uuid: Uuid::new_v4(),
            public_key_uuid: Uuid::new_v4(),
            network_uuid: Uuid::new_v4(),
            internal_ip: Ipv4Addr::new(192, 168, 100, 2),
            root_disk_path: Some("/tmp/ubuntu-24.04.raw".to_string()),
            seed_path: "/tmp/seed.iso".to_string(),
            tap_name: "tap-vm".to_string(),
            mac_address: "02:00:00:00:00:42".to_string(),
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
            memory_size: 1024,
            image_uuid: Uuid::new_v4(),
            public_key_uuid: Uuid::new_v4(),
            network_uuid: Uuid::new_v4(),
            internal_ip: Ipv4Addr::new(192, 168, 100, 2),
            root_disk_path: Some("/tmp/ubuntu-24.04.raw".to_string()),
            seed_path: "/tmp/seed.iso".to_string(),
            tap_name: "tap-vm".to_string(),
            mac_address: "02:00:00:00:00:42".to_string(),
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
            memory_size: 1024,
            image_uuid: Uuid::new_v4(),
            public_key_uuid: Uuid::new_v4(),
            network_uuid: Uuid::new_v4(),
            internal_ip: Ipv4Addr::new(192, 168, 100, 2),
            root_disk_path: Some("/tmp/ubuntu-24.04.raw".to_string()),
            seed_path: "/tmp/seed.iso".to_string(),
            tap_name: "tap-vm".to_string(),
            mac_address: "02:00:00:00:00:42".to_string(),
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
