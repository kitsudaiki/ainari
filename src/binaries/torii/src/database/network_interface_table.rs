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
use uuid::Uuid;

use crate::database::db_handle;

use ainari_api_structs::network_interface_structs::IfaceConfigReq;
use ainari_api_structs::user_context::UserContext;
use ainari_common::objects::*;

// Define the schema for the network_interfaces table
table! {
    network_interfaces (uuid) {
        uuid -> Varchar,
        iface_name -> Varchar,
        ip_cidr -> Nullable<Varchar>,
        up -> Bool,
        vni -> Integer,
        fip_port -> Bool,
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

/// Represents a single entry in the network_interfaces table.
/// Contains the configuration of an existing interface, which places it into a tenant.
///
/// # Fields
/// * `uuid` - Unique identifier of the entry
/// * `iface_name` - Name of the configured interface
/// * `ip_cidr` - Address, which was added to the interface (nullable)
/// * `up` - True, if the interface was brought up
/// * `vni` - Tenant the interface belongs to
/// * `fip_port` - True, if floating IPs are translated on the interface
/// * `owner_id` - User ID of the entry owner
/// * `project_id` - Project ID the entry belongs to
/// * `status` - Current status of the entry (ACTIVE, DELETED, etc.)
/// * `created_at` - Timestamp when the entry was created
/// * `created_by` - User ID who created the entry
/// * `updated_at` - Timestamp when the entry was last updated
/// * `updated_by` - User ID who last updated the entry
/// * `deleted_at` - Timestamp when the entry was deleted (nullable)
/// * `deleted_by` - User ID who deleted the entry (nullable)
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = network_interfaces)]
pub struct NetworkInterfaceEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    pub iface_name: String,
    pub ip_cidr: Option<String>,
    pub up: bool,
    #[diesel(serialize_as = DbVni, deserialize_as = DbVni)]
    pub vni: u32,
    pub fip_port: bool,
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

impl From<NetworkInterfaceEntry> for IfaceConfigReq {
    fn from(entry: NetworkInterfaceEntry) -> Self {
        IfaceConfigReq {
            iface_name: entry.iface_name,
            ip_cidr: entry.ip_cidr,
            up: entry.up,
            vni: entry.vni,
            fip_port: entry.fip_port,
        }
    }
}

/// Persists the configuration of an interface.
///
/// An interface has only one configuration at a time, so an already persisted entry for the same
/// interface is marked as deleted and replaced within one transaction.
///
/// # Arguments
/// * `req` - The request, which configured the interface
/// * `context` - User context containing ownership and project information
///
/// # Returns
/// * `QueryResult<usize>` indicating the number of inserted rows
pub fn set_network_interface(req: &IfaceConfigReq, context: &UserContext) -> QueryResult<usize> {
    let entry = NetworkInterfaceEntry {
        uuid: Uuid::new_v4(),
        iface_name: req.iface_name.clone(),
        ip_cidr: req.ip_cidr.clone(),
        up: req.up,
        vni: req.vni,
        fip_port: req.fip_port,
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

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.transaction(|conn| {
        use self::network_interfaces::dsl::*;
        diesel::update(
            network_interfaces.filter(
                iface_name
                    .eq(req.iface_name.clone())
                    .and(status.eq("ACTIVE")),
            ),
        )
        .set((
            status.eq("DELETED"),
            deleted_at.eq(Utc::now().to_rfc3339()),
            deleted_by.eq(context.user_id.clone()),
        ))
        .execute(conn)?;
        diesel::insert_into(network_interfaces)
            .values(entry)
            .execute(conn)
    })
}

/// Lists all active interface configurations, ordered by their creation.
///
/// # Returns
/// * `QueryResult<Vec<NetworkInterfaceEntry>>` containing all active interface configurations
pub fn list_network_interfaces() -> QueryResult<Vec<NetworkInterfaceEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::network_interfaces::dsl::*;

    let mut entries = network_interfaces
        .filter(status.eq("ACTIVE"))
        .select(NetworkInterfaceEntry::as_select())
        .load(&mut *conn)?;
    entries.sort_by_key(|entry| entry.created_at);
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn hard_delete_interface(name: &str) {
        use self::network_interfaces::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(network_interfaces.filter(iface_name.eq(name))).execute(&mut *conn);
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

    fn active_entries(name: &str) -> Vec<NetworkInterfaceEntry> {
        list_network_interfaces()
            .unwrap()
            .into_iter()
            .filter(|entry| entry.iface_name == name)
            .collect()
    }

    #[test]
    #[serial]
    fn test_set_and_list_network_interface() {
        let name = "test-iface-db";
        hard_delete_interface(name);

        let req = IfaceConfigReq {
            iface_name: name.to_string(),
            ip_cidr: Some("10.0.0.254/24".to_string()),
            up: true,
            vni: 3,
            fip_port: true,
        };
        set_network_interface(&req, &test_context()).unwrap();
        let entries = active_entries(name);
        assert_eq!(entries.len(), 1);
        let restored: IfaceConfigReq = entries[0].clone().into();
        assert_eq!(restored.ip_cidr, req.ip_cidr);
        assert!(restored.up);
        assert_eq!(restored.vni, 3);
        assert!(restored.fip_port);

        // configuring the same interface again replaces the old entry
        let req2 = IfaceConfigReq {
            ip_cidr: None,
            fip_port: false,
            ..req
        };
        set_network_interface(&req2, &test_context()).unwrap();
        let entries = active_entries(name);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].ip_cidr, None);
        assert!(!entries[0].fip_port);

        hard_delete_interface(name);
    }
}
