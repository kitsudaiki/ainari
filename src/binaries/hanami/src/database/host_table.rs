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
use rand::prelude::SliceRandom;
use uuid::Uuid;

use crate::database::db_handle;

use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;
use ainari_common::objects::*;

// Define the schema
table! {
    hosts (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        address -> Varchar,
        external_address -> Nullable<Varchar>,
        mls_signature_key -> Nullable<Varchar>,
        mls_client_id -> Nullable<Varchar>,
        number_of_cores -> BigInt,
        used_number_of_cores -> BigInt,
        memory_size -> BigInt,
        amount_of_used_memory -> BigInt,
        disk_space -> BigInt,
        amount_of_used_disk_space -> BigInt,
        is_host_isolated -> Bool,
        project_id -> Nullable<Varchar>,
        status -> Varchar,
        created_at -> Varchar,
        created_by -> Varchar,
        updated_at -> Varchar,
        updated_by -> Varchar,
        deleted_at -> Nullable<Varchar>,
        deleted_by -> Nullable<Varchar>,
    }
}

/// Represents a host entry in the database.
///
/// This struct maps to the `hosts` table in the database and contains all the fields
/// necessary to create, read, update, and delete host records.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = hosts)]
pub struct HostEntry {
    /// Unique identifier for the host
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    /// Human-readable name for the host
    pub name: String,
    /// Network address of the host
    pub address: String,
    /// Address of the external api of the host, which is the target of its proxies on the torii
    /// at the edge. None for hosts, which only have one address, so `address` is used instead.
    pub external_address: Option<String>,
    /// Number of cpu-threads of the host
    pub number_of_cores: i64,
    /// Number of cpu-threads, which are already in use
    pub used_number_of_cores: i64,
    /// Total memory of the host in MiB
    pub memory_size: i64,
    /// Amount of memory in MiB, which is already in use
    pub amount_of_used_memory: i64,
    /// Total size of the disk for the virtual-machines in GiB
    pub disk_space: i64,
    /// Amount of disk-space in GiB, which is already in use
    pub amount_of_used_disk_space: i64,
    /// True, if the host is isolated and only used by the virtual-machines of a single project
    pub is_host_isolated: bool,
    /// ID of the project, which the host is isolated for. None, if the host is not isolated
    pub project_id: Option<String>,
    /// Current status of the host (ACTIVE, DELETED, etc.)
    pub status: String,
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
    /// Timestamp when the host was deleted (if applicable)
    #[diesel(serialize_as = DbOptDateTime, deserialize_as = DbOptDateTime)]
    pub deleted_at: Option<DateTime<Utc>>,
    /// User ID who deleted the host (if applicable)
    pub deleted_by: Option<String>,
}

impl HostEntry {
    /// Address, which the proxies of the host forward to.
    ///
    /// # Returns
    /// The external address of the host, or its only address, if it has no external one
    pub fn proxy_target_address(&self) -> &str {
        self.external_address.as_deref().unwrap_or(&self.address)
    }
}

/// Hardware-resources of a host, which are reported by the host itself at registration.
#[derive(Debug, PartialEq, Clone)]
pub struct HostResources {
    /// Number of cpu-threads of the host
    pub number_of_cores: i64,
    /// Total memory of the host in MiB
    pub memory_size: i64,
    /// Total size of the disk for the virtual-machines in GiB
    pub disk_space: i64,
    /// Project, which requests the resources. Only hosts isolated for this project, or isolated
    /// hosts without project yet, are used for it. None to use only not isolated hosts
    pub project_id: Option<String>,
}

/// Adds a new host to the database with default values.
///
/// This function creates a new HostEntry with the provided UUID, name, address and
/// hardware-resources, sets the status to "ACTIVE", sets the used resources to 0 and uses
/// the current timestamp and user context for creation and update information.
///
/// # Arguments
/// * `host_uuid` - Unique identifier for the new host
/// * `host_name` - Human-readable name for the host
/// * `host_address` - Network address of the host
/// * `host_external_address` - Address of the external api of the host, if it has one
/// * `resources` - Hardware-resources of the host
/// * `context` - User context containing information about the user performing the action
///
/// # Returns
/// * QueryResult containing the number of rows affected
pub fn add_new_host(
    host_uuid: &Uuid,
    host_name: &str,
    host_address: &str,
    host_external_address: Option<&str>,
    resources: &HostResources,
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
        external_address: host_external_address.map(str::to_owned),
        number_of_cores: resources.number_of_cores,
        used_number_of_cores: 0,
        memory_size: resources.memory_size,
        amount_of_used_memory: 0,
        disk_space: resources.disk_space,
        amount_of_used_disk_space: 0,
        is_host_isolated: false,
        project_id: None,
        status: "ACTIVE".to_string(),
        created_at: Utc::now(),
        created_by: context.user_id.clone(),
        updated_at: Utc::now(),
        updated_by: context.user_id.clone(),
        deleted_at: None,
        deleted_by: None,
    };

    add_host(host.clone())
}

/// Updates the hardware-resources of an existing host.
///
/// This is used, when an already registered host registers itself again, because
/// its hardware could have changed in the meantime.
///
/// # Arguments
/// * `host_uuid` - Unique identifier of the host to update
/// * `resources` - New hardware-resources of the host
/// * `context` - User context containing information about the user performing the action
///
/// # Returns
/// * Ok(()) if the host was successfully updated
/// * DbError::NotFound if the host doesn't exist or is not active
/// * DbError::InternalError if there was an error executing the query
pub fn update_host_resources(
    host_uuid: &Uuid,
    resources: &HostResources,
    context: &UserContext,
) -> Result<(), enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;
    match diesel::update(hosts.filter(uuid.eq(host_uuid.to_string()).and(status.eq("ACTIVE"))))
        .set((
            number_of_cores.eq(resources.number_of_cores),
            memory_size.eq(resources.memory_size),
            disk_space.eq(resources.disk_space),
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

/// Updates the address of the external api of an existing host.
///
/// This is used, when an already registered host registers itself again, because its external
/// address could have changed, or it could have none before.
///
/// # Arguments
/// * `host_uuid` - Unique identifier of the host to update
/// * `host_external_address` - Address of the external api of the host, if it has one
/// * `context` - User context containing information about the user performing the action
///
/// # Returns
/// * Ok(()) if the host was successfully updated
/// * DbError::NotFound if the host doesn't exist or is not active
/// * DbError::InternalError if there was an error executing the query
pub fn update_host_external_address(
    host_uuid: &Uuid,
    host_external_address: Option<&str>,
    context: &UserContext,
) -> Result<(), enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;
    match diesel::update(hosts.filter(uuid.eq(host_uuid.to_string()).and(status.eq("ACTIVE"))))
        .set((
            external_address.eq(host_external_address),
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

/// Sets, if an existing host is isolated for a single project.
///
/// The change is only allowed, if no resources are allocated on the host and the host is not
/// bound to a project yet. Both is checked within the update, so a parallel allocation can not
/// slip in between.
///
/// # Arguments
/// * `host_uuid` - Unique identifier of the host to update
/// * `host_is_isolated` - True to isolate the host, false to use it for all projects
/// * `context` - User context containing information about the user performing the action
///
/// # Returns
/// * Ok(true) if the host was successfully updated
/// * Ok(false) if the change is not allowed, because resources are allocated on the host or the
///   host is already bound to a project
/// * DbError::NotFound if the host doesn't exist or is not active
/// * DbError::InternalError if there was an error executing the query
pub fn set_host_isolation(
    host_uuid: &Uuid,
    host_is_isolated: bool,
    context: &UserContext,
) -> Result<bool, enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;

    let host_uuid = host_uuid.to_string();

    let result = conn.transaction::<_, diesel::result::Error, _>(|conn| {
        let is_active_host = uuid.eq(&host_uuid).and(status.eq("ACTIVE"));
        let is_unused = used_number_of_cores
            .eq(0)
            .and(amount_of_used_memory.eq(0))
            .and(amount_of_used_disk_space.eq(0))
            .and(project_id.is_null());

        let updated = diesel::update(hosts.filter(is_active_host.and(is_unused)))
            .set((
                is_host_isolated.eq(host_is_isolated),
                updated_at.eq(Utc::now().to_rfc3339()),
                updated_by.eq(context.user_id.clone()),
            ))
            .execute(conn)?;
        if updated == 1 {
            return Ok(true);
        }

        // nothing was updated, because the host doesn't exist or the change is not allowed
        hosts
            .filter(is_active_host)
            .select(uuid)
            .first::<String>(conn)
            .map(|_| false)
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

/// Returns the MLS signature-key of the torii of a host, which was pinned with the first
/// membership-grant of the host.
///
/// The key is not part of `HostEntry`, because only the membership-grants care about it.
///
/// # Arguments
/// * `host_uuid` - UUID of the host
///
/// # Returns
/// The pinned key, `None` if no key is pinned yet, or `NotFound` if the host doesn't exist
pub fn get_mls_signature_key(host_uuid: &Uuid) -> Result<Option<String>, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;

    match hosts
        .filter(uuid.eq(host_uuid.to_string()).and(status.eq("ACTIVE")))
        .select(mls_signature_key)
        .first::<Option<String>>(&mut *conn)
    {
        Ok(key) => Ok(key),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Pins the MLS signature-key and the MLS-identity of the torii of a host, if none is pinned yet.
///
/// The key is only set, if the host has none yet, so two parallel requests can't pin different
/// keys. The caller reads the pinned key afterwards to see, which one won.
///
/// # Arguments
/// * `host_uuid` - UUID of the host
/// * `key` - Base64-encoded signature-key
/// * `client_id` - MLS-identity of the torii, which is its underlay-address
///
/// # Returns
/// `Ok(())`, also if another key was pinned before
pub fn pin_mls_signature_key(
    host_uuid: &Uuid,
    key: &str,
    client_id: &str,
) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;

    match diesel::update(
        hosts.filter(
            uuid.eq(host_uuid.to_string())
                .and(status.eq("ACTIVE"))
                .and(mls_signature_key.is_null()),
        ),
    )
    .set((mls_signature_key.eq(key), mls_client_id.eq(client_id)))
    .execute(&mut *conn)
    {
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Forgets the pinned MLS signature-key of the torii of a host.
///
/// A sakura-host registers itself again with every start. Its torii may have started from scratch
/// as well, with a new signature-key and on another address, for example in a new pod. The next
/// membership-grant pins the key, which the torii shows then. The registration is protected by
/// the registration-key, which already decides, which hosts take part at all.
///
/// # Arguments
/// * `host_uuid` - UUID of the host
///
/// # Returns
/// The MLS-identity, which was pinned before, or `None` if nothing was pinned
pub fn reset_mls_pin(host_uuid: &Uuid) -> Result<Option<String>, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;

    let filter = || hosts.filter(uuid.eq(host_uuid.to_string()).and(status.eq("ACTIVE")));
    let previous = match filter()
        .select(mls_client_id)
        .first::<Option<String>>(&mut *conn)
    {
        Ok(previous) => previous,
        Err(diesel::result::Error::NotFound) => return Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            return Err(enums::DbError::InternalError);
        }
    };

    match diesel::update(filter())
        .set((
            mls_signature_key.eq(None::<String>),
            mls_client_id.eq(None::<String>),
        ))
        .execute(&mut *conn)
    {
        Ok(_) => Ok(previous),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Adds a host to the database.
///
/// This is a lower-level function that takes a fully constructed HostEntry and
/// inserts it into the database.
///
/// # Arguments
/// * `host` - HostEntry to be inserted into the database
///
/// # Returns
/// * QueryResult containing the number of rows affected
pub fn add_host(host: HostEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;
    diesel::insert_into(hosts).values(host).execute(&mut *conn)
}

/// Retrieves a host by its network address.
///
/// This function queries the database for a host with the given address that
/// has an "ACTIVE" status.
///
/// # Arguments
/// * `host_address` - Network address of the host to retrieve
/// * `_` - User context (not currently used in the query)
///
/// # Returns
/// * Ok(HostEntry) if a matching host is found
/// * DbError::NotFound if no matching host is found
/// * DbError::InternalError if there was an error executing the query
pub fn get_host_by_address(
    host_address: &String,
    _: &UserContext,
) -> Result<HostEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;

    let query = hosts
        .filter(
            address
                .eq(host_address.to_string())
                .and(status.eq("ACTIVE")),
        )
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

/// Retrieves a host by its UUID.
///
/// This function queries the database for a host with the given UUID that
/// has an "ACTIVE" status.
///
/// # Arguments
/// * `host_uuid` - Unique identifier of the host to retrieve
/// * `_` - User context (not currently used in the query)
///
/// # Returns
/// * Ok(HostEntry) if a matching host is found
/// * DbError::NotFound if no matching host is found
/// * DbError::InternalError if there was an error executing the query
pub fn get_host(host_uuid: &Uuid, _: &UserContext) -> Result<HostEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;

    let query = hosts
        .filter(uuid.eq(host_uuid.to_string()).and(status.eq("ACTIVE")))
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

/// Lists all active hosts in the database.
///
/// This function retrieves all hosts that have an "ACTIVE" status.
///
/// # Arguments
/// * `_` - User context (not currently used in the query)
///
/// # Returns
/// * QueryResult containing a vector of HostEntry objects
pub fn list_hosts(_: &UserContext) -> QueryResult<Vec<HostEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;

    let query = hosts.filter(status.eq("ACTIVE")).into_boxed();

    query.select(HostEntry::as_select()).load(&mut *conn)
}

/// Deletes a host by marking it as "DELETED".
///
/// This function updates the host's status to "DELETED" and sets the deletion
/// timestamp and user. It first verifies that the host exists and is active.
///
/// # Arguments
/// * `host_uuid` - Unique identifier of the host to delete
/// * `context` - User context containing information about the user performing the deletion
///
/// # Returns
/// * Ok(()) if the host was successfully deleted
/// * DbError::NotFound if the host doesn't exist or is already deleted
/// * DbError::InternalError if there was an error executing the query
pub fn delete_host_admin(host_uuid: &Uuid, context: &UserContext) -> Result<(), enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    get_host(host_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;
    match diesel::update(hosts.filter(uuid.eq(host_uuid.to_string())))
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

/// Selects a random active host, which has enough free resources for the requested values,
/// and allocates the requested resources on this host.
///
/// A host is suitable, if for cores, memory and disk the total value minus the used value
/// is greater or equal to the requested value and its isolation matches the project of the
/// request (see `HostResources::project_id`). The used values of the selected host are
/// increased by the requested values.
///
/// With project, the isolated hosts already bound to this project are tried first. Only if none
/// of them fits, an isolated host without project is selected and bound to the project.
///
/// The free resources are checked again within the update, which allocates them, so the
/// allocation is atomic within the database. If another request, for example of another
/// hanami-instance with the same database, allocated the resources of the selected host in
/// between, the update doesn't change anything and the next suitable host is tried.
///
/// # Arguments
/// * `requested` - Resources, which are requested by the new virtual-machine
/// * `_` - User context (not currently used in the query)
///
/// # Returns
/// * Ok(HostEntry) with the selected host and its updated used values
/// * DbError::NotFound if no host has enough free resources
/// * DbError::InternalError if there was an error executing the queries
pub fn allocate_host_resources(
    requested: &HostResources,
    _: &UserContext,
) -> Result<HostEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;

    let requested_project = requested.project_id.as_deref();

    let result = conn.transaction::<_, diesel::result::Error, _>(|conn| {
        // Without project only not isolated hosts are used. With project only isolated hosts,
        // which are not bound to a project yet or bound to the requested project, are used.
        // Both cases are one expression, so the filter has the same type for both.
        let matches_isolation = is_host_isolated.eq(requested_project.is_some()).and(
            is_host_isolated
                .eq(false)
                .or(project_id.is_null())
                .or(project_id.eq(requested_project.unwrap_or_default())),
        );

        let has_free_resources = status
            .eq("ACTIVE")
            .and(matches_isolation)
            .and((number_of_cores - used_number_of_cores).ge(requested.number_of_cores))
            .and((memory_size - amount_of_used_memory).ge(requested.memory_size))
            .and((disk_space - amount_of_used_disk_space).ge(requested.disk_space));

        let mut suitable_hosts = hosts
            .filter(has_free_resources)
            .select((uuid, project_id))
            .load::<(String, Option<String>)>(conn)?;
        suitable_hosts.shuffle(&mut rand::rng());
        // Hosts, which are already bound to the requested project, are tried first and only
        // afterwards the isolated hosts without project. The sort is stable, so both groups
        // stay shuffled.
        suitable_hosts.sort_by_key(|(_, host_project)| host_project.is_none());

        for (selected_uuid, _) in suitable_hosts {
            let used_resources = (
                used_number_of_cores.eq(used_number_of_cores + requested.number_of_cores),
                amount_of_used_memory.eq(amount_of_used_memory + requested.memory_size),
                amount_of_used_disk_space.eq(amount_of_used_disk_space + requested.disk_space),
            );
            let update =
                diesel::update(hosts.filter(uuid.eq(&selected_uuid).and(has_free_resources)));

            // An isolated host is bound to the project, which allocates resources on it at first.
            // If another project bound the host in between, the filter of the update doesn't
            // match anymore and the next host is tried.
            let allocated = match requested_project {
                Some(project) => update
                    .set((used_resources, project_id.eq(project)))
                    .execute(conn)?,
                None => update.set(used_resources).execute(conn)?,
            };

            if allocated == 1 {
                return hosts
                    .filter(uuid.eq(&selected_uuid))
                    .select(HostEntry::as_select())
                    .first::<HostEntry>(conn);
            }
        }

        Err(diesel::result::Error::NotFound)
    });

    match result {
        Ok(host) => Ok(host),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Releases resources, which were allocated on a host by `allocate_host_resources`.
///
/// The used values of the host are decreased by the given values, but never below 0. If the host
/// is isolated and all its used values are 0 afterwards, it is unbound from its project.
///
/// # Arguments
/// * `host_uuid` - Unique identifier of the host
/// * `released` - Resources to release on the host
///
/// # Returns
/// * Ok(()) if the resources were successfully released
/// * DbError::NotFound if the host doesn't exist
/// * DbError::InternalError if there was an error executing the query
pub fn release_host_resources(
    host_uuid: &Uuid,
    released: &HostResources,
) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;
    use diesel::dsl::sql;
    use diesel::sql_types::BigInt;

    // Decreases a used value, but never below 0, which keeps the values valid for the
    // CHECK-constraints of the table. It is written as CASE, because sqlite and mysql have
    // different functions for the maximum of two values.
    let decrease = |column: &str, value: i64| {
        sql::<BigInt>(&format!("CASE WHEN {column} > "))
            .bind::<BigInt, _>(value)
            .sql(&format!(" THEN {column} - "))
            .bind::<BigInt, _>(value)
            .sql(" ELSE 0 END")
    };

    let host_uuid = host_uuid.to_string();

    let result = conn.transaction::<_, diesel::result::Error, _>(|conn| {
        let released_rows = diesel::update(hosts.filter(uuid.eq(&host_uuid)))
            .set((
                used_number_of_cores.eq(decrease("used_number_of_cores", released.number_of_cores)),
                amount_of_used_memory.eq(decrease("amount_of_used_memory", released.memory_size)),
                amount_of_used_disk_space
                    .eq(decrease("amount_of_used_disk_space", released.disk_space)),
            ))
            .execute(conn)?;

        // an isolated host, which runs nothing anymore, is unbound from its project, so it can
        // be used by another project again
        diesel::update(
            hosts.filter(
                uuid.eq(&host_uuid)
                    .and(is_host_isolated.eq(true))
                    .and(used_number_of_cores.eq(0))
                    .and(amount_of_used_memory.eq(0))
                    .and(amount_of_used_disk_space.eq(0)),
            ),
        )
        .set(project_id.eq(None::<String>))
        .execute(conn)?;

        Ok(released_rows)
    });

    match result {
        Ok(0) => Err(enums::DbError::NotFound),
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Deletes all active hosts by marking them as "DELETED".
///
/// This function updates all hosts with an "ACTIVE" status to "DELETED"
/// and sets the deletion timestamp and user. This is typically used for
/// database reset or cleanup purposes.
///
/// # Returns
/// * Ok(()) if all hosts were successfully deleted
/// * DbError::NotFound if no active hosts were found
/// * DbError::InternalError if there was an error executing the query
#[allow(dead_code)]
pub fn delete_all_host() -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;
    match diesel::update(hosts.filter(status.eq("ACTIVE")))
        .set((
            status.eq("DELETED"),
            deleted_at.eq(Utc::now().to_rfc3339()),
            deleted_by.eq("HANAMI_START"),
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
            number_of_cores: 16,
            used_number_of_cores: 0,
            memory_size: 32768,
            amount_of_used_memory: 0,
            disk_space: 1024,
            amount_of_used_disk_space: 0,
            is_host_isolated: false,
            project_id: None,
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
            external_address: None,
        };

        hard_delete_host(&uuid1);

        add_host(host.clone()).unwrap();
        match get_host(&uuid1, &context) {
            Ok(retrieved_host) => {
                assert_eq!(retrieved_host.uuid, host.uuid);
                assert_eq!(retrieved_host.name, host.name);
                assert_eq!(retrieved_host.status, host.status);
                assert_eq!(retrieved_host.created_by, host.created_by);
                assert_eq!(retrieved_host.updated_by, host.updated_by);
                assert_eq!(retrieved_host.deleted_at, host.deleted_at);
                assert_eq!(retrieved_host.deleted_by, host.deleted_by);
                assert_eq!(retrieved_host.number_of_cores, host.number_of_cores);
                assert_eq!(retrieved_host.memory_size, host.memory_size);
                assert_eq!(retrieved_host.disk_space, host.disk_space);
                assert_eq!(retrieved_host.used_number_of_cores, 0);
                assert_eq!(retrieved_host.amount_of_used_memory, 0);
                assert_eq!(retrieved_host.amount_of_used_disk_space, 0);
            }
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        hard_delete_host(&uuid1);
    }

    #[test]
    #[serial]
    fn test_add_new_host_and_update_resources() {
        let uuid1 = Uuid::new_v4();

        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: "test-project".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        hard_delete_host(&uuid1);

        let resources = HostResources {
            number_of_cores: 8,
            memory_size: 16384,
            disk_space: 512,
            project_id: None,
        };
        add_new_host(
            &uuid1,
            "Alice",
            "http://127.0.0.1:11420",
            None,
            &resources,
            &context,
        )
        .unwrap();

        let Ok(retrieved_host) = get_host(&uuid1, &context) else {
            panic!("host not found");
        };
        assert_eq!(retrieved_host.number_of_cores, 8);
        assert_eq!(retrieved_host.memory_size, 16384);
        assert_eq!(retrieved_host.disk_space, 512);
        assert_eq!(retrieved_host.used_number_of_cores, 0);
        assert_eq!(retrieved_host.amount_of_used_memory, 0);
        assert_eq!(retrieved_host.amount_of_used_disk_space, 0);

        let new_resources = HostResources {
            number_of_cores: 32,
            memory_size: 65536,
            disk_space: 2048,
            project_id: None,
        };
        assert!(update_host_resources(&uuid1, &new_resources, &context).is_ok());

        let Ok(retrieved_host) = get_host(&uuid1, &context) else {
            panic!("host not found");
        };
        assert_eq!(retrieved_host.number_of_cores, 32);
        assert_eq!(retrieved_host.memory_size, 65536);
        assert_eq!(retrieved_host.disk_space, 2048);

        // negative values are rejected by the database
        let invalid_resources = HostResources {
            number_of_cores: -1,
            memory_size: 0,
            disk_space: 0,
            project_id: None,
        };
        assert!(update_host_resources(&uuid1, &invalid_resources, &context).is_err());

        hard_delete_host(&uuid1);
        assert!(update_host_resources(&uuid1, &new_resources, &context).is_err());
    }

    #[test]
    #[serial]
    fn test_allocate_and_release_host_resources() {
        let uuid1 = Uuid::new_v4();

        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: "test-project".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        hard_delete_host(&uuid1);

        // the values are that large, that no other host of the test-database can be selected
        let resources = HostResources {
            number_of_cores: 1_000_000,
            memory_size: 1_000_000_000,
            disk_space: 1_000_000_000,
            project_id: None,
        };
        add_new_host(
            &uuid1,
            "Alice",
            "http://127.0.0.1:11420",
            None,
            &resources,
            &context,
        )
        .unwrap();

        let requested = HostResources {
            number_of_cores: 600_000,
            memory_size: 600_000_000,
            disk_space: 600_000_000,
            project_id: None,
        };

        // first allocation fits
        let Ok(host) = allocate_host_resources(&requested, &context) else {
            panic!("no host selected");
        };
        assert_eq!(host.uuid, uuid1);
        assert_eq!(host.used_number_of_cores, 600_000);
        assert_eq!(host.amount_of_used_memory, 600_000_000);
        assert_eq!(host.amount_of_used_disk_space, 600_000_000);

        // second allocation doesn't fit anymore and changes nothing
        assert!(matches!(
            allocate_host_resources(&requested, &context),
            Err(enums::DbError::NotFound)
        ));

        // a single exhausted resource is enough to exclude the host
        let only_disk = HostResources {
            number_of_cores: 1,
            memory_size: 1,
            disk_space: 400_000_001,
            project_id: None,
        };
        assert!(allocate_host_resources(&only_disk, &context).is_err());

        // the remaining resources fit exactly
        let remaining = HostResources {
            number_of_cores: 400_000,
            memory_size: 400_000_000,
            disk_space: 400_000_000,
            project_id: None,
        };
        let Ok(host) = allocate_host_resources(&remaining, &context) else {
            panic!("no host selected");
        };
        assert_eq!(host.used_number_of_cores, 1_000_000);
        assert_eq!(host.amount_of_used_memory, 1_000_000_000);
        assert_eq!(host.amount_of_used_disk_space, 1_000_000_000);

        // release the first allocation, so it fits again
        assert!(release_host_resources(&uuid1, &requested).is_ok());
        let Ok(host) = get_host(&uuid1, &context) else {
            panic!("host not found");
        };
        assert_eq!(host.used_number_of_cores, 400_000);
        assert_eq!(host.amount_of_used_memory, 400_000_000);
        assert_eq!(host.amount_of_used_disk_space, 400_000_000);
        assert!(allocate_host_resources(&requested, &context).is_ok());

        // releasing more than allocated stops at 0
        assert!(release_host_resources(&uuid1, &resources).is_ok());
        assert!(release_host_resources(&uuid1, &resources).is_ok());
        let Ok(host) = get_host(&uuid1, &context) else {
            panic!("host not found");
        };
        assert_eq!(host.used_number_of_cores, 0);
        assert_eq!(host.amount_of_used_memory, 0);
        assert_eq!(host.amount_of_used_disk_space, 0);

        // deleted hosts are not selected
        let _ = delete_host_admin(&uuid1, &context);
        assert!(allocate_host_resources(&requested, &context).is_err());

        hard_delete_host(&uuid1);
    }

    #[test]
    #[serial]
    fn test_allocate_host_resources_parallel() {
        let uuid1 = Uuid::new_v4();

        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: "test-project".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        hard_delete_host(&uuid1);

        // space for exactly 10 allocations of the requested size
        let resources = HostResources {
            number_of_cores: 2_000_000,
            memory_size: 2_000_000_000,
            disk_space: 2_000_000_000,
            project_id: None,
        };
        add_new_host(
            &uuid1,
            "Alice",
            "http://127.0.0.1:11420",
            None,
            &resources,
            &context,
        )
        .unwrap();

        let requested = HostResources {
            number_of_cores: 200_000,
            memory_size: 200_000_000,
            disk_space: 200_000_000,
            project_id: None,
        };

        // 32 parallel requests, where only 10 are allowed to succeed
        let handles: Vec<_> = (0..32)
            .map(|_| {
                let requested = requested.clone();
                let context = context.clone();
                std::thread::spawn(move || allocate_host_resources(&requested, &context).is_ok())
            })
            .collect();
        let successful = handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .filter(|ok| *ok)
            .count();
        assert_eq!(successful, 10);

        let Ok(host) = get_host(&uuid1, &context) else {
            panic!("host not found");
        };
        assert_eq!(host.used_number_of_cores, 2_000_000);
        assert_eq!(host.amount_of_used_memory, 2_000_000_000);
        assert_eq!(host.amount_of_used_disk_space, 2_000_000_000);

        hard_delete_host(&uuid1);
    }

    #[test]
    #[serial]
    fn test_allocate_host_resources_isolation() {
        let shared_uuid = Uuid::new_v4();
        let free_isolated_uuid = Uuid::new_v4();
        let bound_isolated_uuid = Uuid::new_v4();
        let project1 = "project1".to_string();
        let project2 = "project2".to_string();

        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: "test-project".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        // the values are that large, that no other host of the test-database can be selected
        let resources = HostResources {
            number_of_cores: 10_000_000,
            memory_size: 10_000_000_000,
            disk_space: 10_000_000_000,
            project_id: None,
        };
        for host_uuid in [&shared_uuid, &free_isolated_uuid, &bound_isolated_uuid] {
            hard_delete_host(host_uuid);
            add_new_host(
                host_uuid,
                "Alice",
                "http://127.0.0.1:11420",
                None,
                &resources,
                &context,
            )
            .unwrap();
        }

        // isolate two of the hosts, one of them is bound to project1
        let set_isolation = |host_uuid: &Uuid, project: Option<&str>| {
            use self::hosts::dsl::*;
            let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
            diesel::update(hosts.filter(uuid.eq(host_uuid.to_string())))
                .set((is_host_isolated.eq(true), project_id.eq(project)))
                .execute(&mut *conn)
                .unwrap();
        };
        set_isolation(&free_isolated_uuid, None);
        set_isolation(&bound_isolated_uuid, Some(&project1));

        let request = |project: Option<&String>| HostResources {
            number_of_cores: 6_000_000,
            memory_size: 6_000_000_000,
            disk_space: 6_000_000_000,
            project_id: project.cloned(),
        };

        // without project only the not isolated host is used, also if the isolated hosts are
        // still empty
        let Ok(host) = allocate_host_resources(&request(None), &context) else {
            panic!("Expected successful allocation");
        };
        assert_eq!(host.uuid, shared_uuid);
        assert!(allocate_host_resources(&request(None), &context).is_err());

        // free the not isolated host again, so the requests with project below could use it,
        // if the isolation would be ignored
        assert!(release_host_resources(&shared_uuid, &request(None)).is_ok());

        // project1 prefers the host bound to it over the isolated host without project
        let Ok(host) = allocate_host_resources(&request(Some(&project1)), &context) else {
            panic!("Expected successful allocation");
        };
        assert_eq!(host.uuid, bound_isolated_uuid);
        assert_eq!(host.project_id.as_ref(), Some(&project1));

        // project2 gets the isolated host without project, which is bound to it afterwards
        let Ok(host) = allocate_host_resources(&request(Some(&project2)), &context) else {
            panic!("Expected successful allocation");
        };
        assert_eq!(host.uuid, free_isolated_uuid);
        assert_eq!(host.project_id.as_ref(), Some(&project2));

        // the host bound to project1 is full and the other one is bound to project2 now. The not
        // isolated host has enough free resources, but is never used for a project
        assert!(allocate_host_resources(&request(Some(&project1)), &context).is_err());
        assert!(allocate_host_resources(&request(Some(&project2)), &context).is_err());
        assert!(get_host(&shared_uuid, &context).is_ok_and(|h| h.used_number_of_cores == 0));

        for host_uuid in [&shared_uuid, &free_isolated_uuid, &bound_isolated_uuid] {
            hard_delete_host(host_uuid);
        }
    }

    #[test]
    #[serial]
    fn test_set_host_isolation() {
        let uuid1 = Uuid::new_v4();

        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: "test-project".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        hard_delete_host(&uuid1);

        // the values are that large, that no other host of the test-database can be selected
        let resources = HostResources {
            number_of_cores: 10_000_000,
            memory_size: 10_000_000_000,
            disk_space: 10_000_000_000,
            project_id: None,
        };
        add_new_host(
            &uuid1,
            "Alice",
            "http://127.0.0.1:11420",
            None,
            &resources,
            &context,
        )
        .unwrap();

        // an unused host can be isolated and released again
        assert!(matches!(
            set_host_isolation(&uuid1, true, &context),
            Ok(true)
        ));
        assert!(get_host(&uuid1, &context).is_ok_and(|h| h.is_host_isolated));
        assert!(matches!(
            set_host_isolation(&uuid1, false, &context),
            Ok(true)
        ));
        assert!(get_host(&uuid1, &context).is_ok_and(|h| !h.is_host_isolated));

        // not allowed, while resources are allocated on the host
        let requested = HostResources {
            number_of_cores: 1,
            memory_size: 1,
            disk_space: 1,
            project_id: None,
        };
        let Ok(host) = allocate_host_resources(&requested, &context) else {
            panic!("Expected successful allocation");
        };
        assert_eq!(host.uuid, uuid1);
        assert!(matches!(
            set_host_isolation(&uuid1, true, &context),
            Ok(false)
        ));
        assert!(get_host(&uuid1, &context).is_ok_and(|h| !h.is_host_isolated));
        assert!(release_host_resources(&uuid1, &requested).is_ok());

        // not allowed, while the host is bound to a project
        assert!(matches!(
            set_host_isolation(&uuid1, true, &context),
            Ok(true)
        ));
        let requested = HostResources {
            project_id: Some("project1".to_string()),
            ..requested
        };
        assert!(allocate_host_resources(&requested, &context).is_ok());
        assert!(allocate_host_resources(&requested, &context).is_ok());
        assert!(matches!(
            set_host_isolation(&uuid1, false, &context),
            Ok(false)
        ));
        assert!(get_host(&uuid1, &context).is_ok_and(|h| h.is_host_isolated));

        // the host stays bound to the project, as long as resources are allocated on it
        assert!(release_host_resources(&uuid1, &requested).is_ok());
        assert!(
            get_host(&uuid1, &context).is_ok_and(|h| h.project_id.as_deref() == Some("project1"))
        );

        // releasing the last resources unbinds the host from the project, so it can be changed
        assert!(release_host_resources(&uuid1, &requested).is_ok());
        assert!(get_host(&uuid1, &context).is_ok_and(|h| h.project_id.is_none()));
        assert!(matches!(
            set_host_isolation(&uuid1, false, &context),
            Ok(true)
        ));

        // unknown host
        assert!(matches!(
            set_host_isolation(&Uuid::new_v4(), true, &context),
            Err(enums::DbError::NotFound)
        ));

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
            number_of_cores: 16,
            used_number_of_cores: 0,
            memory_size: 32768,
            amount_of_used_memory: 0,
            disk_space: 1024,
            amount_of_used_disk_space: 0,
            is_host_isolated: false,
            project_id: None,
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
            external_address: None,
        };

        let host2 = HostEntry {
            uuid: uuid2,
            name: "Bob".to_string(),
            address: "http://127.0.0.1:11420".to_string(),
            number_of_cores: 16,
            used_number_of_cores: 0,
            memory_size: 32768,
            amount_of_used_memory: 0,
            disk_space: 1024,
            amount_of_used_disk_space: 0,
            is_host_isolated: false,
            project_id: None,
            status: "DELETED".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
            external_address: None,
        };

        hard_delete_host(&uuid1);
        hard_delete_host(&uuid2);

        add_host(host1).unwrap();
        add_host(host2).unwrap();
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
            number_of_cores: 16,
            used_number_of_cores: 0,
            memory_size: 32768,
            amount_of_used_memory: 0,
            disk_space: 1024,
            amount_of_used_disk_space: 0,
            is_host_isolated: false,
            project_id: None,
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
            external_address: None,
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
            number_of_cores: 16,
            used_number_of_cores: 0,
            memory_size: 32768,
            amount_of_used_memory: 0,
            disk_space: 1024,
            amount_of_used_disk_space: 0,
            is_host_isolated: false,
            project_id: None,
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
            external_address: None,
        };

        let host2 = HostEntry {
            uuid: uuid2,
            name: "Bob".to_string(),
            address: "http://127.0.0.1:11420".to_string(),
            number_of_cores: 16,
            used_number_of_cores: 0,
            memory_size: 32768,
            amount_of_used_memory: 0,
            disk_space: 1024,
            amount_of_used_disk_space: 0,
            is_host_isolated: false,
            project_id: None,
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
            external_address: None,
        };

        let host3 = HostEntry {
            uuid: uuid3,
            name: "Poi".to_string(),
            address: "http://127.0.0.1:11420".to_string(),
            number_of_cores: 16,
            used_number_of_cores: 0,
            memory_size: 32768,
            amount_of_used_memory: 0,
            disk_space: 1024,
            amount_of_used_disk_space: 0,
            is_host_isolated: false,
            project_id: None,
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
            external_address: None,
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
}
