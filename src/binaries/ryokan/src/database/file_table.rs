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

use crate::database::db_handle;

use ainari_common::enums;

// Define the schema
table! {
    files (file_path_hash) {
        file_path_hash -> Varchar,
        file_path -> Varchar,
        file_size -> BigInt,
    }
}

#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = files)]
pub struct FileEntry {
    pub file_path_hash: String,
    pub file_path: String,
    pub file_size: i64,
}

pub fn init_file_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS files (
        file_path_hash VARCHAR(40) PRIMARY KEY,
        file_path VARCHAR(1024),
        file_size BIGINT
    );",
    )?;

    Ok(())
}

pub fn add_new_file(file_path_hash: &str, file_path: &str, file_size: u64) -> QueryResult<usize> {
    let file = FileEntry {
        file_path_hash: file_path_hash.to_owned(),
        file_path: file_path.to_owned(),
        file_size: file_size as i64,
    };

    add_file(&file)
}

pub fn add_file(file: &FileEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::files::dsl::*;
    diesel::insert_into(files).values(file).execute(&mut *conn)
}

pub fn get_file(path_hash: &String) -> Result<FileEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::files::dsl::*;

    let query = files
        .filter(file_path_hash.eq(path_hash.to_string()))
        .into_boxed();

    match query
        .select(FileEntry::as_select())
        .first::<FileEntry>(&mut *conn)
    {
        Ok(file) => Ok(file),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

pub fn list_files() -> QueryResult<Vec<FileEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::files::dsl::*;

    files.select(FileEntry::as_select()).load(&mut *conn)
}

pub fn delete_file(path_hash: &String) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::files::dsl::*;
    match diesel::delete(files.filter(file_path_hash.eq(path_hash.to_string()))).execute(&mut *conn)
    {
        Ok(_) => Ok(()),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

pub fn delete_all_file() -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::files::dsl::*;
    match diesel::delete(files).execute(&mut *conn) {
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

    use ainari_common::functions::create_sha256_hash;

    #[test]
    #[serial]
    fn test_add_get_file() {
        let _ = init_file_table();
        let file_path: String = "/tmp/test-file".to_string();
        let file_path_hash = create_sha256_hash(&file_path);
        let file_size = 42;

        let file = FileEntry {
            file_path_hash: file_path_hash.clone(),
            file_path: file_path.clone(),
            file_size,
        };

        let _ = delete_file(&file_path_hash);

        add_file(&file).unwrap();
        match get_file(&file_path_hash) {
            Ok(retrieved_file) => {
                assert_eq!(retrieved_file.file_path, file.file_path);
                assert_eq!(retrieved_file.file_path_hash, file.file_path_hash);
                assert_eq!(retrieved_file.file_size, file.file_size);
            }
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        let _ = delete_file(&file_path_hash);
    }

    #[test]
    #[serial]
    fn test_list_files() {
        let _ = init_file_table();
        let file_path1: String = "/tmp/test-file1".to_string();
        let file_path_hash1 = create_sha256_hash(&file_path1);
        let file_size1 = 42;

        let file_path2: String = "/tmp/test-file2".to_string();
        let file_path_hash2 = create_sha256_hash(&file_path2);
        let file_size2 = 43;

        let file1 = FileEntry {
            file_path_hash: file_path_hash1.clone(),
            file_path: file_path1.clone(),
            file_size: file_size1,
        };

        let file2 = FileEntry {
            file_path_hash: file_path_hash2.clone(),
            file_path: file_path2.clone(),
            file_size: file_size2,
        };

        let _ = delete_file(&file_path_hash1);
        let _ = delete_file(&file_path_hash2);

        add_file(&file1).unwrap();
        add_file(&file2).unwrap();
        let files = list_files().unwrap();
        assert_eq!(files.len(), 2);
        let _ = delete_file(&file_path_hash1);
        let _ = delete_file(&file_path_hash2);
    }

    #[test]
    #[serial]
    fn test_delete_file() {
        let _ = init_file_table();
        let file_path: String = "/tmp/test-file".to_string();
        let file_path_hash = create_sha256_hash(&file_path);
        let file_size = 42;

        let file = FileEntry {
            file_path_hash: file_path_hash.clone(),
            file_path: file_path.clone(),
            file_size,
        };

        let _ = delete_file(&file_path_hash);

        add_file(&file).unwrap();
        let _ = delete_file(&file_path_hash);
        let result = get_file(&file_path_hash);
        assert!(result.is_err());
    }
}
