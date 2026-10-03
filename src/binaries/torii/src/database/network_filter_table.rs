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
use std::collections::HashMap;
use std::net::Ipv4Addr;
use uuid::Uuid;

use crate::core::models::FilterKey;
use crate::database::db_handle;

use ainari_api_structs::network_filter_structs::{FilterDirection, RouteFilterRules};
use ainari_api_structs::user_context::UserContext;
use ainari_common::objects::*;

/// Rule-type of an entry of the IP-range include-list of a filter.
const RULE_TYPE_IP_RANGE: &str = "IP_RANGE";
/// Rule-type of an entry of the port include-list of a filter.
const RULE_TYPE_PORT: &str = "PORT";

// Define the schema for the network_filters table
table! {
    network_filters (uuid) {
        uuid -> Varchar,
        vni -> Integer,
        ip -> Varchar,
        direction -> Varchar,
        rule_type -> Varchar,
        spec -> Varchar,
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

/// Represents a single entry in the network_filters table.
/// Each entry is one rule of the include-lists of the packet filter of one direction of an
/// address within a tenant.
///
/// # Fields
/// * `uuid` - Unique identifier of the rule
/// * `vni` - Tenant of the address the rule belongs to
/// * `ip` - Address the rule belongs to
/// * `direction` - Direction of the filter the rule belongs to (ingress or egress)
/// * `rule_type` - Include-list the rule belongs to (IP_RANGE or PORT)
/// * `spec` - Canonical textual form of the rule, like `10.0.0.0/24` or `8000-8100`
/// * `owner_id` - User ID of the rule owner
/// * `project_id` - Project ID the rule belongs to
/// * `status` - Current status of the rule (ACTIVE, DELETED, etc.)
/// * `created_at` - Timestamp when the rule was created
/// * `created_by` - User ID who created the rule
/// * `updated_at` - Timestamp when the rule was last updated
/// * `updated_by` - User ID who last updated the rule
/// * `deleted_at` - Timestamp when the rule was deleted (nullable)
/// * `deleted_by` - User ID who deleted the rule (nullable)
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = network_filters)]
pub struct NetworkFilterEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    #[diesel(serialize_as = DbVni, deserialize_as = DbVni)]
    pub vni: u32,
    #[diesel(serialize_as = DbIpv4Addr, deserialize_as = DbIpv4Addr)]
    pub ip: Ipv4Addr,
    pub direction: String,
    pub rule_type: String,
    pub spec: String,
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

/// Replaces the persisted packet filter of one direction of an address by a new set of
/// include-lists.
///
/// All active rules of the filter are marked as deleted and the new rules are inserted within one
/// transaction, so the database never holds a half-written filter. Empty include-lists leave the
/// filter without any active rule, which is the unfiltered state.
///
/// # Arguments
/// * `key` - Tenant, address and direction of the filter
/// * `rules` - The include-lists the filter has from now on
/// * `context` - User context containing ownership and project information
///
/// # Returns
/// * `QueryResult<()>` indicating success or failure
pub fn set_filter_rules(
    key: &FilterKey,
    rules: &RouteFilterRules,
    context: &UserContext,
) -> QueryResult<()> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(ainari_common::enums::permission_denied_error());
    }

    let ip_ranges = rules
        .ip_ranges
        .iter()
        .map(|rule| (RULE_TYPE_IP_RANGE, rule.spec.clone()));
    let ports = rules
        .ports
        .iter()
        .map(|rule| (RULE_TYPE_PORT, rule.spec.clone()));
    let entries: Vec<NetworkFilterEntry> = ip_ranges
        .chain(ports)
        .map(|(filter_rule_type, filter_spec)| NetworkFilterEntry {
            uuid: Uuid::new_v4(),
            vni: key.vni,
            ip: key.ip,
            direction: key.direction.to_string(),
            rule_type: filter_rule_type.to_string(),
            spec: filter_spec,
            owner_id: context.user_id.clone(),
            project_id: context.project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: context.user_id.clone(),
            updated_at: Utc::now(),
            updated_by: context.user_id.clone(),
            deleted_at: None,
            deleted_by: None,
        })
        .collect();

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.transaction(|conn| {
        delete_filter_rules_in(conn, key.vni, key.ip, Some(key.direction), context)?;
        use self::network_filters::dsl::*;
        diesel::insert_into(network_filters)
            .values(entries)
            .execute(conn)?;
        Ok(())
    })
}

/// Marks the rules of the packet filters of an address as deleted on an already locked
/// connection.
///
/// It takes the connection, so the deletion can be part of a bigger transaction, like the one,
/// which deletes the route towards the address.
///
/// # Arguments
/// * `conn` - The locked database connection
/// * `filter_vni` - Tenant of the address
/// * `filter_ip` - The address
/// * `filter_direction` - Direction of the filter, whose rules are deleted, or `None` for both
/// * `context` - User context to record who performed the deletion
///
/// # Returns
/// * `QueryResult<usize>` with the number of deleted rules
pub fn delete_filter_rules_in(
    conn: &mut diesel::sqlite::SqliteConnection,
    filter_vni: u32,
    filter_ip: Ipv4Addr,
    filter_direction: Option<FilterDirection>,
    context: &UserContext,
) -> QueryResult<usize> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(ainari_common::enums::permission_denied_error());
    }

    use self::network_filters::dsl::*;
    let mut query = network_filters
        .filter(
            vni.eq(filter_vni as i32)
                .and(ip.eq(filter_ip.to_string()))
                .and(status.eq("ACTIVE")),
        )
        .into_boxed();
    if let Some(filter_direction) = filter_direction {
        query = query.filter(direction.eq(filter_direction.to_string()));
    }
    let rule_uuids: Vec<String> = query.select(uuid).load(conn)?;

    diesel::update(network_filters.filter(uuid.eq_any(rule_uuids)))
        .set((
            status.eq("DELETED"),
            deleted_at.eq(Utc::now().to_rfc3339()),
            deleted_by.eq(context.user_id.clone()),
        ))
        .execute(conn)
}

/// Moves the rules of the packet filters of an address to another address on an already locked
/// connection.
///
/// This follows a route, which changed its destination or its tenant, because the filters move
/// with it. Active rules, which the new address had before, are replaced.
///
/// # Arguments
/// * `conn` - The locked database connection
/// * `from` - Tenant and address the rules belong to
/// * `to` - Tenant and address the rules belong to from now on
/// * `context` - User context to record who performed the update
///
/// # Returns
/// * `QueryResult<usize>` with the number of moved rules
pub fn move_filter_rules_in(
    conn: &mut diesel::sqlite::SqliteConnection,
    from: (u32, Ipv4Addr),
    to: (u32, Ipv4Addr),
    context: &UserContext,
) -> QueryResult<usize> {
    if from == to {
        return Ok(0);
    }

    delete_filter_rules_in(conn, to.0, to.1, None, context)?;

    use self::network_filters::dsl::*;
    diesel::update(
        network_filters.filter(
            vni.eq(from.0 as i32)
                .and(ip.eq(from.1.to_string()))
                .and(status.eq("ACTIVE")),
        ),
    )
    .set((
        vni.eq(to.0 as i32),
        ip.eq(to.1.to_string()),
        updated_at.eq(Utc::now().to_rfc3339()),
        updated_by.eq(context.user_id.clone()),
    ))
    .execute(conn)
}

/// Lists the persisted packet filters.
///
/// The textual rules are parsed back into their include-lists. A rule, which can not be parsed
/// anymore, is skipped with an error-log instead of dropping the whole filter.
///
/// # Returns
/// * `QueryResult<HashMap<FilterKey, RouteFilterRules>>` with every filter, which has a rule
pub fn list_filter_rules() -> QueryResult<HashMap<FilterKey, RouteFilterRules>> {
    let entries = {
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        use self::network_filters::dsl::*;
        network_filters
            .filter(status.eq("ACTIVE"))
            .select(NetworkFilterEntry::as_select())
            .load(&mut *conn)?
    };

    let mut filters: HashMap<FilterKey, RouteFilterRules> = HashMap::new();
    for entry in entries {
        let filter_direction = match entry.direction.parse() {
            Ok(filter_direction) => filter_direction,
            Err(e) => {
                log::error!("Skip filter-rule '{}' of {}: {e}", entry.spec, entry.ip);
                continue;
            }
        };
        let key = FilterKey {
            vni: entry.vni,
            ip: entry.ip,
            direction: filter_direction,
        };
        let rules = filters.entry(key).or_default();
        let parsed = match entry.rule_type.as_str() {
            RULE_TYPE_IP_RANGE => entry.spec.parse().map(|rule| rules.ip_ranges.push(rule)),
            RULE_TYPE_PORT => entry.spec.parse().map(|rule| rules.ports.push(rule)),
            other => Err(format!("unknown rule-type '{other}'")),
        };
        if let Err(e) = parsed {
            log::error!(
                "Skip filter-rule '{}' of {} in tenant {}: {e}",
                entry.spec,
                entry.ip,
                entry.vni
            );
        }
    }

    Ok(filters)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ainari_common::enums::ProjectRole;
    use serial_test::serial;

    /// Every test uses its own tenant, so the tests don't see the rules of each other.
    fn test_key(filter_vni: u32, filter_direction: FilterDirection) -> FilterKey {
        FilterKey {
            vni: filter_vni,
            ip: Ipv4Addr::new(10, 0, 0, 5),
            direction: filter_direction,
        }
    }

    fn hard_delete_filters(filter_vni: u32) {
        use self::network_filters::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ =
            diesel::delete(network_filters.filter(vni.eq(filter_vni as i32))).execute(&mut *conn);
    }

    fn test_context() -> UserContext {
        UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: "test-project".to_string(),
            is_admin: true.to_string(),
            project_role: ProjectRole::Admin.to_string(),
        }
    }

    #[test]
    #[serial]
    fn test_set_and_list_filter_rules() {
        let ingress = test_key(9001, FilterDirection::Ingress);
        let egress = test_key(9001, FilterDirection::Egress);
        hard_delete_filters(ingress.vni);

        let rules = RouteFilterRules {
            ip_ranges: vec![
                "10.0.0.0/24".parse().unwrap(),
                "10.0.1.5-10.0.1.9".parse().unwrap(),
            ],
            ports: vec!["22".parse().unwrap(), "8000-8100".parse().unwrap()],
        };
        set_filter_rules(&ingress, &rules, &test_context()).unwrap();

        let filters = list_filter_rules().unwrap();
        let restored = filters.get(&ingress).unwrap();
        let ip_specs: Vec<&str> = restored.ip_ranges.iter().map(|r| r.spec.as_str()).collect();
        let port_specs: Vec<&str> = restored.ports.iter().map(|r| r.spec.as_str()).collect();
        assert_eq!(ip_specs, vec!["10.0.0.0/24", "10.0.1.5-10.0.1.9"]);
        assert_eq!(port_specs, vec!["22", "8000-8100"]);
        // the other direction of the same address is not affected
        assert!(!filters.contains_key(&egress));

        // a new set replaces the old one completely
        let rules = RouteFilterRules {
            ip_ranges: Vec::new(),
            ports: vec!["443".parse().unwrap()],
        };
        set_filter_rules(&ingress, &rules, &test_context()).unwrap();
        set_filter_rules(&egress, &rules, &test_context()).unwrap();
        let filters = list_filter_rules().unwrap();
        let restored = filters.get(&ingress).unwrap();
        assert!(restored.ip_ranges.is_empty());
        assert_eq!(restored.ports.len(), 1);
        assert_eq!(filters.get(&egress).unwrap().ports.len(), 1);

        hard_delete_filters(ingress.vni);
    }

    #[test]
    #[serial]
    fn test_delete_filter_rules() {
        let ingress = test_key(9002, FilterDirection::Ingress);
        let egress = test_key(9002, FilterDirection::Egress);
        hard_delete_filters(ingress.vni);

        let rules = RouteFilterRules {
            ip_ranges: vec!["10.0.0.7".parse().unwrap()],
            ports: Vec::new(),
        };
        set_filter_rules(&ingress, &rules, &test_context()).unwrap();
        set_filter_rules(&egress, &rules, &test_context()).unwrap();

        // only the given direction is deleted ...
        assert_eq!(
            delete_filter_rules_in(
                &mut db_handle::DB_CONN.lock().expect("mutex poisoned"),
                ingress.vni,
                ingress.ip,
                Some(FilterDirection::Egress),
                &test_context()
            )
            .unwrap(),
            1
        );
        let filters = list_filter_rules().unwrap();
        assert!(filters.contains_key(&ingress));
        assert!(!filters.contains_key(&egress));

        // ... or both of them
        set_filter_rules(&egress, &rules, &test_context()).unwrap();
        assert_eq!(
            delete_filter_rules_in(
                &mut db_handle::DB_CONN.lock().expect("mutex poisoned"),
                ingress.vni,
                ingress.ip,
                None,
                &test_context()
            )
            .unwrap(),
            2
        );
        let filters = list_filter_rules().unwrap();
        assert!(!filters.contains_key(&ingress));
        assert!(!filters.contains_key(&egress));

        // an empty filter leaves no active rule behind
        set_filter_rules(&ingress, &rules, &test_context()).unwrap();
        set_filter_rules(&ingress, &RouteFilterRules::default(), &test_context()).unwrap();
        assert!(!list_filter_rules().unwrap().contains_key(&ingress));

        hard_delete_filters(ingress.vni);
    }

    #[test]
    #[serial]
    fn test_move_filter_rules() {
        let from = test_key(9003, FilterDirection::Ingress);
        let to = FilterKey {
            vni: 9004,
            ip: Ipv4Addr::new(10, 0, 0, 6),
            direction: FilterDirection::Ingress,
        };
        hard_delete_filters(from.vni);
        hard_delete_filters(to.vni);

        let rules = RouteFilterRules {
            ip_ranges: vec!["10.0.0.7".parse().unwrap()],
            ports: vec!["22".parse().unwrap()],
        };
        let old_rules = RouteFilterRules {
            ip_ranges: Vec::new(),
            ports: vec!["443".parse().unwrap()],
        };
        set_filter_rules(&from, &rules, &test_context()).unwrap();
        set_filter_rules(&to, &old_rules, &test_context()).unwrap();

        assert_eq!(
            move_filter_rules_in(
                &mut db_handle::DB_CONN.lock().expect("mutex poisoned"),
                (from.vni, from.ip),
                (to.vni, to.ip),
                &test_context()
            )
            .unwrap(),
            2
        );

        // the moved rules replace the ones, which the new address had before
        let filters = list_filter_rules().unwrap();
        assert!(!filters.contains_key(&from));
        let moved = filters.get(&to).unwrap();
        assert_eq!(moved.ip_ranges.len(), 1);
        assert_eq!(moved.ports.len(), 1);
        assert_eq!(moved.ports[0].spec, "22");

        hard_delete_filters(from.vni);
        hard_delete_filters(to.vni);
    }
}
