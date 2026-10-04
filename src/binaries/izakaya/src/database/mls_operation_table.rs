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

//! Queue of the changes of the groups, which their committers make one after another.

use diesel::prelude::*;
use uuid::Uuid;

use crate::database::db_handle;

use ainari_common::objects::*;

// Define the schema for mls_operations table
table! {
    mls_operations (uuid) {
        uuid -> Varchar,
        vni -> Integer,
        kind -> Varchar,
        client_id -> Nullable<Varchar>,
        grant_json -> Nullable<Text>,
        status -> Varchar,
        retries -> Integer,
        created_at -> BigInt,
        started_at -> Nullable<BigInt>,
    }
}

/// Operation is waiting for its turn
pub const STATUS_QUEUED: &str = "QUEUED";
/// Operation was handed to the committer
pub const STATUS_INFLIGHT: &str = "INFLIGHT";
/// Operation is finished, successful or not
pub const STATUS_DONE: &str = "DONE";

/// One change of a group. The timestamps are unix-times in milliseconds.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = mls_operations)]
pub struct MlsOperationEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    #[diesel(serialize_as = DbVni, deserialize_as = DbVni)]
    pub vni: u32,
    pub kind: String,
    pub client_id: Option<String>,
    /// JSON-encoded grant of the gateway, which is added
    pub grant_json: Option<String>,
    pub status: String,
    pub retries: i32,
    pub created_at: i64,
    pub started_at: Option<i64>,
}

/// Adds an operation to the queue.
///
/// # Arguments
/// * `entry` - The operation
///
/// # Returns
/// `Ok(())` once the operation is queued
pub fn add_operation(entry: MlsOperationEntry) -> QueryResult<()> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::mls_operations::dsl::*;
    diesel::insert_into(mls_operations)
        .values(entry)
        .execute(&mut *conn)
        .map(|_| ())
}

/// Lists the operations of a group, which are not done yet, in the order they were queued.
///
/// # Arguments
/// * `op_vni` - Tenant of the network
///
/// # Returns
/// The open operations
pub fn list_open_operations(op_vni: u32) -> QueryResult<Vec<MlsOperationEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::mls_operations::dsl::*;
    let mut entries = mls_operations
        .filter(vni.eq(op_vni as i32).and(status.ne(STATUS_DONE)))
        .select(MlsOperationEntry::as_select())
        .load(&mut *conn)?;
    entries.sort_by_key(|entry| entry.created_at);
    Ok(entries)
}

/// Stores the new state of an operation.
///
/// # Arguments
/// * `entry` - The operation with its new state
///
/// # Returns
/// `Ok(())` once the state is stored
pub fn update_operation(entry: &MlsOperationEntry) -> QueryResult<()> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::mls_operations::dsl::*;
    diesel::update(mls_operations.filter(uuid.eq(entry.uuid.to_string())))
        .set((
            status.eq(entry.status.clone()),
            retries.eq(entry.retries),
            started_at.eq(entry.started_at),
        ))
        .execute(&mut *conn)
        .map(|_| ())
}

/// Marks all open operations of a group as done, for example because the group starts again.
///
/// # Arguments
/// * `op_vni` - Tenant of the network
///
/// # Returns
/// The number of dropped operations
pub fn drop_open_operations(op_vni: u32) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::mls_operations::dsl::*;
    diesel::update(mls_operations.filter(vni.eq(op_vni as i32).and(status.ne(STATUS_DONE))))
        .set(status.eq(STATUS_DONE))
        .execute(&mut *conn)
}
