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
use diesel::result::DatabaseErrorKind;
use uuid::Uuid;

use crate::database::db_handle;

use ainari_api_structs::network_filter_structs::{FilterDirection, RouteFilterRules};
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;
use ainari_common::objects::*;

/// Separator of the rules within the `ip_ranges`- and `ports`-columns. The canonical notation of
/// a rule never contains it.
const RULE_SEPARATOR: &str = ",";

// Define the schema for network_filters table
table! {
    network_filters (uuid) {
        uuid -> Varchar,
        virtual_machine_uuid -> Varchar,
        direction -> Varchar,
        ip_ranges -> Varchar,
        ports -> Varchar,
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

/// Represents an entry in the network_filters table.
///
/// It is the view of hanami on the packet filter of one direction of a virtual_machine, which is
/// applied by the torii of the host of the virtual_machine. The include-lists are stored in their
/// canonical notation, separated by commas.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = network_filters)]
pub struct NetworkFilterEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub virtual_machine_uuid: Uuid,
    /// Direction of the filter, `ingress` or `egress`
    pub direction: String,
    /// Include-list of the ip-ranges, like `10.0.0.0/24,10.0.1.5`
    pub ip_ranges: String,
    /// Include-list of the ports, like `22,8000-8100`
    pub ports: String,
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

impl NetworkFilterEntry {
    /// Splits the stored include-list of the ip-ranges into its rules.
    ///
    /// # Returns
    /// The ip-ranges in their canonical notation
    pub fn ip_range_list(&self) -> Vec<String> {
        split_rules(&self.ip_ranges)
    }

    /// Splits the stored include-list of the ports into its rules.
    ///
    /// # Returns
    /// The ports in their canonical notation
    pub fn port_list(&self) -> Vec<String> {
        split_rules(&self.ports)
    }
}

/// Splits a stored include-list into its rules.
///
/// # Arguments
/// * `rules` - The stored include-list
///
/// # Returns
/// The rules of the list, which is empty for an empty string
fn split_rules(rules: &str) -> Vec<String> {
    rules
        .split(RULE_SEPARATOR)
        .filter(|rule| !rule.is_empty())
        .map(str::to_string)
        .collect()
}

/// Stores the include-lists of a packet filter of a virtual_machine.
///
/// The include-lists are the ones, which the torii reported back after it applied the filter,
/// so they replace the stored ones completely. An existing entry is updated, otherwise a new one
/// is created. A filter with two empty include-lists doesn't restrict anything, so its entry is
/// marked as deleted instead.
///
/// There is no permission-check of the virtual_machine, so the caller has to verify, that the
/// user is allowed to access it.
///
/// # Arguments
/// * `filter_virtual_machine_uuid` - The UUID of the virtual_machine
/// * `filter_direction` - The direction of the filter
/// * `rules` - The include-lists of the filter
/// * `context` - The user context containing information about the user and project
///
/// # Returns
/// A Result containing the stored entry, which is marked as deleted for empty include-lists, or
/// a DbError if the entry could not be stored
pub fn set_network_filter(
    filter_virtual_machine_uuid: &Uuid,
    filter_direction: FilterDirection,
    rules: &RouteFilterRules,
    context: &UserContext,
) -> Result<NetworkFilterEntry, enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    let new_ip_ranges = join_rules(rules.ip_ranges.iter().map(|rule| rule.spec.as_str()));
    let new_ports = join_rules(rules.ports.iter().map(|rule| rule.spec.as_str()));

    // a parallel request may have created the entry between the update and the insert, so the
    // update is tried a second time in that case
    for _ in 0..2 {
        let updated = update_active_entry(
            filter_virtual_machine_uuid,
            filter_direction,
            &new_ip_ranges,
            &new_ports,
            rules.is_empty(),
            context,
        )
        .map_err(log_db_error)?;
        if let Some(entry) = updated {
            return Ok(entry);
        }

        // an unrestricted filter without an entry has nothing to store
        let entry = NetworkFilterEntry {
            uuid: Uuid::new_v4(),
            virtual_machine_uuid: *filter_virtual_machine_uuid,
            direction: filter_direction.to_string(),
            ip_ranges: new_ip_ranges.clone(),
            ports: new_ports.clone(),
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
        if rules.is_empty() {
            return Ok(NetworkFilterEntry {
                status: "DELETED".to_string(),
                ..entry
            });
        }

        match add_network_filter(entry.clone()) {
            Ok(_) => return Ok(entry),
            Err(diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
                continue;
            }
            Err(e) => return Err(log_db_error(e)),
        }
    }

    log::error!(
        "Failed to store the {filter_direction} packet-filter of virtual_machine \
         '{filter_virtual_machine_uuid}', because of concurrent updates"
    );
    Err(enums::DbError::InternalError)
}

/// Updates the include-lists of the active entry of a packet filter.
///
/// # Arguments
/// * `filter_virtual_machine_uuid` - The UUID of the virtual_machine
/// * `filter_direction` - The direction of the filter
/// * `new_ip_ranges` - The stored include-list of the ip-ranges
/// * `new_ports` - The stored include-list of the ports
/// * `delete` - True to mark the entry as deleted
/// * `context` - The user context of the user, who updates the filter
///
/// # Returns
/// A QueryResult containing the updated entry, or None if there is no active entry
fn update_active_entry(
    filter_virtual_machine_uuid: &Uuid,
    filter_direction: FilterDirection,
    new_ip_ranges: &str,
    new_ports: &str,
    delete: bool,
    context: &UserContext,
) -> QueryResult<Option<NetworkFilterEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::network_filters::dsl::*;

    let existing = network_filters
        .filter(
            virtual_machine_uuid
                .eq(filter_virtual_machine_uuid.to_string())
                .and(direction.eq(filter_direction.to_string()))
                .and(status.eq("ACTIVE")),
        )
        .select(NetworkFilterEntry::as_select())
        .first::<NetworkFilterEntry>(&mut *conn)
        .optional()?;
    let Some(mut entry) = existing else {
        return Ok(None);
    };

    let now = Utc::now();
    entry.ip_ranges = new_ip_ranges.to_string();
    entry.ports = new_ports.to_string();
    entry.updated_at = now;
    entry.updated_by = context.user_id.clone();
    if delete {
        entry.status = "DELETED".to_string();
        entry.deleted_at = Some(now);
        entry.deleted_by = Some(context.user_id.clone());
    }

    diesel::update(network_filters.filter(uuid.eq(entry.uuid.to_string())))
        .set((
            ip_ranges.eq(&entry.ip_ranges),
            ports.eq(&entry.ports),
            status.eq(&entry.status),
            updated_at.eq(now.to_rfc3339()),
            updated_by.eq(&entry.updated_by),
            deleted_at.eq(entry.deleted_at.map(|time| time.to_rfc3339())),
            deleted_by.eq(&entry.deleted_by),
        ))
        .execute(&mut *conn)?;

    Ok(Some(entry))
}

/// Joins rules into a stored include-list.
///
/// # Arguments
/// * `rules` - The rules in their canonical notation
///
/// # Returns
/// The include-list as it is stored in the database
fn join_rules<'a>(rules: impl Iterator<Item = &'a str>) -> String {
    rules.collect::<Vec<_>>().join(RULE_SEPARATOR)
}

/// Logs a database-error and converts it into a DbError.
///
/// # Arguments
/// * `e` - The error of the database
///
/// # Returns
/// The DbError, which is handed to the caller
fn log_db_error(e: diesel::result::Error) -> enums::DbError {
    log::error!("Database-error: {e:?}");
    enums::DbError::InternalError
}

/// Adds a network_filter-entry to the database.
///
/// This is a helper function that performs the actual insertion of a NetworkFilterEntry into the
/// database.
///
/// # Arguments
/// * `network_filter` - The NetworkFilterEntry to be inserted
///
/// # Returns
/// A QueryResult indicating the number of rows affected
pub fn add_network_filter(network_filter: NetworkFilterEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::network_filters::dsl::*;
    diesel::insert_into(network_filters)
        .values(network_filter)
        .execute(&mut *conn)
}

/// Retrieves the packet filter of one direction of a virtual_machine from the database.
///
/// Only active entries are returned, and the query is filtered based on the user's role and
/// project membership.
///
/// # Arguments
/// * `filter_virtual_machine_uuid` - The UUID of the virtual_machine
/// * `filter_direction` - The direction of the filter
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A Result containing the NetworkFilterEntry if found, or a DbError if not found or an error
/// occurs
pub fn get_network_filter(
    filter_virtual_machine_uuid: &Uuid,
    filter_direction: FilterDirection,
    context: &UserContext,
) -> Result<NetworkFilterEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::network_filters::dsl::*;

    let mut query = network_filters
        .filter(
            virtual_machine_uuid
                .eq(filter_virtual_machine_uuid.to_string())
                .and(direction.eq(filter_direction.to_string()))
                .and(status.eq("ACTIVE")),
        )
        .into_boxed();

    // Apply permission-based filtering
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
    }

    match query
        .select(NetworkFilterEntry::as_select())
        .first::<NetworkFilterEntry>(&mut *conn)
    {
        Ok(network_filter) => Ok(network_filter),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Lists all packet filters that the user has access to.
///
/// This function retrieves all active packet filters and applies permission-based filtering.
/// The results are filtered based on the user's role and project membership.
///
/// # Arguments
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A QueryResult containing a vector of NetworkFilterEntry objects
pub fn list_network_filters(context: &UserContext) -> QueryResult<Vec<NetworkFilterEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::network_filters::dsl::*;

    let mut query = network_filters.filter(status.eq("ACTIVE")).into_boxed();

    // Apply permission-based filtering
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
    }

    query
        .select(NetworkFilterEntry::as_select())
        .load(&mut *conn)
}

/// Marks the packet filters of both directions of a virtual_machine as deleted.
///
/// The torii drops the filters together with the route towards the virtual_machine, so this only
/// follows the deletion of the virtual_machine. There is no permission-check of the
/// virtual_machine, so the caller has to verify, that the user is allowed to delete it.
///
/// # Arguments
/// * `filter_virtual_machine_uuid` - The UUID of the deleted virtual_machine
/// * `context` - The user context of the user, who deleted the virtual_machine
///
/// # Returns
/// A Result indicating success or an error
pub fn delete_network_filters_of_virtual_machine(
    filter_virtual_machine_uuid: &Uuid,
    context: &UserContext,
) -> Result<(), enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::network_filters::dsl::*;
    match diesel::update(
        network_filters.filter(
            virtual_machine_uuid
                .eq(filter_virtual_machine_uuid.to_string())
                .and(status.eq("ACTIVE")),
        ),
    )
    .set((
        status.eq("DELETED"),
        deleted_at.eq(Utc::now().to_rfc3339()),
        deleted_by.eq(context.user_id.clone()),
    ))
    .execute(&mut *conn)
    {
        Ok(_) => Ok(()),
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

    fn hard_delete_network_filters(filter_virtual_machine_uuid: &Uuid) {
        use self::network_filters::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(
            network_filters
                .filter(virtual_machine_uuid.eq(filter_virtual_machine_uuid.to_string())),
        )
        .execute(&mut *conn);
    }

    fn new_context(
        user_id: &str,
        project_id: &str,
        is_admin: bool,
        project_role: ProjectRole,
    ) -> UserContext {
        UserContext {
            token: "".to_string(),
            user_id: user_id.to_string(),
            project_id: project_id.to_string(),
            is_admin: is_admin.to_string(),
            project_role: project_role.to_string(),
        }
    }

    fn test_rules(ip_ranges: &[&str], ports: &[&str]) -> RouteFilterRules {
        RouteFilterRules {
            ip_ranges: ip_ranges.iter().map(|rule| rule.parse().unwrap()).collect(),
            ports: ports.iter().map(|rule| rule.parse().unwrap()).collect(),
        }
    }

    /// Unwraps a get-result. `enums::DbError` implements neither `Debug` nor `PartialEq`,
    /// so the results can not be handled by `expect` and `assert_eq`.
    fn expect_entry(result: Result<NetworkFilterEntry, enums::DbError>) -> NetworkFilterEntry {
        match result {
            Ok(entry) => entry,
            Err(_) => panic!("expected an entry"),
        }
    }

    #[test]
    #[serial]
    fn test_set_and_get_network_filter() {
        let vm_uuid = Uuid::new_v4();
        hard_delete_network_filters(&vm_uuid);
        let context = new_context("test-user", "test-project", false, ProjectRole::Member);

        // the first include-list creates the entry
        let rules = test_rules(&["10.0.0.0/24", "10.0.1.5"], &[]);
        let created = expect_entry(set_network_filter(
            &vm_uuid,
            FilterDirection::Ingress,
            &rules,
            &context,
        ));
        assert_eq!(created.status, "ACTIVE");
        assert_eq!(created.ip_range_list(), vec!["10.0.0.0/24", "10.0.1.5"]);
        assert!(created.port_list().is_empty());

        // the next one updates the same entry
        let rules = test_rules(&["10.0.0.0/24"], &["22", "8000-8100"]);
        let updated = expect_entry(set_network_filter(
            &vm_uuid,
            FilterDirection::Ingress,
            &rules,
            &context,
        ));
        assert_eq!(updated.uuid, created.uuid);

        let entry = expect_entry(get_network_filter(
            &vm_uuid,
            FilterDirection::Ingress,
            &context,
        ));
        assert_eq!(entry.uuid, created.uuid);
        assert_eq!(entry.ip_range_list(), vec!["10.0.0.0/24"]);
        assert_eq!(entry.port_list(), vec!["22", "8000-8100"]);
        assert_eq!(entry.created_by, "test-user");

        // the other direction is a filter of its own
        assert!(get_network_filter(&vm_uuid, FilterDirection::Egress, &context).is_err());

        // empty include-lists remove the filter
        let cleared = expect_entry(set_network_filter(
            &vm_uuid,
            FilterDirection::Ingress,
            &RouteFilterRules::default(),
            &context,
        ));
        assert_eq!(cleared.status, "DELETED");
        assert!(get_network_filter(&vm_uuid, FilterDirection::Ingress, &context).is_err());

        // clearing a filter, which doesn't exist, leaves nothing behind
        let cleared = expect_entry(set_network_filter(
            &vm_uuid,
            FilterDirection::Egress,
            &RouteFilterRules::default(),
            &context,
        ));
        assert_eq!(cleared.status, "DELETED");
        assert!(get_network_filter(&vm_uuid, FilterDirection::Egress, &context).is_err());

        hard_delete_network_filters(&vm_uuid);
    }

    #[test]
    #[serial]
    fn test_network_filter_permissions() {
        let vm_uuid = Uuid::new_v4();
        hard_delete_network_filters(&vm_uuid);
        let context = new_context("test-user", "test-project", false, ProjectRole::Member);
        let other_project = new_context("other-user", "other-project", false, ProjectRole::Member);
        let observer = new_context("observer", "test-project", false, ProjectRole::Observer);
        let admin = new_context("admin", "admin-project", true, ProjectRole::Member);
        let rules = test_rules(&[], &["443"]);

        // observers are only allowed to read
        assert!(set_network_filter(&vm_uuid, FilterDirection::Egress, &rules, &observer).is_err());

        expect_entry(set_network_filter(
            &vm_uuid,
            FilterDirection::Egress,
            &rules,
            &context,
        ));

        // the filter is visible within its project and for admins only
        assert!(get_network_filter(&vm_uuid, FilterDirection::Egress, &observer).is_ok());
        assert!(get_network_filter(&vm_uuid, FilterDirection::Egress, &admin).is_ok());
        assert!(get_network_filter(&vm_uuid, FilterDirection::Egress, &other_project).is_err());

        let listed = |context: &UserContext| {
            list_network_filters(context)
                .unwrap()
                .iter()
                .any(|entry| entry.virtual_machine_uuid == vm_uuid)
        };
        assert!(listed(&context));
        assert!(listed(&admin));
        assert!(!listed(&other_project));

        hard_delete_network_filters(&vm_uuid);
    }

    #[test]
    #[serial]
    fn test_delete_network_filters_of_virtual_machine() {
        let vm_uuid = Uuid::new_v4();
        hard_delete_network_filters(&vm_uuid);
        let context = new_context("test-user", "test-project", false, ProjectRole::Member);
        let rules = test_rules(&["10.0.0.7"], &[]);

        for filter_direction in [FilterDirection::Ingress, FilterDirection::Egress] {
            expect_entry(set_network_filter(
                &vm_uuid,
                filter_direction,
                &rules,
                &context,
            ));
        }

        assert!(delete_network_filters_of_virtual_machine(&vm_uuid, &context).is_ok());
        for filter_direction in [FilterDirection::Ingress, FilterDirection::Egress] {
            assert!(get_network_filter(&vm_uuid, filter_direction, &context).is_err());
        }

        // a new filter can be created again after the deletion
        let entry = expect_entry(set_network_filter(
            &vm_uuid,
            FilterDirection::Ingress,
            &rules,
            &context,
        ));
        assert_eq!(entry.status, "ACTIVE");

        hard_delete_network_filters(&vm_uuid);
    }
}
