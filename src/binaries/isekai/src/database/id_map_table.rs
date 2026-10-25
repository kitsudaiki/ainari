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

use diesel::prelude::*;
use uuid::Uuid;

use crate::database::db_handle;

use ainari_common::objects::*;

table! {
    id_maps (job_uuid, old_uuid) {
        job_uuid -> Varchar,
        resource_type -> Varchar,
        old_uuid -> Varchar,
        new_uuid -> Varchar,
    }
}

/// Represents an entry in the id_maps table: a resource, which was restored by a job under
/// another UUID, because its original UUID was already used.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = id_maps)]
pub struct IdMapEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub job_uuid: Uuid,
    /// kind of the resource, like `virtual_machine` or `network`
    pub resource_type: String,
    /// UUID of the resource within the backup
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub old_uuid: Uuid,
    /// UUID of the restored resource
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub new_uuid: Uuid,
}

/// Stores the new UUID of a restored resource. An existing mapping of the same resource within
/// the job is replaced, so a repeated step doesn't fail.
///
/// # Arguments
///
/// * `entry` - The mapping
///
/// # Returns
///
/// A QueryResult indicating the number of rows affected
pub fn put_id_map_entry(entry: IdMapEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::id_maps::dsl::*;

    conn.transaction(|conn| {
        diesel::delete(
            id_maps
                .filter(job_uuid.eq(entry.job_uuid.to_string()))
                .filter(old_uuid.eq(entry.old_uuid.to_string())),
        )
        .execute(conn)?;
        diesel::insert_into(id_maps).values(entry).execute(conn)
    })
}

/// Lists the new UUIDs of the resources, which were restored by a job.
///
/// # Arguments
///
/// * `filter_job_uuid` - The job
///
/// # Returns
///
/// A QueryResult containing the mappings of the job
pub fn list_id_map_of_job(filter_job_uuid: &Uuid) -> QueryResult<Vec<IdMapEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::id_maps::dsl::*;

    id_maps
        .filter(job_uuid.eq(filter_job_uuid.to_string()))
        .order(resource_type.asc())
        .select(IdMapEntry::as_select())
        .load(&mut *conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn hard_delete_id_map(filter_job_uuid: &Uuid) {
        use self::id_maps::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(id_maps.filter(job_uuid.eq(filter_job_uuid.to_string())))
            .execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_put_and_list_id_map() {
        let job = Uuid::new_v4();
        let (old_vm, new_vm, newer_vm) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let (old_network, new_network) = (Uuid::new_v4(), Uuid::new_v4());
        hard_delete_id_map(&job);

        let entry = |resource: &str, old: Uuid, new: Uuid| IdMapEntry {
            job_uuid: job,
            resource_type: resource.to_string(),
            old_uuid: old,
            new_uuid: new,
        };
        put_id_map_entry(entry("virtual_machine", old_vm, new_vm)).unwrap();
        put_id_map_entry(entry("network", old_network, new_network)).unwrap();
        // a repeated step replaces the mapping of the same resource
        put_id_map_entry(entry("virtual_machine", old_vm, newer_vm)).unwrap();

        let listed = list_id_map_of_job(&job).unwrap();
        assert_eq!(
            listed,
            vec![
                entry("network", old_network, new_network),
                entry("virtual_machine", old_vm, newer_vm),
            ]
        );
        assert!(list_id_map_of_job(&Uuid::new_v4()).unwrap().is_empty());

        hard_delete_id_map(&job);
    }
}
