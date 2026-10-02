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

use ainari_api_structs::floating_ip_structs::FloatingIpInternalCreateReq;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;
use ainari_common::objects::*;

// Define the schema for the floating_ips table
table! {
    floating_ips (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        network_uuid -> Varchar,
        floating_ip -> Varchar,
        internal_ip -> Varchar,
        vni -> Integer,
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

/// Represents a single entry in the floating_ips table.
/// Contains all fields necessary to re-program the NAT of a floating IP after a restart.
///
/// # Fields
/// * `uuid` - Unique identifier of the floating IP
/// * `name` - Name of the floating IP
/// * `network_uuid` - UUID of the network of the internal address
/// * `floating_ip` - The public address
/// * `internal_ip` - The address of the VM behind the floating IP
/// * `vni` - Tenant of the internal address
/// * `owner_id` - User ID of the floating IP owner
/// * `project_id` - Project ID the floating IP belongs to
/// * `status` - Current status of the floating IP (ACTIVE, DELETED, etc.)
/// * `created_at` - Timestamp when the floating IP was created
/// * `created_by` - User ID who created the floating IP
/// * `updated_at` - Timestamp when the floating IP was last updated
/// * `updated_by` - User ID who last updated the floating IP
/// * `deleted_at` - Timestamp when the floating IP was deleted (nullable)
/// * `deleted_by` - User ID who deleted the floating IP (nullable)
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = floating_ips)]
pub struct FloatingIpEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    pub name: String,
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub network_uuid: Uuid,
    #[diesel(serialize_as = DbIpv4Addr, deserialize_as = DbIpv4Addr)]
    pub floating_ip: Ipv4Addr,
    #[diesel(serialize_as = DbIpv4Addr, deserialize_as = DbIpv4Addr)]
    pub internal_ip: Ipv4Addr,
    #[diesel(serialize_as = DbVni, deserialize_as = DbVni)]
    pub vni: u32,
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

/// Persists a registered floating IP.
///
/// A floating IP is unique, so an already persisted entry for the same address is marked as
/// deleted and replaced within one transaction. This keeps the database in line with the
/// datapath, where a repeated registration of the same address simply overwrites the old one.
///
/// # Arguments
/// * `fip_uuid` - Unique identifier of the floating IP
/// * `req` - The request, which registered the floating IP
/// * `context` - User context containing ownership and project information
///
/// # Returns
/// * `QueryResult<usize>` indicating the number of inserted rows
pub fn set_floating_ip(
    fip_uuid: &Uuid,
    req: &FloatingIpInternalCreateReq,
    context: &UserContext,
) -> QueryResult<usize> {
    let entry = FloatingIpEntry {
        uuid: *fip_uuid,
        name: req.name.clone(),
        network_uuid: req.network_uuid,
        floating_ip: req.floating_ip,
        internal_ip: req.internal_ip,
        vni: req.vni,
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
        delete_floating_ip_in(conn, &req.floating_ip, context)?;
        use self::floating_ips::dsl::*;
        diesel::insert_into(floating_ips)
            .values(entry)
            .execute(conn)
    })
}

/// Lists all active floating IP entries, ordered by their creation.
///
/// # Returns
/// * `QueryResult<Vec<FloatingIpEntry>>` containing all active floating IPs
pub fn list_floating_ips() -> QueryResult<Vec<FloatingIpEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::floating_ips::dsl::*;

    let mut entries = floating_ips
        .filter(status.eq("ACTIVE"))
        .select(FloatingIpEntry::as_select())
        .load(&mut *conn)?;
    entries.sort_by_key(|entry| entry.created_at);
    Ok(entries)
}

/// Marks a floating IP as deleted in the database.
///
/// # Arguments
/// * `ip` - The public address of the floating IP
/// * `context` - User context to record who performed the deletion
///
/// # Returns
/// * `Result<(), enums::DbError>` indicating success or failure
pub fn delete_floating_ip(ip: &Ipv4Addr, context: &UserContext) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    match delete_floating_ip_in(&mut conn, ip, context) {
        Ok(0) => Err(enums::DbError::NotFound),
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Marks the active entry of a floating IP as deleted on an already locked connection.
///
/// # Arguments
/// * `conn` - The locked database connection
/// * `ip` - The public address of the floating IP
/// * `context` - User context to record who performed the deletion
///
/// # Returns
/// * `QueryResult<usize>` with the number of deleted entries
fn delete_floating_ip_in(
    conn: &mut diesel::sqlite::SqliteConnection,
    ip: &Ipv4Addr,
    context: &UserContext,
) -> QueryResult<usize> {
    use self::floating_ips::dsl::*;
    diesel::update(floating_ips.filter(floating_ip.eq(ip.to_string()).and(status.eq("ACTIVE"))))
        .set((
            status.eq("DELETED"),
            deleted_at.eq(Utc::now().to_rfc3339()),
            deleted_by.eq(context.user_id.clone()),
        ))
        .execute(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ainari_common::enums::ProjectRole;
    use serial_test::serial;

    fn hard_delete_floating_ip(ip: &Ipv4Addr) {
        use self::floating_ips::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ =
            diesel::delete(floating_ips.filter(floating_ip.eq(ip.to_string()))).execute(&mut *conn);
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

    fn test_req(internal_ip: Ipv4Addr) -> FloatingIpInternalCreateReq {
        FloatingIpInternalCreateReq {
            name: "test-fip".to_string(),
            network_uuid: Uuid::new_v4(),
            floating_ip: Ipv4Addr::new(10, 99, 0, 42),
            internal_ip,
            vni: 7,
        }
    }

    fn active_entries(ip: &Ipv4Addr) -> Vec<FloatingIpEntry> {
        list_floating_ips()
            .unwrap()
            .into_iter()
            .filter(|entry| entry.floating_ip == *ip)
            .collect()
    }

    #[test]
    #[serial]
    fn test_set_and_list_floating_ip() {
        let req = test_req(Ipv4Addr::new(192, 168, 100, 5));
        hard_delete_floating_ip(&req.floating_ip);

        let fip_uuid = Uuid::new_v4();
        set_floating_ip(&fip_uuid, &req, &test_context()).unwrap();
        let entries = active_entries(&req.floating_ip);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].uuid, fip_uuid);
        assert_eq!(entries[0].internal_ip, req.internal_ip);
        assert_eq!(entries[0].network_uuid, req.network_uuid);
        assert_eq!(entries[0].vni, 7);

        // registering the same address again replaces the old entry
        let req2 = test_req(Ipv4Addr::new(192, 168, 100, 6));
        set_floating_ip(&Uuid::new_v4(), &req2, &test_context()).unwrap();
        let entries = active_entries(&req.floating_ip);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].internal_ip, req2.internal_ip);

        hard_delete_floating_ip(&req.floating_ip);
    }

    #[test]
    #[serial]
    fn test_delete_floating_ip() {
        let req = test_req(Ipv4Addr::new(192, 168, 100, 5));
        hard_delete_floating_ip(&req.floating_ip);

        set_floating_ip(&Uuid::new_v4(), &req, &test_context()).unwrap();
        assert!(delete_floating_ip(&req.floating_ip, &test_context()).is_ok());
        assert!(active_entries(&req.floating_ip).is_empty());
        assert!(delete_floating_ip(&req.floating_ip, &test_context()).is_err());

        hard_delete_floating_ip(&req.floating_ip);
    }
}
