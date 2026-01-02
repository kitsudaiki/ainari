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

use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use std::error::Error;
use uuid::Uuid;

use crate::database::db_handle;

use ainari_common::enums;

// Define the schema
table! {
    simple_crypto (secret_uuid) {
        secret_uuid -> Varchar,
        encrypted_secret -> Varchar,
    }
}

#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = simple_crypto)]
pub struct SimpleCryptoEntry {
    pub secret_uuid: String,
    pub encrypted_secret: String,
}

pub fn init_simple_crypto_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS simple_crypto (
        secret_uuid VARCHAR(40) PRIMARY KEY,
        encrypted_secret TEXT
    );",
    )?;

    Ok(())
}

pub fn add_new_simple_crypto_data(uuid: &Uuid, encrypted_secret: &str) -> QueryResult<usize> {
    let secret = SimpleCryptoEntry {
        secret_uuid: uuid.to_string().clone(),
        encrypted_secret: encrypted_secret.to_owned(),
    };

    add_secret(&secret)
}

pub fn add_secret(secret: &SimpleCryptoEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::simple_crypto::dsl::*;
    diesel::insert_into(simple_crypto)
        .values(secret)
        .execute(&mut *conn)
}

pub fn get_secret(uuid: &Uuid) -> Result<SimpleCryptoEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::simple_crypto::dsl::*;

    let query = simple_crypto
        .filter(secret_uuid.eq(uuid.to_string()))
        .into_boxed();

    match query
        .select(SimpleCryptoEntry::as_select())
        .first::<SimpleCryptoEntry>(&mut *conn)
    {
        Ok(secret) => Ok(secret),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

#[allow(dead_code)]
pub fn list_simple_crypto() -> QueryResult<Vec<SimpleCryptoEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::simple_crypto::dsl::*;
    simple_crypto.load::<SimpleCryptoEntry>(&mut *conn)
}

pub fn delete_secret(uuid: &Uuid) -> Result<(), enums::DbError> {
    get_secret(uuid)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::simple_crypto::dsl::*;
    match diesel::delete(simple_crypto.filter(secret_uuid.eq(uuid.to_string()))).execute(&mut *conn)
    {
        Ok(_) => Ok(()),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_add_get_secret() {
        let _ = init_simple_crypto_table();
        let uuid1 = Uuid::new_v4();
        let encrypted_secret = "just a dummy-secret".to_string();

        let secret = SimpleCryptoEntry {
            secret_uuid: uuid1.to_string(),
            encrypted_secret: encrypted_secret.clone(),
        };

        let _ = delete_secret(&uuid1);

        add_secret(&secret).unwrap();
        match get_secret(&uuid1) {
            Ok(retrieved_secret) => {
                assert_eq!(retrieved_secret.secret_uuid, secret.secret_uuid);
                assert_eq!(retrieved_secret.encrypted_secret, secret.encrypted_secret);
            }
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        let _ = delete_secret(&uuid1);
    }

    #[test]
    #[serial]
    fn test_list_simple_crypto() {
        let _ = init_simple_crypto_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let encrypted_secret = "just a dummy-secret".to_string();

        let secret1 = SimpleCryptoEntry {
            secret_uuid: uuid1.to_string(),
            encrypted_secret: encrypted_secret.clone(),
        };

        let secret2 = SimpleCryptoEntry {
            secret_uuid: uuid2.to_string(),
            encrypted_secret: encrypted_secret.clone(),
        };

        let _ = delete_secret(&uuid1);
        let _ = delete_secret(&uuid2);

        add_secret(&secret1).unwrap();
        add_secret(&secret2).unwrap();
        let simple_crypto = list_simple_crypto().unwrap();
        assert_eq!(simple_crypto.len(), 2);
        let _ = delete_secret(&uuid1);
        let _ = delete_secret(&uuid2);
    }

    #[test]
    #[serial]
    fn test_delete_secret() {
        let _ = init_simple_crypto_table();
        let uuid1 = Uuid::new_v4();
        let encrypted_secret = "just a dummy-secret".to_string();

        let secret = SimpleCryptoEntry {
            secret_uuid: uuid1.to_string(),
            encrypted_secret: encrypted_secret.clone(),
        };

        let _ = delete_secret(&uuid1);

        add_secret(&secret).unwrap();
        let _ = delete_secret(&uuid1);
        let result = get_secret(&uuid1);
        assert!(result.is_err());
    }
}
