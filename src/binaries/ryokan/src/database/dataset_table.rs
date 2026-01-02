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

use chrono::Utc;
use diesel::connection::SimpleConnection;
use diesel::dsl::count_star;
use diesel::prelude::*;
use diesel::result::DatabaseErrorKind;
use std::error::Error;
use uuid::Uuid;

use crate::database::db_handle;
use ainari_api_structs::user_context::UserContext;

use ainari_common::enums;

// Define the schema
table! {
    datasets (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        onsen_address -> Varchar,
        file_path -> Text,
        secret_uuid -> Varchar,
        number_of_rows -> BigInt,
        number_of_columns -> BigInt,
        column_names -> Text,
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

#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = datasets)]
pub struct DatasetEntry {
    pub uuid: String,
    pub name: String,
    pub onsen_address: String,
    pub file_path: String,
    pub secret_uuid: String,
    pub number_of_rows: i64,
    pub number_of_columns: i64,
    pub column_names: String,
    pub owner_id: String,
    pub project_id: String,
    pub status: String,
    pub created_at: String,
    pub created_by: String,
    pub updated_at: String,
    pub updated_by: String,
    pub deleted_at: Option<String>,
    pub deleted_by: Option<String>,
}

pub fn init_dataset_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS datasets (
        uuid VARCHAR(40) PRIMARY KEY,
        name VARCHAR(256),
        onsen_address VARCHAR(256),
        file_path TEXT,
        secret_uuid VARCHAR(40),
        number_of_rows BIGINT,
        number_of_columns BIGINT,
        column_names TEXT,
        owner_id VARCHAR(256),
        project_id VARCHAR(256),
        status VARCHAR(8),
        created_at VARCHAR(64),
        created_by VARCHAR(256),
        updated_at VARCHAR(64),
        updated_by VARCHAR(256),
        deleted_at VARCHAR(64),
        deleted_by VARCHAR(256)
    );",
    )?;

    Ok(())
}

pub fn add_new_dataset(
    dataset_uuid: &Uuid,
    dataset_name: &str,
    onsen_address: &str,
    file_path: &str,
    secret_uuid: &Uuid,
    dimension: &(i64, Vec<String>),
    context: &UserContext,
) -> QueryResult<usize> {
    let column_names_str = match serde_json::to_string(&dimension.1) {
        Ok(column_names_str) => column_names_str,
        Err(e) => {
            return Err(diesel::result::Error::DatabaseError(
                DatabaseErrorKind::SerializationFailure,
                Box::new(format!("Failed to serialize column_names with error: {e}")),
            ));
        }
    };

    let dataset = DatasetEntry {
        uuid: dataset_uuid.to_string().clone(),
        name: dataset_name.to_owned(),
        onsen_address: onsen_address.to_owned(),
        file_path: file_path.to_owned(),
        secret_uuid: secret_uuid.to_string().clone(),
        number_of_rows: dimension.0,
        number_of_columns: dimension.1.len() as i64,
        column_names: column_names_str,
        owner_id: context.user_id.clone(),
        project_id: context.project_id.clone(),
        status: "ACTIVE".to_string(),
        created_at: Utc::now().to_rfc3339(),
        created_by: context.user_id.clone(),
        updated_at: Utc::now().to_rfc3339(),
        updated_by: context.user_id.clone(),
        deleted_at: None,
        deleted_by: None,
    };

    add_dataset(&dataset)
}

pub fn add_dataset(dataset: &DatasetEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::datasets::dsl::*;

    diesel::insert_into(datasets)
        .values(dataset)
        .execute(&mut *conn)
}

pub fn get_dataset(
    dataset_uuid: &Uuid,
    context: &UserContext,
) -> Result<DatasetEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::datasets::dsl::*;

    let mut query = datasets
        .filter(uuid.eq(dataset_uuid.to_string()).and(status.eq("ACTIVE")))
        .into_boxed();

    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    match query
        .select(DatasetEntry::as_select())
        .first::<DatasetEntry>(&mut *conn)
    {
        Ok(dataset) => Ok(dataset),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

pub fn list_datasets(context: &UserContext) -> QueryResult<Vec<DatasetEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::datasets::dsl::*;

    let mut query = datasets.filter(status.eq("ACTIVE")).into_boxed();

    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    query.select(DatasetEntry::as_select()).load(&mut *conn)
}

pub fn count_datasets(context: &UserContext) -> QueryResult<i64> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::datasets::dsl::*;

    let mut query = datasets.filter(status.eq("ACTIVE")).into_boxed();

    query = query.filter(project_id.eq(context.project_id.clone()));
    query = query.filter(owner_id.eq(context.user_id.clone()));

    query.select(count_star()).first::<i64>(&mut *conn)
}

pub fn delete_dataset(dataset_uuid: &Uuid, context: &UserContext) -> Result<(), enums::DbError> {
    get_dataset(dataset_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::datasets::dsl::*;

    match diesel::update(datasets.filter(uuid.eq(dataset_uuid.to_string())))
        .set((
            status.eq("DELETED"),
            deleted_at.eq(Utc::now().to_rfc3339()),
            deleted_by.eq(context.user_id.clone()),
        ))
        .execute(&mut *conn)
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

    fn hard_delete_dataset(dataset_uuid: &Uuid) {
        use self::datasets::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ =
            diesel::delete(datasets.filter(uuid.eq(dataset_uuid.to_string()))).execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_add_get_dataset() {
        let _ = init_dataset_table();
        let uuid1 = Uuid::new_v4();
        let onsen_address = "127.0.0.1:1234".to_string();
        let secret_uuid = Uuid::new_v4();
        let number_of_rows = 42;
        let number_of_columns = 43;

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        let column_names = "[\"input\", \"output\"]".to_string();

        let dataset = DatasetEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string(),
            number_of_rows,
            number_of_columns,
            column_names,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_dataset(&uuid1);

        add_dataset(&dataset).unwrap();
        if let Ok(retrieved_dataset) = get_dataset(&uuid1, &context) {
            assert_eq!(retrieved_dataset.uuid, dataset.uuid);
            assert_eq!(retrieved_dataset.name, dataset.name);
            assert_eq!(retrieved_dataset.file_path, dataset.file_path);
            assert_eq!(retrieved_dataset.secret_uuid, dataset.secret_uuid);
            assert_eq!(retrieved_dataset.number_of_rows, dataset.number_of_rows);
            assert_eq!(
                retrieved_dataset.number_of_columns,
                dataset.number_of_columns
            );
            assert_eq!(retrieved_dataset.status, dataset.status);
            assert_eq!(retrieved_dataset.created_by, dataset.created_by);
            assert_eq!(retrieved_dataset.updated_by, dataset.updated_by);
            assert_eq!(retrieved_dataset.deleted_at, dataset.deleted_at);
            assert_eq!(retrieved_dataset.deleted_by, dataset.deleted_by);
        };

        hard_delete_dataset(&uuid1);
    }

    #[test]
    #[serial]
    fn test_list_datasets() {
        let _ = init_dataset_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let onsen_address = "127.0.0.1:1234".to_string();
        let secret_uuid = Uuid::new_v4();
        let number_of_rows = 42;
        let number_of_columns = 43;

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        let column_names = "[\"input\", \"output\"]".to_string();

        let dataset1 = DatasetEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string(),
            number_of_rows,
            number_of_columns,
            column_names: column_names.clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let dataset2 = DatasetEntry {
            uuid: uuid2.to_string(),
            name: "Bob".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string(),
            number_of_rows,
            number_of_columns,
            column_names: column_names.clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "DELETED".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_dataset(&uuid1);
        hard_delete_dataset(&uuid2);

        add_dataset(&dataset1).unwrap();
        add_dataset(&dataset2).unwrap();
        let datasets = list_datasets(&context).unwrap();
        assert_eq!(datasets.len(), 1);
        hard_delete_dataset(&uuid1);
        hard_delete_dataset(&uuid2);
    }

    #[test]
    #[serial]
    fn test_delete_dataset() {
        let _ = init_dataset_table();
        let uuid1 = Uuid::new_v4();
        let onsen_address = "127.0.0.1:1234".to_string();
        let secret_uuid = Uuid::new_v4();
        let number_of_rows = 42;
        let number_of_columns = 43;

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        let column_names = "[\"input\", \"output\"]".to_string();

        let dataset = DatasetEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string(),
            number_of_rows,
            number_of_columns,
            column_names: column_names.clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_dataset(&uuid1);

        add_dataset(&dataset).unwrap();
        let _ = delete_dataset(&uuid1, &context);
        let result = get_dataset(&uuid1, &context);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_count_datasets() {
        let _ = init_dataset_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let name = "test-dataset".to_string();
        let onsen_address = "127.0.0.1:1234".to_string();
        let secret_uuid = Uuid::new_v4();
        let number_of_rows = 42;
        let number_of_columns = 43;

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        let column_names = "[\"input\", \"output\"]".to_string();

        let dataset1 = DatasetEntry {
            uuid: uuid1.to_string(),
            name: name.clone(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string(),
            number_of_rows,
            number_of_columns,
            column_names: column_names.clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let dataset2 = DatasetEntry {
            uuid: uuid2.to_string(),
            name: name.clone(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string(),
            number_of_rows,
            number_of_columns,
            column_names: column_names.clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let dataset3 = DatasetEntry {
            uuid: uuid3.to_string(),
            name: name.clone(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string(),
            number_of_rows,
            number_of_columns,
            column_names: column_names.clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_dataset(&uuid1);
        hard_delete_dataset(&uuid2);
        hard_delete_dataset(&uuid3);

        add_dataset(&dataset1).unwrap();
        add_dataset(&dataset2).unwrap();
        add_dataset(&dataset3).unwrap();

        let number = count_datasets(&context).unwrap();
        assert_eq!(number, 3);

        hard_delete_dataset(&uuid1);
        hard_delete_dataset(&uuid2);
        hard_delete_dataset(&uuid3);
    }

    #[test]
    #[serial]
    fn test_datasets_permissions() {
        let _ = init_dataset_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let onsen_address = "127.0.0.1:1234".to_string();
        let secret_uuid = Uuid::new_v4();
        let number_of_rows = 42;
        let number_of_columns = 43;
        let column_names = "[\"input\", \"output\"]".to_string();

        let dataset1 = DatasetEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string(),
            number_of_rows,
            number_of_columns,
            column_names: column_names.clone(),
            owner_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let dataset2 = DatasetEntry {
            uuid: uuid2.to_string(),
            name: "Bob".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string(),
            number_of_rows,
            number_of_columns,
            column_names: column_names.clone(),
            owner_id: "test-user-43".to_string(),
            project_id: "test_permissions_1".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let dataset3 = DatasetEntry {
            uuid: uuid3.to_string(),
            name: "Poi".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string(),
            number_of_rows,
            number_of_columns,
            column_names: column_names.clone(),
            owner_id: "test-user-44".to_string(),
            project_id: "test_permissions_2".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_dataset(&uuid1);
        hard_delete_dataset(&uuid2);
        hard_delete_dataset(&uuid3);

        add_dataset(&dataset1).unwrap();
        add_dataset(&dataset2).unwrap();
        add_dataset(&dataset3).unwrap();

        // list-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        let datasets = list_datasets(&context).unwrap();
        assert_eq!(datasets.len(), 1);

        // list-test project-admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: true.to_string(),
        };
        let datasets = list_datasets(&context).unwrap();
        assert_eq!(datasets.len(), 2);

        // list-test admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: true.to_string(),
            is_project_admin: false.to_string(),
        };
        let datasets = list_datasets(&context).unwrap();
        assert_eq!(datasets.len(), 3);

        // get-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        match get_dataset(&uuid1, &context) {
            Ok(retrieved_dataset) => {
                assert_eq!(retrieved_dataset.uuid, uuid1.to_string());
            }
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        // get-test normal user false uuid
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        if get_dataset(&uuid3, &context).is_ok() {
            assert_eq!(true, false);
        };

        // delete-test normal user false uuid
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        if delete_dataset(&uuid3, &context).is_ok() {
            assert_eq!(true, false);
        };

        hard_delete_dataset(&uuid1);
        hard_delete_dataset(&uuid2);
        hard_delete_dataset(&uuid3);
    }
}
