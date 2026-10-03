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
use uuid::Uuid;

use crate::database::db_handle;

use ainari_api_structs::network_filter_structs::RouteFilterRules;
use ainari_api_structs::user_context::UserContext;
use ainari_common::objects::*;

/// Rule-type of an entry of the IP-range include-list of a route.
const RULE_TYPE_IP_RANGE: &str = "IP_RANGE";
/// Rule-type of an entry of the port include-list of a route.
const RULE_TYPE_PORT: &str = "PORT";

// Define the schema for the network_filters table
table! {
    network_filters (uuid) {
        uuid -> Varchar,
        route_uuid -> Varchar,
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
/// Each entry is one rule of the include-lists of the packet filter of a route.
///
/// # Fields
/// * `uuid` - Unique identifier of the rule
/// * `route_uuid` - UUID of the route the rule belongs to
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
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub route_uuid: Uuid,
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

/// Replaces the persisted packet filter of a route by a new set of include-lists.
///
/// All active rules of the route are marked as deleted and the new rules are inserted within one
/// transaction, so the database never holds a half-written filter. Empty include-lists leave the
/// route without any active rule, which is the unfiltered state.
///
/// # Arguments
/// * `filter_route_uuid` - UUID of the route the filter belongs to
/// * `rules` - The include-lists the route has from now on
/// * `context` - User context containing ownership and project information
///
/// # Returns
/// * `QueryResult<()>` indicating success or failure
pub fn set_filter_rules(
    filter_route_uuid: &Uuid,
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
            route_uuid: *filter_route_uuid,
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
        delete_filter_rules_in(conn, filter_route_uuid, context)?;
        use self::network_filters::dsl::*;
        diesel::insert_into(network_filters)
            .values(entries)
            .execute(conn)?;
        Ok(())
    })
}

/// Marks all rules of the packet filter of a route as deleted on an already locked connection.
///
/// It takes the connection, so the deletion can be part of a bigger transaction, like the one,
/// which deletes the route itself.
///
/// # Arguments
/// * `conn` - The locked database connection
/// * `filter_route_uuid` - UUID of the route the filter belongs to
/// * `context` - User context to record who performed the deletion
///
/// # Returns
/// * `QueryResult<usize>` with the number of deleted rules
pub fn delete_filter_rules_in(
    conn: &mut diesel::sqlite::SqliteConnection,
    filter_route_uuid: &Uuid,
    context: &UserContext,
) -> QueryResult<usize> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(ainari_common::enums::permission_denied_error());
    }

    use self::network_filters::dsl::*;
    diesel::update(
        network_filters.filter(
            route_uuid
                .eq(filter_route_uuid.to_string())
                .and(status.eq("ACTIVE")),
        ),
    )
    .set((
        status.eq("DELETED"),
        deleted_at.eq(Utc::now().to_rfc3339()),
        deleted_by.eq(context.user_id.clone()),
    ))
    .execute(conn)
}

/// Lists the persisted packet filters of all routes.
///
/// The textual rules are parsed back into their include-lists. A rule, which can not be parsed
/// anymore, is skipped with an error-log instead of dropping the whole filter of its route.
///
/// # Returns
/// * `QueryResult<HashMap<Uuid, RouteFilterRules>>` with the filter of every route, which has one
pub fn list_filter_rules() -> QueryResult<HashMap<Uuid, RouteFilterRules>> {
    let entries = {
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        use self::network_filters::dsl::*;
        network_filters
            .filter(status.eq("ACTIVE"))
            .select(NetworkFilterEntry::as_select())
            .load(&mut *conn)?
    };

    let mut filters: HashMap<Uuid, RouteFilterRules> = HashMap::new();
    for entry in entries {
        let rules = filters.entry(entry.route_uuid).or_default();
        let parsed = match entry.rule_type.as_str() {
            RULE_TYPE_IP_RANGE => entry.spec.parse().map(|rule| rules.ip_ranges.push(rule)),
            RULE_TYPE_PORT => entry.spec.parse().map(|rule| rules.ports.push(rule)),
            other => Err(format!("unknown rule-type '{other}'")),
        };
        if let Err(e) = parsed {
            log::error!(
                "Skip filter-rule '{}' of route '{}': {e}",
                entry.spec,
                entry.route_uuid
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

    fn hard_delete_filter(filter_route_uuid: &Uuid) {
        use self::network_filters::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ =
            diesel::delete(network_filters.filter(route_uuid.eq(filter_route_uuid.to_string())))
                .execute(&mut *conn);
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
        let route_uuid1 = Uuid::new_v4();
        hard_delete_filter(&route_uuid1);

        let rules = RouteFilterRules {
            ip_ranges: vec![
                "10.0.0.0/24".parse().unwrap(),
                "10.0.1.5-10.0.1.9".parse().unwrap(),
            ],
            ports: vec!["22".parse().unwrap(), "8000-8100".parse().unwrap()],
        };
        set_filter_rules(&route_uuid1, &rules, &test_context()).unwrap();

        let filters = list_filter_rules().unwrap();
        let restored = filters.get(&route_uuid1).unwrap();
        let ip_specs: Vec<&str> = restored.ip_ranges.iter().map(|r| r.spec.as_str()).collect();
        let port_specs: Vec<&str> = restored.ports.iter().map(|r| r.spec.as_str()).collect();
        assert_eq!(ip_specs, vec!["10.0.0.0/24", "10.0.1.5-10.0.1.9"]);
        assert_eq!(port_specs, vec!["22", "8000-8100"]);

        // a new set replaces the old one completely
        let rules = RouteFilterRules {
            ip_ranges: Vec::new(),
            ports: vec!["443".parse().unwrap()],
        };
        set_filter_rules(&route_uuid1, &rules, &test_context()).unwrap();
        let filters = list_filter_rules().unwrap();
        let restored = filters.get(&route_uuid1).unwrap();
        assert!(restored.ip_ranges.is_empty());
        assert_eq!(restored.ports.len(), 1);

        hard_delete_filter(&route_uuid1);
    }

    #[test]
    #[serial]
    fn test_delete_filter_rules() {
        let route_uuid1 = Uuid::new_v4();
        hard_delete_filter(&route_uuid1);

        let rules = RouteFilterRules {
            ip_ranges: vec!["10.0.0.7".parse().unwrap()],
            ports: Vec::new(),
        };
        set_filter_rules(&route_uuid1, &rules, &test_context()).unwrap();
        assert_eq!(
            delete_filter_rules_in(
                &mut db_handle::DB_CONN.lock().expect("mutex poisoned"),
                &route_uuid1,
                &test_context()
            )
            .unwrap(),
            1
        );
        assert!(!list_filter_rules().unwrap().contains_key(&route_uuid1));

        // an empty filter leaves no active rule behind
        set_filter_rules(&route_uuid1, &rules, &test_context()).unwrap();
        set_filter_rules(&route_uuid1, &RouteFilterRules::default(), &test_context()).unwrap();
        assert!(!list_filter_rules().unwrap().contains_key(&route_uuid1));

        hard_delete_filter(&route_uuid1);
    }
}
