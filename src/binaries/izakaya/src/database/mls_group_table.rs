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

//! State of the coordination of every group, which izakaya keeps.

use diesel::prelude::*;

use crate::database::db_handle;

use ainari_common::objects::*;

// Define the schema for mls_groups table
table! {
    mls_groups (vni) {
        vni -> Integer,
        committer -> Varchar,
        epoch -> BigInt,
        members -> Text,
        round_epoch -> Nullable<BigInt>,
        round_phase -> Nullable<Varchar>,
        round_acks -> Nullable<Text>,
        round_updated_at -> Nullable<BigInt>,
        rotated_at -> BigInt,
        created_at -> BigInt,
    }
}

/// Coordination-state of the group of a network. The members and the acknowledgements of the
/// round are JSON-encoded lists of identities, the timestamps are unix-times in seconds.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = mls_groups)]
pub struct MlsGroupEntry {
    #[diesel(serialize_as = DbVni, deserialize_as = DbVni)]
    pub vni: u32,
    pub committer: String,
    pub epoch: i64,
    pub members: String,
    pub round_epoch: Option<i64>,
    pub round_phase: Option<String>,
    pub round_acks: Option<String>,
    pub round_updated_at: Option<i64>,
    pub rotated_at: i64,
    pub created_at: i64,
}

/// Returns the coordination-state of the group of a network.
///
/// # Arguments
/// * `group_vni` - Tenant of the network
///
/// # Returns
/// The state, or `None` if the network has no group
pub fn get_group(group_vni: u32) -> QueryResult<Option<MlsGroupEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::mls_groups::dsl::*;
    mls_groups
        .filter(vni.eq(group_vni as i32))
        .select(MlsGroupEntry::as_select())
        .first(&mut *conn)
        .optional()
}

/// Lists the tenants of all networks, which have a group.
///
/// # Returns
/// The tenants
pub fn list_group_vnis() -> QueryResult<Vec<u32>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::mls_groups::dsl::*;
    let vnis: Vec<i32> = mls_groups.select(vni).load(&mut *conn)?;
    Ok(vnis.into_iter().map(|it| it as u32).collect())
}

/// Stores the coordination-state of a group, which replaces its previous state.
///
/// # Arguments
/// * `entry` - The new state
///
/// # Returns
/// `Ok(())` once the state is stored
pub fn save_group(entry: MlsGroupEntry) -> QueryResult<()> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.transaction(|conn| {
        use self::mls_groups::dsl::*;
        diesel::delete(mls_groups.filter(vni.eq(entry.vni as i32))).execute(conn)?;
        diesel::insert_into(mls_groups)
            .values(entry)
            .execute(conn)?;
        Ok(())
    })
}

/// Removes the coordination-state of the group of a network.
///
/// # Arguments
/// * `group_vni` - Tenant of the network
///
/// # Returns
/// The number of removed groups
pub fn delete_group(group_vni: u32) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::mls_groups::dsl::*;
    diesel::delete(mls_groups.filter(vni.eq(group_vni as i32))).execute(&mut *conn)
}
