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

/// Represents a dataset entry in the database.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = datasets)]
pub struct DatasetEntry {
    /// Unique identifier for the dataset
    pub uuid: String,
    /// Name of the dataset
    pub name: String,
    /// Address of the Onsen service associated with this dataset
    pub onsen_address: String,
    /// Path to the file containing the dataset
    pub file_path: String,
    /// Secret UUID used for authentication with the dataset
    pub secret_uuid: String,
    /// Number of rows in the dataset
    pub number_of_rows: i64,
    /// Number of columns in the dataset
    pub number_of_columns: i64,
    /// JSON string containing the names of all columns in the dataset
    pub column_names: String,
    /// ID of the user who owns this dataset
    pub owner_id: String,
    /// ID of the project this dataset belongs to
    pub project_id: String,
    /// Status of the dataset (e.g., "ACTIVE", "DELETED")
    pub status: String,
    /// Timestamp when the dataset was created
    pub created_at: String,
    /// ID of the user who created the dataset
    pub created_by: String,
    /// Timestamp when the dataset was last updated
    pub updated_at: String,
    /// ID of the user who last updated the dataset
    pub updated_by: String,
    /// Timestamp when the dataset was deleted (if applicable)
    pub deleted_at: Option<String>,
    /// ID of the user who deleted the dataset (if applicable)
    pub deleted_by: Option<String>,
}

/// Initializes the datasets table in the database.
///
/// This function creates the table if it doesn't already exist.
/// Returns `Ok(())` on success or an error if the operation fails.
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

/// Adds a new dataset to the database.
///
/// This function creates a new `DatasetEntry` with the provided parameters
/// and inserts it into the database.
///
/// # Arguments
/// * `dataset_uuid` - The unique identifier for the new dataset
/// * `dataset_name` - The name of the dataset
/// * `onsen_address` - The address of the Onsen service
/// * `file_path` - The path to the file containing the dataset
/// * `secret_uuid` - The secret UUID for authentication
/// * `dimension` - A tuple containing the number of rows and column names
/// * `context` - The user context containing authentication information
///
/// # Returns
/// * `QueryResult<usize>` - The number of rows affected by the insert operation
pub fn add_new_dataset(
    dataset_uuid: &Uuid,
    dataset_name: &str,
    onsen_address: &str,
    file_path: &str,
    secret_uuid: &Uuid,
    dimension: &(i64, Vec<String>),
    context: &UserContext,
) -> QueryResult<usize> {
    // Serialize the column names vector to a JSON string
    let column_names_str = match serde_json::to_string(&dimension.1) {
        Ok(column_names_str) => column_names_str,
        Err(e) => {
            return Err(diesel::result::Error::DatabaseError(
                DatabaseErrorKind::SerializationFailure,
                Box::new(format!("Failed to serialize column_names with error: {e}")),
            ));
        }
    };

    // Create a new DatasetEntry with the provided parameters
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

/// Adds a dataset to the database.
///
/// This is a helper function that performs the actual database insert operation.
///
/// # Arguments
/// * `dataset` - A reference to the `DatasetEntry` to be inserted
///
/// # Returns
/// * `QueryResult<usize>` - The number of rows affected by the insert operation
pub fn add_dataset(dataset: &DatasetEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::datasets::dsl::*;

    diesel::insert_into(datasets)
        .values(dataset)
        .execute(&mut *conn)
}

/// Retrieves a dataset from the database.
///
/// This function fetches a dataset by its UUID, applying appropriate filters
/// based on the user's permissions.
///
/// # Arguments
/// * `dataset_uuid` - The UUID of the dataset to retrieve
/// * `context` - The user context containing authentication information
///
/// # Returns
/// * `Result<DatasetEntry, enums::DbError>` - The requested dataset or an error
pub fn get_dataset(
    dataset_uuid: &Uuid,
    context: &UserContext,
) -> Result<DatasetEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::datasets::dsl::*;

    // Start building the query with basic filters
    let mut query = datasets
        .filter(uuid.eq(dataset_uuid.to_string()).and(status.eq("ACTIVE")))
        .into_boxed();

    // Apply additional filters based on user permissions
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    // Execute the query and handle the result
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

/// Lists all datasets accessible to the user.
///
/// This function retrieves all datasets that are active and accessible to the user
/// based on their permissions.
///
/// # Arguments
/// * `context` - The user context containing authentication information
///
/// # Returns
/// * `QueryResult<Vec<DatasetEntry>>` - A vector of accessible datasets or an error
pub fn list_datasets(context: &UserContext) -> QueryResult<Vec<DatasetEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::datasets::dsl::*;

    // Start building the query with basic filters
    let mut query = datasets.filter(status.eq("ACTIVE")).into_boxed();

    // Apply additional filters based on user permissions
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    // Execute the query and return the results
    query.select(DatasetEntry::as_select()).load(&mut *conn)
}

/// Counts the number of datasets accessible to the user.
///
/// This function counts all datasets that are active and owned by the user.
///
/// # Arguments
/// * `context` - The user context containing authentication information
///
/// # Returns
/// * `QueryResult<i64>` - The count of accessible datasets or an error
pub fn count_datasets(context: &UserContext) -> QueryResult<i64> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::datasets::dsl::*;

    // Start building the query with basic filters
    let mut query = datasets.filter(status.eq("ACTIVE")).into_boxed();

    // Apply filters to only count datasets owned by the user
    query = query.filter(project_id.eq(context.project_id.clone()));
    query = query.filter(owner_id.eq(context.user_id.clone()));

    // Execute the query and return the count
    query.select(count_star()).first::<i64>(&mut *conn)
}

/// Deletes a dataset from the database.
///
/// This function marks a dataset as deleted by updating its status and setting
/// the deletion timestamp and user.
///
/// # Arguments
/// * `dataset_uuid` - The UUID of the dataset to delete
/// * `context` - The user context containing authentication information
///
/// # Returns
/// * `Result<(), enums::DbError>` - Success or an error
pub fn delete_dataset(dataset_uuid: &Uuid, context: &UserContext) -> Result<(), enums::DbError> {
    // First verify that the dataset exists and is accessible to the user
    get_dataset(dataset_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::datasets::dsl::*;

    // Update the dataset status and deletion information
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
