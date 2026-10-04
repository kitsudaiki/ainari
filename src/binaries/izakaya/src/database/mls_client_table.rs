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

//! Last contact of every gateway, which tells izakaya, if a gateway is still alive.

use diesel::prelude::*;

use crate::database::db_handle;

// Define the schema for mls_clients table
table! {
    mls_clients (client_id) {
        client_id -> Varchar,
        last_seen -> BigInt,
    }
}

#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = mls_clients)]
pub struct MlsClientEntry {
    pub client_id: String,
    /// Unix-time in seconds of the last contact
    pub last_seen: i64,
}

/// Records a contact of a gateway.
///
/// # Arguments
/// * `seen_client_id` - Identity of the gateway
/// * `now` - Current unix-time in seconds
///
/// # Returns
/// `Ok(())` once the contact is recorded
pub fn touch_client(seen_client_id: &str, now: i64) -> QueryResult<()> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.transaction(|conn| {
        use self::mls_clients::dsl::*;
        let updated = diesel::update(mls_clients.filter(client_id.eq(seen_client_id)))
            .set(last_seen.eq(now))
            .execute(conn)?;
        if updated == 0 {
            diesel::insert_into(mls_clients)
                .values(MlsClientEntry {
                    client_id: seen_client_id.to_string(),
                    last_seen: now,
                })
                .execute(conn)?;
        }
        Ok(())
    })
}

/// Returns the last contact of a gateway.
///
/// # Arguments
/// * `seen_client_id` - Identity of the gateway
///
/// # Returns
/// The unix-time in seconds of the last contact, or `None` if the gateway never got in touch
pub fn last_seen_of(seen_client_id: &str) -> QueryResult<Option<i64>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::mls_clients::dsl::*;
    mls_clients
        .filter(client_id.eq(seen_client_id))
        .select(last_seen)
        .first(&mut *conn)
        .optional()
}
