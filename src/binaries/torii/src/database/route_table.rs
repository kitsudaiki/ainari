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
use std::net::Ipv4Addr;
use uuid::Uuid;

use crate::core::models::Route;
use crate::database::{db_handle, network_filter_table};

use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;
use ainari_common::objects::*;

// Define the schema for the routes table
table! {
    routes (uuid) {
        uuid -> Varchar,
        vni -> Integer,
        dest_ip -> Varchar,
        target_iface -> Varchar,
        gateway_ip -> Nullable<Varchar>,
        next_hop_ip -> Nullable<Varchar>,
        next_hop_mac -> Nullable<Varchar>,
        encrypted -> Bool,
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

/// Represents a single entry in the routes table.
/// Contains all fields necessary to re-program a route into the eBPF maps after a restart.
///
/// # Fields
/// * `uuid` - Unique identifier of the route
/// * `vni` - Tenant the route belongs to
/// * `dest_ip` - Destination address of the route
/// * `target_iface` - Interface the matching packets leave on
/// * `gateway_ip` - Underlay address of the remote gateway for tunnel routes (nullable)
/// * `next_hop_ip` - Address of the link layer next hop (nullable)
/// * `next_hop_mac` - Explicit MAC of the link layer next hop (nullable)
/// * `encrypted` - True, if the destination is reached over IPsec
/// * `owner_id` - User ID of the route owner
/// * `project_id` - Project ID the route belongs to
/// * `status` - Current status of the route (ACTIVE, DELETED, etc.)
/// * `created_at` - Timestamp when the route was created
/// * `created_by` - User ID who created the route
/// * `updated_at` - Timestamp when the route was last updated
/// * `updated_by` - User ID who last updated the route
/// * `deleted_at` - Timestamp when the route was deleted (nullable)
/// * `deleted_by` - User ID who deleted the route (nullable)
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = routes)]
pub struct RouteEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    #[diesel(serialize_as = DbVni, deserialize_as = DbVni)]
    pub vni: u32,
    #[diesel(serialize_as = DbIpv4Addr, deserialize_as = DbIpv4Addr)]
    pub dest_ip: Ipv4Addr,
    pub target_iface: String,
    #[diesel(serialize_as = DbOptIpv4Addr, deserialize_as = DbOptIpv4Addr)]
    pub gateway_ip: Option<Ipv4Addr>,
    #[diesel(serialize_as = DbOptIpv4Addr, deserialize_as = DbOptIpv4Addr)]
    pub next_hop_ip: Option<Ipv4Addr>,
    pub next_hop_mac: Option<String>,
    pub encrypted: bool,
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

impl From<RouteEntry> for Route {
    fn from(entry: RouteEntry) -> Self {
        Route {
            uuid: entry.uuid,
            vni: entry.vni,
            dest_ip: entry.dest_ip,
            target_iface: entry.target_iface,
            gateway_ip: entry.gateway_ip,
            next_hop_ip: entry.next_hop_ip,
            next_hop_mac: entry.next_hop_mac,
            encrypted: entry.encrypted,
        }
    }
}

/// Adds a new route entry to the database with default values for a newly created route.
///
/// # Arguments
/// * `route` - The route, which was programmed into the datapath
/// * `context` - User context containing ownership and project information
///
/// # Returns
/// * `QueryResult<usize>` indicating the number of rows affected
pub fn add_new_route(route: &Route, context: &UserContext) -> QueryResult<usize> {
    let entry = RouteEntry {
        uuid: route.uuid,
        vni: route.vni,
        dest_ip: route.dest_ip,
        target_iface: route.target_iface.clone(),
        gateway_ip: route.gateway_ip,
        next_hop_ip: route.next_hop_ip,
        next_hop_mac: route.next_hop_mac.clone(),
        encrypted: route.encrypted,
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

    add_route(entry)
}

/// Adds a route entry to the database.
///
/// # Arguments
/// * `route` - The RouteEntry to be added
///
/// # Returns
/// * `QueryResult<usize>` indicating the number of rows affected
pub fn add_route(route: RouteEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::routes::dsl::*;
    diesel::insert_into(routes)
        .values(route)
        .execute(&mut *conn)
}

/// Retrieves an active route entry from the database based on its UUID.
///
/// # Arguments
/// * `route_uuid` - UUID of the route to retrieve
///
/// # Returns
/// * `Result<RouteEntry, enums::DbError>` containing the route if found, or an appropriate error
#[cfg(test)]
pub fn get_route(route_uuid: &Uuid) -> Result<RouteEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::routes::dsl::*;

    match routes
        .filter(uuid.eq(route_uuid.to_string()).and(status.eq("ACTIVE")))
        .select(RouteEntry::as_select())
        .first::<RouteEntry>(&mut *conn)
    {
        Ok(route) => Ok(route),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Lists all active route entries, ordered by their last update.
///
/// Routes are re-programmed in this order after a restart, so for two routes with the same
/// `(vni, dest_ip)` the last written one ends up in the eBPF map, like before the restart.
///
/// # Returns
/// * `QueryResult<Vec<RouteEntry>>` containing all active routes
pub fn list_routes() -> QueryResult<Vec<RouteEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::routes::dsl::*;

    let mut entries = routes
        .filter(status.eq("ACTIVE"))
        .select(RouteEntry::as_select())
        .load(&mut *conn)?;
    entries.sort_by_key(|entry| entry.updated_at);
    Ok(entries)
}

/// Overwrites the values of an active route in the database.
///
/// # Arguments
/// * `route` - The updated route, which was programmed into the datapath
/// * `context` - User context to record who performed the update
///
/// # Returns
/// * `Result<(), enums::DbError>` indicating success or failure
pub fn update_route(route: &Route, context: &UserContext) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::routes::dsl::*;
    match diesel::update(routes.filter(uuid.eq(route.uuid.to_string()).and(status.eq("ACTIVE"))))
        .set((
            vni.eq(route.vni as i32),
            dest_ip.eq(route.dest_ip.to_string()),
            target_iface.eq(route.target_iface.clone()),
            gateway_ip.eq(route.gateway_ip.map(|ip| ip.to_string())),
            next_hop_ip.eq(route.next_hop_ip.map(|ip| ip.to_string())),
            next_hop_mac.eq(route.next_hop_mac.clone()),
            encrypted.eq(route.encrypted),
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

/// Marks a route and all rules of its packet filter as deleted in the database.
///
/// Both happen within one transaction, so a route is never deleted without its filter or the
/// other way around.
///
/// # Arguments
/// * `route_uuid` - UUID of the route to delete
/// * `context` - User context to record who performed the deletion
///
/// # Returns
/// * `Result<(), enums::DbError>` indicating success or failure. `NotFound` means, that the
///   route itself had no active entry. Its filter-rules are deleted nevertheless.
pub fn delete_route(route_uuid: &Uuid, context: &UserContext) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    let result = conn.transaction(|conn| {
        network_filter_table::delete_filter_rules_in(conn, route_uuid, context)?;
        use self::routes::dsl::*;
        diesel::update(routes.filter(uuid.eq(route_uuid.to_string()).and(status.eq("ACTIVE"))))
            .set((
                status.eq("DELETED"),
                deleted_at.eq(Utc::now().to_rfc3339()),
                deleted_by.eq(context.user_id.clone()),
            ))
            .execute(conn)
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

#[cfg(test)]
mod tests {
    use super::*;
    use ainari_api_structs::network_filter_structs::RouteFilterRules;
    use serial_test::serial;

    fn hard_delete_route(route_uuid: &Uuid) {
        use self::routes::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(routes.filter(uuid.eq(route_uuid.to_string()))).execute(&mut *conn);
    }

    fn test_context() -> UserContext {
        UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: "test-project".to_string(),
            is_admin: true.to_string(),
            is_project_admin: true.to_string(),
        }
    }

    fn test_route(route_uuid: Uuid) -> Route {
        Route {
            uuid: route_uuid,
            vni: 42,
            dest_ip: Ipv4Addr::new(192, 168, 100, 5),
            target_iface: "tap-test".to_string(),
            gateway_ip: Some(Ipv4Addr::new(172, 30, 0, 21)),
            next_hop_ip: None,
            next_hop_mac: Some("aa:bb:cc:dd:ee:ff".to_string()),
            encrypted: true,
        }
    }

    #[test]
    #[serial]
    fn test_add_get_route() {
        let route_uuid = Uuid::new_v4();
        let route = test_route(route_uuid);
        hard_delete_route(&route_uuid);

        add_new_route(&route, &test_context()).unwrap();
        let entry = get_route(&route_uuid).ok().unwrap();
        assert_eq!(entry.created_by, "test-user");
        let restored: Route = entry.into();
        assert_eq!(restored.uuid, route.uuid);
        assert_eq!(restored.vni, route.vni);
        assert_eq!(restored.dest_ip, route.dest_ip);
        assert_eq!(restored.target_iface, route.target_iface);
        assert_eq!(restored.gateway_ip, route.gateway_ip);
        assert_eq!(restored.next_hop_ip, route.next_hop_ip);
        assert_eq!(restored.next_hop_mac, route.next_hop_mac);
        assert_eq!(restored.encrypted, route.encrypted);

        hard_delete_route(&route_uuid);
    }

    #[test]
    #[serial]
    fn test_update_route() {
        let route_uuid = Uuid::new_v4();
        let mut route = test_route(route_uuid);
        hard_delete_route(&route_uuid);

        add_new_route(&route, &test_context()).unwrap();
        route.dest_ip = Ipv4Addr::new(192, 168, 100, 6);
        route.gateway_ip = None;
        route.encrypted = false;
        assert!(update_route(&route, &test_context()).is_ok());

        let entry = get_route(&route_uuid).ok().unwrap();
        assert_eq!(entry.dest_ip, route.dest_ip);
        assert_eq!(entry.gateway_ip, None);
        assert!(!entry.encrypted);

        assert!(update_route(&test_route(Uuid::new_v4()), &test_context()).is_err());

        hard_delete_route(&route_uuid);
    }

    #[test]
    #[serial]
    fn test_delete_route() {
        let route_uuid = Uuid::new_v4();
        hard_delete_route(&route_uuid);

        add_new_route(&test_route(route_uuid), &test_context()).unwrap();
        assert!(list_routes().unwrap().iter().any(|r| r.uuid == route_uuid));
        let rules = RouteFilterRules {
            ip_ranges: vec!["10.0.0.0/24".parse().unwrap()],
            ports: vec!["22".parse().unwrap()],
        };
        network_filter_table::set_filter_rules(&route_uuid, &rules, &test_context()).unwrap();

        // the packet-filter is deleted together with its route
        assert!(delete_route(&route_uuid, &test_context()).is_ok());
        assert!(get_route(&route_uuid).is_err());
        let filters = network_filter_table::list_filter_rules().unwrap();
        assert!(!filters.contains_key(&route_uuid));
        assert!(!list_routes().unwrap().iter().any(|r| r.uuid == route_uuid));
        assert!(delete_route(&route_uuid, &test_context()).is_err());

        hard_delete_route(&route_uuid);
    }
}
