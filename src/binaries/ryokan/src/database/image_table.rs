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
    images (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        onsen_address -> Varchar,
        file_path -> Text,
        secret_uuid -> Varchar,
        number_of_rows -> BigInt,
        number_of_columns -> BigInt,
        column_names -> Text,
        is_snapshot -> Bool,
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

/// Represents an image entry in the database.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = images)]
pub struct ImageEntry {
    /// Unique identifier for the image
    pub uuid: String,
    /// Name of the image
    pub name: String,
    /// Address of the Onsen service associated with this image
    pub onsen_address: String,
    /// Path to the file containing the image
    pub file_path: String,
    /// Secret UUID used for authentication with the image
    pub secret_uuid: String,
    /// Number of rows in the image
    pub number_of_rows: i64,
    /// Number of columns in the image
    pub number_of_columns: i64,
    /// JSON string containing the names of all columns in the image
    pub column_names: String,
    /// True, if the image is a snapshot of the root-disk of a virtual_machine
    pub is_snapshot: bool,
    /// ID of the user who owns this image
    pub owner_id: String,
    /// ID of the project this image belongs to
    pub project_id: String,
    /// Status of the image (e.g., "ACTIVE", "DELETED")
    pub status: String,
    /// Timestamp when the image was created
    pub created_at: String,
    /// ID of the user who created the image
    pub created_by: String,
    /// Timestamp when the image was last updated
    pub updated_at: String,
    /// ID of the user who last updated the image
    pub updated_by: String,
    /// Timestamp when the image was deleted (if applicable)
    pub deleted_at: Option<String>,
    /// ID of the user who deleted the image (if applicable)
    pub deleted_by: Option<String>,
}

/// Initializes the images table in the database.
///
/// This function creates the table if it doesn't already exist.
/// Returns `Ok(())` on success or an error if the operation fails.
pub fn init_image_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS images (
        uuid VARCHAR(40) PRIMARY KEY,
        name VARCHAR(256),
        onsen_address VARCHAR(256),
        file_path TEXT,
        secret_uuid VARCHAR(40),
        number_of_rows BIGINT,
        number_of_columns BIGINT,
        column_names TEXT,
        is_snapshot BOOLEAN NOT NULL DEFAULT FALSE,
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

    // added separately, so it is also added to tables of older versions. Snapshots were stored
    // in their own table in older versions, so all existing images are no snapshots.
    match conn
        .batch_execute("ALTER TABLE images ADD COLUMN is_snapshot BOOLEAN NOT NULL DEFAULT FALSE;")
    {
        Ok(()) => {}
        Err(e) if e.to_string().contains("duplicate column name") => {}
        Err(e) => return Err(e.into()),
    }

    Ok(())
}

/// Adds a new image to the database.
///
/// This function creates a new `ImageEntry` with the provided parameters
/// and inserts it into the database.
///
/// # Arguments
/// * `image_uuid` - The unique identifier for the new image
/// * `image_name` - The name of the image
/// * `onsen_address` - The address of the Onsen service
/// * `file_path` - The path to the file containing the image
/// * `secret_uuid` - The secret UUID for authentication
/// * `dimension` - A tuple containing the number of rows and column names
/// * `is_snapshot` - True, if the image is a snapshot of the root-disk of a virtual_machine
/// * `context` - The user context containing authentication information
///
/// # Returns
/// * `QueryResult<usize>` - The number of rows affected by the insert operation
#[allow(clippy::too_many_arguments)]
pub fn add_new_image(
    image_uuid: &Uuid,
    image_name: &str,
    onsen_address: &str,
    file_path: &str,
    secret_uuid: &Uuid,
    dimension: &(i64, Vec<String>),
    is_snapshot: bool,
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

    // Create a new ImageEntry with the provided parameters
    let image = ImageEntry {
        uuid: image_uuid.to_string().clone(),
        name: image_name.to_owned(),
        onsen_address: onsen_address.to_owned(),
        file_path: file_path.to_owned(),
        secret_uuid: secret_uuid.to_string().clone(),
        number_of_rows: dimension.0,
        number_of_columns: dimension.1.len() as i64,
        column_names: column_names_str,
        is_snapshot,
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

    add_image(&image)
}

/// Adds an image to the database.
///
/// This is a helper function that performs the actual database insert operation.
///
/// # Arguments
/// * `image` - A reference to the `ImageEntry` to be inserted
///
/// # Returns
/// * `QueryResult<usize>` - The number of rows affected by the insert operation
pub fn add_image(image: &ImageEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::images::dsl::*;

    diesel::insert_into(images)
        .values(image)
        .execute(&mut *conn)
}

/// Retrieves an image from the database.
///
/// This function fetches an image by its UUID, applying appropriate filters
/// based on the user's permissions.
///
/// # Arguments
/// * `image_uuid` - The UUID of the image to retrieve
/// * `context` - The user context containing authentication information
///
/// # Returns
/// * `Result<ImageEntry, enums::DbError>` - The requested image or an error
pub fn get_image(image_uuid: &Uuid, context: &UserContext) -> Result<ImageEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::images::dsl::*;

    // Start building the query with basic filters
    let mut query = images
        .filter(uuid.eq(image_uuid.to_string()).and(status.eq("ACTIVE")))
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
        .select(ImageEntry::as_select())
        .first::<ImageEntry>(&mut *conn)
    {
        Ok(image) => Ok(image),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Lists all images accessible to the user.
///
/// This function retrieves all images that are active and accessible to the user
/// based on their permissions.
///
/// # Arguments
/// * `context` - The user context containing authentication information
///
/// # Returns
/// * `QueryResult<Vec<ImageEntry>>` - A vector of accessible images or an error
pub fn list_images(context: &UserContext) -> QueryResult<Vec<ImageEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::images::dsl::*;

    // Start building the query with basic filters
    let mut query = images.filter(status.eq("ACTIVE")).into_boxed();

    // Apply additional filters based on user permissions
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    // Execute the query and return the results
    query.select(ImageEntry::as_select()).load(&mut *conn)
}

/// Counts the number of images accessible to the user.
///
/// This function counts all images that are active and owned by the user.
///
/// # Arguments
/// * `context` - The user context containing authentication information
///
/// # Returns
/// * `QueryResult<i64>` - The count of accessible images or an error
pub fn count_images(context: &UserContext) -> QueryResult<i64> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::images::dsl::*;

    // Start building the query with basic filters
    let mut query = images.filter(status.eq("ACTIVE")).into_boxed();

    // Apply filters to only count images owned by the user
    query = query.filter(project_id.eq(context.project_id.clone()));
    query = query.filter(owner_id.eq(context.user_id.clone()));

    // Execute the query and return the count
    query.select(count_star()).first::<i64>(&mut *conn)
}

/// Deletes an image from the database.
///
/// This function marks an image as deleted by updating its status and setting
/// the deletion timestamp and user.
///
/// # Arguments
/// * `image_uuid` - The UUID of the image to delete
/// * `context` - The user context containing authentication information
///
/// # Returns
/// * `Result<(), enums::DbError>` - Success or an error
pub fn delete_image(image_uuid: &Uuid, context: &UserContext) -> Result<(), enums::DbError> {
    // First verify that the image exists and is accessible to the user
    get_image(image_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::images::dsl::*;

    // Update the image status and deletion information
    match diesel::update(images.filter(uuid.eq(image_uuid.to_string())))
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

    fn hard_delete_image(image_uuid: &Uuid) {
        use self::images::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(images.filter(uuid.eq(image_uuid.to_string()))).execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_add_get_image() {
        let _ = init_image_table();
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

        let image = ImageEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string(),
            number_of_rows,
            number_of_columns,
            column_names,
            is_snapshot: true,
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

        hard_delete_image(&uuid1);

        add_image(&image).unwrap();
        if let Ok(retrieved_image) = get_image(&uuid1, &context) {
            assert_eq!(retrieved_image.uuid, image.uuid);
            assert_eq!(retrieved_image.name, image.name);
            assert_eq!(retrieved_image.file_path, image.file_path);
            assert_eq!(retrieved_image.secret_uuid, image.secret_uuid);
            assert_eq!(retrieved_image.number_of_rows, image.number_of_rows);
            assert_eq!(retrieved_image.number_of_columns, image.number_of_columns);
            assert_eq!(retrieved_image.is_snapshot, image.is_snapshot);
            assert_eq!(retrieved_image.status, image.status);
            assert_eq!(retrieved_image.created_by, image.created_by);
            assert_eq!(retrieved_image.updated_by, image.updated_by);
            assert_eq!(retrieved_image.deleted_at, image.deleted_at);
            assert_eq!(retrieved_image.deleted_by, image.deleted_by);
        };

        hard_delete_image(&uuid1);
    }

    #[test]
    #[serial]
    fn test_list_images() {
        let _ = init_image_table();
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

        let image1 = ImageEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string(),
            number_of_rows,
            number_of_columns,
            column_names: column_names.clone(),
            is_snapshot: false,
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

        let image2 = ImageEntry {
            uuid: uuid2.to_string(),
            name: "Bob".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string(),
            number_of_rows,
            number_of_columns,
            column_names: column_names.clone(),
            is_snapshot: false,
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

        hard_delete_image(&uuid1);
        hard_delete_image(&uuid2);

        add_image(&image1).unwrap();
        add_image(&image2).unwrap();
        let images = list_images(&context).unwrap();
        assert_eq!(images.len(), 1);
        hard_delete_image(&uuid1);
        hard_delete_image(&uuid2);
    }

    #[test]
    #[serial]
    fn test_delete_image() {
        let _ = init_image_table();
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

        let image = ImageEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string(),
            number_of_rows,
            number_of_columns,
            column_names: column_names.clone(),
            is_snapshot: false,
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

        hard_delete_image(&uuid1);

        add_image(&image).unwrap();
        let _ = delete_image(&uuid1, &context);
        let result = get_image(&uuid1, &context);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_count_images() {
        let _ = init_image_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let name = "test-image".to_string();
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

        let image1 = ImageEntry {
            uuid: uuid1.to_string(),
            name: name.clone(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string(),
            number_of_rows,
            number_of_columns,
            column_names: column_names.clone(),
            is_snapshot: false,
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

        let image2 = ImageEntry {
            uuid: uuid2.to_string(),
            name: name.clone(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string(),
            number_of_rows,
            number_of_columns,
            column_names: column_names.clone(),
            is_snapshot: false,
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

        let image3 = ImageEntry {
            uuid: uuid3.to_string(),
            name: name.clone(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string(),
            number_of_rows,
            number_of_columns,
            column_names: column_names.clone(),
            is_snapshot: false,
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

        hard_delete_image(&uuid1);
        hard_delete_image(&uuid2);
        hard_delete_image(&uuid3);

        add_image(&image1).unwrap();
        add_image(&image2).unwrap();
        add_image(&image3).unwrap();

        let number = count_images(&context).unwrap();
        assert_eq!(number, 3);

        hard_delete_image(&uuid1);
        hard_delete_image(&uuid2);
        hard_delete_image(&uuid3);
    }

    #[test]
    #[serial]
    fn test_images_permissions() {
        let _ = init_image_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let onsen_address = "127.0.0.1:1234".to_string();
        let secret_uuid = Uuid::new_v4();
        let number_of_rows = 42;
        let number_of_columns = 43;
        let column_names = "[\"input\", \"output\"]".to_string();

        let image1 = ImageEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string(),
            number_of_rows,
            number_of_columns,
            column_names: column_names.clone(),
            is_snapshot: false,
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

        let image2 = ImageEntry {
            uuid: uuid2.to_string(),
            name: "Bob".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string(),
            number_of_rows,
            number_of_columns,
            column_names: column_names.clone(),
            is_snapshot: false,
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

        let image3 = ImageEntry {
            uuid: uuid3.to_string(),
            name: "Poi".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string(),
            number_of_rows,
            number_of_columns,
            column_names: column_names.clone(),
            is_snapshot: false,
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

        hard_delete_image(&uuid1);
        hard_delete_image(&uuid2);
        hard_delete_image(&uuid3);

        add_image(&image1).unwrap();
        add_image(&image2).unwrap();
        add_image(&image3).unwrap();

        // list-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        let images = list_images(&context).unwrap();
        assert_eq!(images.len(), 1);

        // list-test project-admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: true.to_string(),
        };
        let images = list_images(&context).unwrap();
        assert_eq!(images.len(), 2);

        // list-test admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: true.to_string(),
            is_project_admin: false.to_string(),
        };
        let images = list_images(&context).unwrap();
        assert_eq!(images.len(), 3);

        // get-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        match get_image(&uuid1, &context) {
            Ok(retrieved_image) => {
                assert_eq!(retrieved_image.uuid, uuid1.to_string());
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
        if get_image(&uuid3, &context).is_ok() {
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
        if delete_image(&uuid3, &context).is_ok() {
            assert_eq!(true, false);
        };

        hard_delete_image(&uuid1);
        hard_delete_image(&uuid2);
        hard_delete_image(&uuid3);
    }
}
