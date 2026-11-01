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

//! Marks of the virtual_machines, which are migrated right now.
//!
//! The marks are stored in the database, so all replicas of hanami see the migrations of the
//! others. The policy, when a mark is stale, is up to the caller (see `core::migration::job`).

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::result::DatabaseErrorKind;
use uuid::Uuid;

use crate::database::db_handle;

use ainari_common::objects::*;

table! {
    virtual_machine_migrations (virtual_machine_uuid) {
        virtual_machine_uuid -> Varchar,
        lock_uuid -> Varchar,
        owner -> Varchar,
        started_at -> Varchar,
        heartbeat_at -> Varchar,
    }
}

/// Mark of a virtual_machine, which is migrated right now.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = virtual_machine_migrations)]
pub struct VmMigrationEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub virtual_machine_uuid: Uuid,
    /// Identifies a single mark, so a replica only removes or refreshes its own one
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub lock_uuid: Uuid,
    /// Replica, which runs the migration
    pub owner: String,
    #[diesel(serialize_as = DbDateTime, deserialize_as = DbDateTime)]
    pub started_at: DateTime<Utc>,
    /// Last sign of life of the migration
    #[diesel(serialize_as = DbDateTime, deserialize_as = DbDateTime)]
    pub heartbeat_at: DateTime<Utc>,
}

/// Adds the mark of a virtual_machine.
///
/// # Arguments
/// * `entry` - The mark
///
/// # Returns
/// A QueryResult with true, if the mark was added, or false, if the virtual_machine is already
/// marked
pub fn add_vm_migration(entry: VmMigrationEntry) -> QueryResult<bool> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::virtual_machine_migrations::dsl::*;
    match diesel::insert_into(virtual_machine_migrations)
        .values(entry)
        .execute(&mut *conn)
    {
        Ok(_) => Ok(true),
        Err(diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
            Ok(false)
        }
        Err(e) => Err(e),
    }
}

/// Retrieves the mark of a virtual_machine.
///
/// # Arguments
/// * `vm_uuid` - The UUID of the virtual_machine
///
/// # Returns
/// A QueryResult with the mark, or None if the virtual_machine is not marked
pub fn get_vm_migration(vm_uuid: &Uuid) -> QueryResult<Option<VmMigrationEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::virtual_machine_migrations::dsl::*;
    virtual_machine_migrations
        .filter(virtual_machine_uuid.eq(vm_uuid.to_string()))
        .select(VmMigrationEntry::as_select())
        .first::<VmMigrationEntry>(&mut *conn)
        .optional()
}

/// Refreshes the sign of life of a mark.
///
/// # Arguments
/// * `vm_uuid` - The UUID of the virtual_machine
/// * `lock` - The mark, which has to be refreshed
/// * `now` - The new sign of life
///
/// # Returns
/// A QueryResult with true, if the mark was refreshed, or false, if it doesn't exist anymore
pub fn refresh_vm_migration(vm_uuid: &Uuid, lock: &Uuid, now: DateTime<Utc>) -> QueryResult<bool> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::virtual_machine_migrations::dsl::*;
    diesel::update(
        virtual_machine_migrations.filter(
            virtual_machine_uuid
                .eq(vm_uuid.to_string())
                .and(lock_uuid.eq(lock.to_string())),
        ),
    )
    .set(heartbeat_at.eq(now.to_rfc3339()))
    .execute(&mut *conn)
    .map(|rows| rows > 0)
}

/// Removes a mark of a virtual_machine. A newer mark of the same virtual_machine is kept.
///
/// # Arguments
/// * `vm_uuid` - The UUID of the virtual_machine
/// * `lock` - The mark, which has to be removed
///
/// # Returns
/// A QueryResult with true, if the mark was removed, or false, if it doesn't exist anymore
pub fn remove_vm_migration(vm_uuid: &Uuid, lock: &Uuid) -> QueryResult<bool> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::virtual_machine_migrations::dsl::*;
    diesel::delete(
        virtual_machine_migrations.filter(
            virtual_machine_uuid
                .eq(vm_uuid.to_string())
                .and(lock_uuid.eq(lock.to_string())),
        ),
    )
    .execute(&mut *conn)
    .map(|rows| rows > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn new_entry(vm_uuid: Uuid) -> VmMigrationEntry {
        // the database stores the timestamps as RFC-3339, which keeps only full nanoseconds
        let now = DateTime::parse_from_rfc3339(&Utc::now().to_rfc3339())
            .unwrap()
            .with_timezone(&Utc);
        VmMigrationEntry {
            virtual_machine_uuid: vm_uuid,
            lock_uuid: Uuid::new_v4(),
            owner: "test-replica".to_string(),
            started_at: now,
            heartbeat_at: now,
        }
    }

    #[test]
    #[serial]
    fn test_vm_migration_marks() {
        let vm_uuid = Uuid::new_v4();
        assert_eq!(get_vm_migration(&vm_uuid).unwrap(), None);

        let first = new_entry(vm_uuid);
        assert!(add_vm_migration(first.clone()).unwrap());
        assert_eq!(get_vm_migration(&vm_uuid).unwrap(), Some(first.clone()));

        // only one mark per virtual_machine
        let second = new_entry(vm_uuid);
        assert!(!add_vm_migration(second.clone()).unwrap());

        // only the own mark is refreshed and removed
        let later = first.heartbeat_at + chrono::Duration::seconds(5);
        assert!(!refresh_vm_migration(&vm_uuid, &second.lock_uuid, later).unwrap());
        assert!(refresh_vm_migration(&vm_uuid, &first.lock_uuid, later).unwrap());
        assert_eq!(
            get_vm_migration(&vm_uuid).unwrap().map(|e| e.heartbeat_at),
            Some(later)
        );
        assert!(!remove_vm_migration(&vm_uuid, &second.lock_uuid).unwrap());
        assert!(remove_vm_migration(&vm_uuid, &first.lock_uuid).unwrap());
        assert_eq!(get_vm_migration(&vm_uuid).unwrap(), None);

        // the virtual_machine can be marked again afterwards
        assert!(add_vm_migration(second.clone()).unwrap());
        assert!(remove_vm_migration(&vm_uuid, &second.lock_uuid).unwrap());
    }
}
