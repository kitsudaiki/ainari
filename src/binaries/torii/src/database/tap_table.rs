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

use crate::database::db_handle;

use ainari_api_structs::network_interface_structs::TapReq;
use ainari_api_structs::user_context::UserContext;
use ainari_common::objects::*;

// Define the schema for the taps table
table! {
    taps (uuid) {
        uuid -> Varchar,
        tap_name -> Varchar,
        vni -> Integer,
        vm_mac -> Nullable<Varchar>,
        vm_ip -> Nullable<Varchar>,
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

/// Represents a single entry in the taps table.
/// Contains all fields necessary to re-create a TAP device and its eBPF state after a restart.
///
/// # Fields
/// * `uuid` - Unique identifier of the entry
/// * `tap_name` - Name of the TAP device
/// * `vni` - Tenant the TAP device belongs to
/// * `vm_mac` - MAC of the VM behind the TAP device (nullable)
/// * `vm_ip` - Address of the VM behind the TAP device (nullable)
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
#[diesel(table_name = taps)]
pub struct TapEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    pub tap_name: String,
    #[diesel(serialize_as = DbVni, deserialize_as = DbVni)]
    pub vni: u32,
    pub vm_mac: Option<String>,
    #[diesel(serialize_as = DbOptIpv4Addr, deserialize_as = DbOptIpv4Addr)]
    pub vm_ip: Option<Ipv4Addr>,
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

impl From<TapEntry> for TapReq {
    fn from(entry: TapEntry) -> Self {
        TapReq {
            tap_name: entry.tap_name,
            vni: entry.vni,
            vm_mac: entry.vm_mac,
            vm_ip: entry.vm_ip,
        }
    }
}

/// Persists a registered TAP device.
///
/// A TAP device is registered only once at a time, so an already persisted entry for the same
/// device is marked as deleted and replaced within one transaction.
///
/// # Arguments
/// * `req` - The request, which registered the TAP device
/// * `context` - User context containing ownership and project information
///
/// # Returns
/// * `QueryResult<usize>` indicating the number of inserted rows
pub fn set_tap(req: &TapReq, context: &UserContext) -> QueryResult<usize> {
    let entry = TapEntry {
        uuid: Uuid::new_v4(),
        tap_name: req.tap_name.clone(),
        vni: req.vni,
        vm_mac: req.vm_mac.clone(),
        vm_ip: req.vm_ip,
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
        use self::taps::dsl::*;
        diesel::update(taps.filter(tap_name.eq(req.tap_name.clone()).and(status.eq("ACTIVE"))))
            .set((
                status.eq("DELETED"),
                deleted_at.eq(Utc::now().to_rfc3339()),
                deleted_by.eq(context.user_id.clone()),
            ))
            .execute(conn)?;
        diesel::insert_into(taps).values(entry).execute(conn)
    })
}

/// Retrieves the active registration of a TAP device.
///
/// # Arguments
/// * `name` - Name of the TAP device
///
/// # Returns
/// * `QueryResult<Option<TapEntry>>` with the registration, or `None` if the device has none
pub fn get_tap(name: &str) -> QueryResult<Option<TapEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::taps::dsl::*;

    taps.filter(tap_name.eq(name).and(status.eq("ACTIVE")))
        .select(TapEntry::as_select())
        .first(&mut *conn)
        .optional()
}

/// Lists all active TAP devices, ordered by their creation.
///
/// # Returns
/// * `QueryResult<Vec<TapEntry>>` containing all active TAP devices
pub fn list_taps() -> QueryResult<Vec<TapEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::taps::dsl::*;

    let mut entries = taps
        .filter(status.eq("ACTIVE"))
        .select(TapEntry::as_select())
        .load(&mut *conn)?;
    entries.sort_by_key(|entry| entry.created_at);
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ainari_common::enums::ProjectRole;
    use serial_test::serial;

    fn hard_delete_tap(name: &str) {
        use self::taps::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(taps.filter(tap_name.eq(name))).execute(&mut *conn);
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

    fn active_entries(name: &str) -> Vec<TapEntry> {
        list_taps()
            .unwrap()
            .into_iter()
            .filter(|entry| entry.tap_name == name)
            .collect()
    }

    #[test]
    #[serial]
    fn test_set_and_list_tap() {
        let name = "tap-test-db";
        hard_delete_tap(name);

        let req = TapReq {
            tap_name: name.to_string(),
            vni: 5,
            vm_mac: Some("52:54:00:12:34:56".to_string()),
            vm_ip: Some(Ipv4Addr::new(192, 168, 100, 5)),
        };
        assert!(get_tap(name).unwrap().is_none());
        set_tap(&req, &test_context()).unwrap();
        let entries = active_entries(name);
        assert_eq!(entries.len(), 1);
        assert_eq!(get_tap(name).unwrap(), Some(entries[0].clone()));
        let restored: TapReq = entries[0].clone().into();
        assert_eq!(restored.vni, req.vni);
        assert_eq!(restored.vm_mac, req.vm_mac);
        assert_eq!(restored.vm_ip, req.vm_ip);

        // registering the same TAP again replaces the old entry
        let req2 = TapReq {
            vni: 6,
            vm_mac: None,
            vm_ip: None,
            ..req
        };
        set_tap(&req2, &test_context()).unwrap();
        let entries = active_entries(name);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].vni, 6);
        assert_eq!(entries[0].vm_ip, None);

        hard_delete_tap(name);
    }
}
