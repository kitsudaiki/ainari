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
use torii_common::VNI_MAX;

/// Number of random VNIs, which are checked for a network, before the assignment gives up.
const VNI_ATTEMPTS: u32 = 100;

// Define the schema for networks table
table! {
    networks (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        subnet -> Varchar,
        owner_id -> Varchar,
        project_id -> Varchar,
        created_at -> Varchar,
        created_by -> Varchar,
        updated_at -> Varchar,
        updated_by -> Varchar,
        disable_encryption -> Bool,
        vni -> Nullable<Integer>,
    }
}

/// Represents an entry in the networks table.
/// This struct contains all the fields required to create, query, and update meta network records.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = networks)]
pub struct NetworkEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    pub name: String,
    pub subnet: String,
    pub owner_id: String,
    pub project_id: String,
    #[diesel(serialize_as = DbDateTime, deserialize_as = DbDateTime)]
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    #[diesel(serialize_as = DbDateTime, deserialize_as = DbDateTime)]
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
    /// The traffic between the hosts of the network is not encrypted and the network has no
    /// MLS-group
    pub disable_encryption: bool,
    /// Tenant of the network, which all its addresses live in. It is assigned with the first
    /// address of the network and kept, also while the network has no virtual_machine.
    pub vni: Option<i32>,
}

/// Adds a new meta network to the database.
///
/// This function creates a new NetworkEntry with the provided parameters and inserts it into the database.
/// The timestamps are set to the current time.
///
/// # Arguments
/// * `network_uuid` - The unique identifier for the meta network
/// * `network_name` - The name of the meta network
/// * `subnet` - The subnet of the network in CIDR notation
/// * `network_disable_encryption` - Don't encrypt the traffic between the hosts of the network
/// * `context` - The user context containing information about the user and project
///
/// # Returns
/// A QueryResult indicating the number of rows affected
pub fn add_new_network(
    network_uuid: &Uuid,
    network_name: &str,
    subnet: &str,
    network_disable_encryption: bool,
    context: &UserContext,
) -> QueryResult<usize> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::permission_denied_error());
    }

    let network = NetworkEntry {
        uuid: *network_uuid,
        name: network_name.to_string().clone(),
        subnet: subnet.to_string(),
        owner_id: context.user_id.clone(),
        project_id: context.project_id.clone(),
        created_at: Utc::now(),
        created_by: context.user_id.clone(),
        updated_at: Utc::now(),
        updated_by: context.user_id.clone(),
        disable_encryption: network_disable_encryption,
        vni: None,
    };

    add_network(network)
}

/// Adds a meta network to the database.
///
/// This is a helper function that performs the actual insertion of a NetworkEntry into the database.
///
/// # Arguments
/// * `network` - The NetworkEntry to be inserted
///
/// # Returns
/// A QueryResult indicating the number of rows affected
pub fn add_network(network: NetworkEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::networks::dsl::*;
    diesel::insert_into(networks)
        .values(network)
        .execute(&mut *conn)
}

/// Retrieves a meta network from the database.
///
/// This function queries the database for a meta network with the specified UUID and checks the user's permissions.
/// Only active networks are returned, and the query is filtered based on the user's role and project membership.
///
/// # Arguments
/// * `network_uuid` - The UUID of the meta network to retrieve
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A Result containing the NetworkEntry if found, or a DbError if not found or an error occurs
pub fn get_network(
    network_uuid: &Uuid,
    context: &UserContext,
) -> Result<NetworkEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::networks::dsl::*;

    let mut query = networks
        .filter(uuid.eq(network_uuid.to_string()))
        .into_boxed();

    // Apply permission-based filtering
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
    }

    match query
        .select(NetworkEntry::as_select())
        .first::<NetworkEntry>(&mut *conn)
    {
        Ok(network) => Ok(network),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Lists all meta networks that the user has access to.
///
/// This function retrieves all active meta networks and applies permission-based filtering.
/// The results are filtered based on the user's role and project membership.
///
/// # Arguments
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A QueryResult containing a vector of NetworkEntry objects
pub fn list_networks(context: &UserContext) -> QueryResult<Vec<NetworkEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::networks::dsl::*;

    let mut query = networks.into_boxed();

    // Apply permission-based filtering
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
    }

    query.select(NetworkEntry::as_select()).load(&mut *conn)
}

/// Counts the number of networks of a whole project.
///
/// Unlike `count_networks`, the networks of all users of the project are counted, because the quota,
/// which is checked with this number, belongs to the project.
///
/// # Arguments
///
/// * `project` - The ID of the project, which is counted
///
/// # Returns
///
/// A QueryResult containing the count of networks as an i64
pub fn count_networks_of_project(project: &str) -> QueryResult<i64> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::networks::dsl::*;

    networks
        .filter(project_id.eq(project))
        .select(count_star())
        .first::<i64>(&mut *conn)
}

/// Force deletes a meta network from the database.
///
/// This function removes a meta network without checking permissions.
/// It's intended for system-level operations where permission checks are not required.
///
/// # Arguments
/// * `network_uuid` - The UUID of the meta network to delete
///
/// # Returns
/// A Result indicating success or an error
#[cfg(test)]
pub fn force_delete_network(network_uuid: &Uuid) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::networks::dsl::*;
    match diesel::delete(networks.filter(uuid.eq(network_uuid.to_string()))).execute(&mut *conn) {
        Ok(_) => Ok(()),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Deletes a meta network from the database.
///
/// This function removes a meta network after verifying that the user has permission to delete it.
/// It first checks if the network exists and if the user has the necessary permissions.
///
/// # Arguments
/// * `network_uuid` - The UUID of the meta network to delete
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A Result indicating success or an error
pub fn delete_network(network_uuid: &Uuid, context: &UserContext) -> Result<(), enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    // Verify the meta network exists and the user has permission to delete it
    get_network(network_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::networks::dsl::*;
    match diesel::delete(networks.filter(uuid.eq(network_uuid.to_string()))).execute(&mut *conn) {
        Ok(_) => Ok(()),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Deletes all meta networks from the database.
///
/// This function removes all meta networks without checking permissions.
/// It's intended for system-level operations where permission checks are not required.
///
/// # Returns
/// A Result indicating success or an error
#[allow(dead_code)]
pub fn delete_all_network() -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::networks::dsl::*;
    match diesel::delete(networks).execute(&mut *conn) {
        Ok(_) => Ok(()),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Checks, if the encryption is disabled for a network.
///
/// There is no permission-based filtering, because the gateways of a network don't belong to a
/// single user.
///
/// # Arguments
/// * `network_uuid` - The UUID of the network
///
/// # Returns
/// `true` if the network has its encryption disabled, `NotFound` if it doesn't exist
pub fn is_encryption_disabled(network_uuid: &Uuid) -> Result<bool, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::networks::dsl::*;

    match networks
        .filter(uuid.eq(network_uuid.to_string()))
        .select(disable_encryption)
        .first::<bool>(&mut *conn)
    {
        Ok(disabled) => Ok(disabled),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Lists the UUIDs of all active networks, which have their encryption disabled.
///
/// # Returns
/// A QueryResult containing the UUIDs
pub fn list_networks_without_encryption() -> QueryResult<Vec<Uuid>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::networks::dsl::*;

    let uuids: Vec<String> = networks
        .filter(disable_encryption.eq(true))
        .select(uuid)
        .load(&mut *conn)?;
    Ok(uuids
        .iter()
        .filter_map(|entry| Uuid::parse_str(entry).ok())
        .collect())
}

/// Searches a network by its UUID within all projects.
///
/// It is used to check, if the UUID is still free, before a network with this UUID is imported.
///
/// # Arguments
/// * `network_uuid` - The UUID of the network
///
/// # Returns
/// A QueryResult containing the network, or None, if the UUID is not used
pub fn find_network(network_uuid: &Uuid) -> QueryResult<Option<NetworkEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::networks::dsl::*;

    networks
        .filter(uuid.eq(network_uuid.to_string()))
        .select(NetworkEntry::as_select())
        .first::<NetworkEntry>(&mut *conn)
        .optional()
}

/// Adds a network to the database or replaces the existing entry with the same UUID.
///
/// It is used to import a network with its original UUID, which is the primary key of the table.
/// A replaced network keeps its tenant, if the new entry brings none, because the addresses of
/// its virtual_machines live in it.
///
/// # Arguments
/// * `network` - The network to store
///
/// # Returns
/// A QueryResult indicating the number of rows affected by the insert operation
pub fn put_network(network: NetworkEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::networks::dsl::*;

    conn.transaction(|conn| {
        let mut network = network;
        if network.vni.is_none() {
            network.vni = networks
                .filter(uuid.eq(network.uuid.to_string()))
                .select(vni)
                .first::<Option<i32>>(conn)
                .optional()?
                .flatten();
        }
        diesel::delete(networks.filter(uuid.eq(network.uuid.to_string()))).execute(conn)?;
        diesel::insert_into(networks).values(network).execute(conn)
    })
}

/// Gets the tenant of a network and assigns a random free one, if the network has none yet.
///
/// Tenants start at 1, because `VNI_DEFAULT` (0) is the shared tenant of the uplink and of
/// everything, which was never placed into a tenant of its own. Up to `VNI_ATTEMPTS` random VNIs
/// are checked one after another and the first one, which no other network uses, is taken. The
/// assignment only happens, if the network still has no tenant, so concurrent requests end up
/// with the same tenant.
///
/// There is no permission-based filtering, because the tenant is required for every address of
/// the network, independent of the user.
///
/// # Arguments
/// * `network_uuid` - The UUID of the network
///
/// # Returns
/// A Result containing the VNI of the network, `NotFound` if the network doesn't exist, or
/// `InternalError` if no free VNI was found
pub fn get_or_assign_vni(network_uuid: &Uuid) -> Result<u32, enums::DbError> {
    let db_error = |e: diesel::result::Error| {
        log::error!("Database-error: {e:?}");
        enums::DbError::InternalError
    };
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::networks::dsl::*;
    let network = || networks.filter(uuid.eq(network_uuid.to_string()));

    let existing = network()
        .select(vni)
        .first::<Option<i32>>(&mut *conn)
        .optional()
        .map_err(db_error)?;
    match existing {
        None => return Err(enums::DbError::NotFound),
        Some(Some(existing)) => return Ok(existing as u32),
        Some(None) => {}
    }

    for _ in 0..VNI_ATTEMPTS {
        let candidate = rand::random_range(1..=VNI_MAX) as i32;
        let used = networks
            .filter(vni.eq(candidate))
            .select(count_star())
            .first::<i64>(&mut *conn)
            .map_err(db_error)?
            > 0;
        if used {
            continue;
        }
        diesel::update(network().filter(vni.is_null()))
            .set(vni.eq(candidate))
            .execute(&mut *conn)
            .map_err(db_error)?;
        // another request could have assigned a tenant in the meantime, which is kept then
        return network()
            .select(vni)
            .first::<Option<i32>>(&mut *conn)
            .map_err(db_error)?
            .map(|assigned| assigned as u32)
            .ok_or(enums::DbError::InternalError);
    }

    log::error!("No free VNI found within {VNI_ATTEMPTS} random attempts");
    Err(enums::DbError::InternalError)
}

/// Lists all active networks of a project.
///
/// Unlike the listing for the users, it is done for the project of the path and not the one of
/// the token, so an admin can export any project.
///
/// # Arguments
/// * `project` - The ID of the project
///
/// # Returns
/// A QueryResult containing the networks of the project
pub fn list_networks_of_project(project: &str) -> QueryResult<Vec<NetworkEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::networks::dsl::*;

    networks
        .filter(project_id.eq(project))
        .select(NetworkEntry::as_select())
        .load(&mut *conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ainari_common::enums::ProjectRole;
    use serial_test::serial;

    fn hard_delete_network(network_uuid: &Uuid) {
        use self::networks::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ =
            diesel::delete(networks.filter(uuid.eq(network_uuid.to_string()))).execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_get_or_assign_vni() {
        let (uuid1, uuid2) = (Uuid::new_v4(), Uuid::new_v4());
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: "test-project".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        for network_uuid in [&uuid1, &uuid2] {
            add_new_network(network_uuid, "net", "10.0.0.0/24", false, &context).unwrap();
        }

        // a new network gets its tenant with the first request and keeps it
        let vni1 = get_or_assign_vni(&uuid1).ok().expect("no VNI assigned");
        assert!((1..=VNI_MAX).contains(&vni1));
        assert_eq!(get_or_assign_vni(&uuid1).ok(), Some(vni1));
        let vni2 = get_or_assign_vni(&uuid2).ok().expect("no VNI assigned");
        assert_ne!(vni1, vni2);

        // an imported network, which replaces the entry, keeps the tenant
        let mut imported = find_network(&uuid1).unwrap().unwrap();
        imported.name = "imported".to_string();
        imported.vni = None;
        put_network(imported).unwrap();
        assert_eq!(get_or_assign_vni(&uuid1).ok(), Some(vni1));

        // an unknown network has no tenant
        assert!(matches!(
            get_or_assign_vni(&Uuid::new_v4()),
            Err(enums::DbError::NotFound)
        ));

        hard_delete_network(&uuid1);
        hard_delete_network(&uuid2);
    }

    #[test]
    #[serial]
    fn test_add_get_network() {
        let uuid1 = Uuid::new_v4();
        let name = "test-network".to_string();
        let subnet = "127.0.0.1".to_string();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let network = NetworkEntry {
            uuid: uuid1,
            name: name.clone(),
            subnet: subnet.clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            disable_encryption: false,
            vni: None,
        };

        hard_delete_network(&uuid1);

        add_network(network.clone()).unwrap();
        match get_network(&uuid1, &context) {
            Ok(retrieved_network) => {
                assert_eq!(retrieved_network.uuid, network.uuid);
                assert_eq!(retrieved_network.subnet, network.subnet);
                assert_eq!(retrieved_network.owner_id, network.owner_id);
                assert_eq!(retrieved_network.project_id, network.project_id);
                assert_eq!(retrieved_network.created_by, network.created_by);
                assert_eq!(retrieved_network.updated_by, network.updated_by);
            }
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        hard_delete_network(&uuid1);
    }

    #[test]
    #[serial]
    fn the_encryption_of_a_network_can_be_disabled() {
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: "test-project".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        let encrypted = Uuid::new_v4();
        let unencrypted = Uuid::new_v4();

        add_new_network(&encrypted, "encrypted", "10.1.0.0/24", false, &context).unwrap();
        add_new_network(&unencrypted, "unencrypted", "10.2.0.0/24", true, &context).unwrap();

        assert!(
            !get_network(&encrypted, &context)
                .ok()
                .unwrap()
                .disable_encryption
        );
        assert!(
            get_network(&unencrypted, &context)
                .ok()
                .unwrap()
                .disable_encryption
        );
        assert!(matches!(is_encryption_disabled(&encrypted), Ok(false)));
        assert!(matches!(is_encryption_disabled(&unencrypted), Ok(true)));
        assert!(matches!(
            is_encryption_disabled(&Uuid::new_v4()),
            Err(enums::DbError::NotFound)
        ));

        let without_encryption = list_networks_without_encryption().unwrap();
        assert!(without_encryption.contains(&unencrypted));
        assert!(!without_encryption.contains(&encrypted));

        hard_delete_network(&encrypted);
        hard_delete_network(&unencrypted);
    }

    #[test]
    #[serial]
    fn test_list_networks() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let name = "test-network".to_string();
        let subnet = "127.0.0.1".to_string();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let network1 = NetworkEntry {
            uuid: uuid1,
            name: name.clone(),
            subnet: subnet.clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            disable_encryption: false,
            vni: None,
        };

        let network2 = NetworkEntry {
            uuid: uuid2,
            name: name.clone(),
            subnet: subnet.clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            disable_encryption: false,
            vni: None,
        };

        hard_delete_network(&uuid1);
        hard_delete_network(&uuid2);

        add_network(network1).unwrap();
        add_network(network2).unwrap();
        // a deleted network is not listed anymore
        hard_delete_network(&uuid2);
        let networks = list_networks(&context).unwrap();
        assert_eq!(networks.len(), 1);
        hard_delete_network(&uuid1);
        hard_delete_network(&uuid2);
    }

    #[test]
    #[serial]
    fn test_delete_network() {
        let uuid1 = Uuid::new_v4();
        let name = "test-network".to_string();
        let subnet = "127.0.0.1".to_string();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let network = NetworkEntry {
            uuid: uuid1,
            name: name.clone(),
            subnet: subnet.clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            disable_encryption: false,
            vni: None,
        };

        hard_delete_network(&uuid1);

        add_network(network.clone()).unwrap();
        let _ = delete_network(&uuid1, &context);
        let result = get_network(&uuid1, &context);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_count_networks_of_project() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let name = "test-network".to_string();
        let subnet = "127.0.0.1".to_string();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let network1 = NetworkEntry {
            uuid: uuid1,
            name: name.clone(),
            subnet: subnet.clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            disable_encryption: false,
            vni: None,
        };

        let network2 = NetworkEntry {
            uuid: uuid2,
            name: name.clone(),
            subnet: subnet.clone(),
            owner_id: "other-user".to_string(),
            project_id: project_id.clone(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            disable_encryption: false,
            vni: None,
        };

        let network3 = NetworkEntry {
            uuid: uuid3,
            name: name.clone(),
            subnet: subnet.clone(),
            owner_id: owner_id.clone(),
            project_id: "other-project".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            disable_encryption: false,
            vni: None,
        };

        hard_delete_network(&uuid1);
        hard_delete_network(&uuid2);
        hard_delete_network(&uuid3);

        add_network(network1).unwrap();
        add_network(network2).unwrap();
        add_network(network3).unwrap();

        // the networks of all users of the project are counted, but not the ones of other projects
        let number = count_networks_of_project(&context.project_id).unwrap();
        assert_eq!(number, 2);

        hard_delete_network(&uuid1);
        hard_delete_network(&uuid2);
        hard_delete_network(&uuid3);
    }

    #[test]
    #[serial]
    fn test_networks_permissions() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let name = "test-network".to_string();
        let subnet = "127.0.0.1".to_string();

        let network1 = NetworkEntry {
            uuid: uuid1,
            name: name.clone(),
            subnet: subnet.clone(),
            owner_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            disable_encryption: false,
            vni: None,
        };

        let network2 = NetworkEntry {
            uuid: uuid2,
            name: name.clone(),
            subnet: subnet.clone(),
            owner_id: "test-user-43".to_string(),
            project_id: "test_permissions_1".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            disable_encryption: false,
            vni: None,
        };

        let network3 = NetworkEntry {
            uuid: uuid3,
            name: name.clone(),
            subnet: subnet.clone(),
            owner_id: "test-user-44".to_string(),
            project_id: "test_permissions_2".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            disable_encryption: false,
            vni: None,
        };

        hard_delete_network(&uuid1);
        hard_delete_network(&uuid2);
        hard_delete_network(&uuid3);

        add_network(network1).unwrap();
        add_network(network2).unwrap();
        add_network(network3).unwrap();

        // list-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        let networks = list_networks(&context).unwrap();
        // members see all entries of their project, also the ones of other users
        assert_eq!(networks.len(), 2);

        // list-test project-admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Admin.to_string(),
        };
        let networks = list_networks(&context).unwrap();
        assert_eq!(networks.len(), 2);

        // list-test admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: true.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        let networks = list_networks(&context).unwrap();
        assert_eq!(networks.len(), 3);

        // get-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        match get_network(&uuid1, &context) {
            Ok(retrieved_network) => {
                assert_eq!(retrieved_network.uuid, uuid1);
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
        if get_network(&uuid3, &context).is_ok() {
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
        if delete_network(&uuid3, &context).is_ok() {
            assert_eq!(true, false);
        };

        hard_delete_network(&uuid1);
        hard_delete_network(&uuid2);
        hard_delete_network(&uuid3);
    }
}
