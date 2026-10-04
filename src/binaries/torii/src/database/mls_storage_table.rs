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

//! Persistence of the MLS-client of the gateway.
//!
//! openmls keeps its whole state - the identity of the gateway, its unused key-packages and the
//! secrets of every group - in a key-value-store. That store is kept in memory and written into
//! this table after every change, so the gateway is still a member of its groups after a restart.

use std::collections::HashMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use diesel::prelude::*;

use crate::database::db_handle;

// Define the schema for the mls_storage table
table! {
    mls_storage (storage_key) {
        storage_key -> Text,
        storage_value -> Text,
    }
}

/// One entry of the key-value-store, both sides base64-encoded
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = mls_storage)]
pub struct MlsStorageEntry {
    pub storage_key: String,
    pub storage_value: String,
}

/// Loads the whole key-value-store of the MLS-client.
///
/// # Returns
/// The decoded store, or an error-message, if it can not be read or an entry is broken
pub fn load_mls_storage() -> Result<HashMap<Vec<u8>, Vec<u8>>, String> {
    let entries = {
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        use self::mls_storage::dsl::*;
        mls_storage
            .select(MlsStorageEntry::as_select())
            .load(&mut *conn)
            .map_err(|e| format!("Failed to read the mls-storage: {e}"))?
    };

    let mut values = HashMap::with_capacity(entries.len());
    for entry in entries {
        let key = BASE64
            .decode(&entry.storage_key)
            .map_err(|e| format!("Broken key in the mls-storage: {e}"))?;
        let value = BASE64
            .decode(&entry.storage_value)
            .map_err(|e| format!("Broken value in the mls-storage: {e}"))?;
        values.insert(key, value);
    }

    Ok(values)
}

/// Replaces the whole key-value-store of the MLS-client.
///
/// The store is written within one transaction, so a failure leaves the previous state intact
/// instead of a mix of two states, which openmls could not use anymore.
///
/// # Arguments
/// * `values` - The current key-value-store
///
/// # Returns
/// `Ok(())` once the store is written, otherwise the database-error
pub fn save_mls_storage(values: &HashMap<Vec<u8>, Vec<u8>>) -> QueryResult<()> {
    let entries: Vec<MlsStorageEntry> = values
        .iter()
        .map(|(key, value)| MlsStorageEntry {
            storage_key: BASE64.encode(key),
            storage_value: BASE64.encode(value),
        })
        .collect();

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.transaction(|conn| {
        use self::mls_storage::dsl::*;
        diesel::delete(mls_storage).execute(conn)?;
        for entry in entries {
            diesel::insert_into(mls_storage)
                .values(entry)
                .execute(conn)?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn the_store_survives_a_round_trip() {
        let previous = load_mls_storage().unwrap();

        let mut values = HashMap::new();
        values.insert(b"key".to_vec(), vec![0u8, 1, 2, 255]);
        values.insert(vec![0xff, 0x00], Vec::new());
        save_mls_storage(&values).unwrap();
        assert_eq!(load_mls_storage().unwrap(), values);

        // a smaller store replaces the bigger one completely
        values.remove(b"key".as_slice());
        save_mls_storage(&values).unwrap();
        assert_eq!(load_mls_storage().unwrap(), values);

        save_mls_storage(&previous).unwrap();
    }
}
