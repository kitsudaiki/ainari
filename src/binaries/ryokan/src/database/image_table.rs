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

use chrono::{DateTime, Utc};
use diesel::dsl::count_star;
use diesel::prelude::*;
use uuid::Uuid;

use crate::database::db_handle;
use crate::database::host_table;
use ainari_api_structs::image_structs::ImageStatus;
use ainari_api_structs::user_context::UserContext;

use ainari_common::database::DbConnection;
use ainari_common::enums;
use ainari_common::objects::*;

// Define the schema
table! {
    images (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        onsen_address -> Varchar,
        file_path -> Text,
        file_type -> Varchar,
        file_size -> BigInt,
        image_status -> Varchar,
        secret_uuid -> Varchar,
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
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    /// Name of the image
    pub name: String,
    /// Address of the Onsen service associated with this image
    pub onsen_address: String,
    /// Path to the file containing the image
    pub file_path: String,
    /// Type of the unencrypted file (e.g. "qcow2"), which was identified before the encryption
    pub file_type: String,
    /// Size of the unencrypted file in bytes
    pub file_size: i64,
    /// State of the file of the image (see `ImageStatus`)
    pub image_status: String,
    /// Secret UUID used for authentication with the image
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub secret_uuid: Uuid,
    /// True, if the image is a snapshot of the root-disk of a virtual_machine
    pub is_snapshot: bool,
    /// ID of the user who owns this image
    pub owner_id: String,
    /// ID of the project this image belongs to
    pub project_id: String,
    /// Status of the image (e.g., "ACTIVE", "DELETED")
    pub status: String,
    /// Timestamp when the image was created
    #[diesel(serialize_as = DbDateTime, deserialize_as = DbDateTime)]
    pub created_at: DateTime<Utc>,
    /// ID of the user who created the image
    pub created_by: String,
    /// Timestamp when the image was last updated
    #[diesel(serialize_as = DbDateTime, deserialize_as = DbDateTime)]
    pub updated_at: DateTime<Utc>,
    /// ID of the user who last updated the image
    pub updated_by: String,
    /// Timestamp when the image was deleted (if applicable)
    #[diesel(serialize_as = DbOptDateTime, deserialize_as = DbOptDateTime)]
    pub deleted_at: Option<DateTime<Utc>>,
    /// ID of the user who deleted the image (if applicable)
    pub deleted_by: Option<String>,
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
/// * `file_type` - The type of the unencrypted file, or "unknown" for a snapshot, whose file
///   doesn't exist yet
/// * `file_size` - The size of the unencrypted file in bytes, or 0, if it is not known yet
/// * `secret_uuid` - The secret UUID for authentication
/// * `is_snapshot` - True, if the image is a snapshot of the root-disk of a virtual_machine
/// * `image_status` - Initial state of the file of the image
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
    file_type: &str,
    file_size: i64,
    secret_uuid: &Uuid,
    is_snapshot: bool,
    image_status: &ImageStatus,
    context: &UserContext,
) -> QueryResult<usize> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::permission_denied_error());
    }

    // Create a new ImageEntry with the provided parameters
    let image = ImageEntry {
        uuid: *image_uuid,
        name: image_name.to_owned(),
        onsen_address: onsen_address.to_owned(),
        file_path: file_path.to_owned(),
        file_type: file_type.to_owned(),
        file_size,
        image_status: image_status.to_string(),
        secret_uuid: *secret_uuid,
        is_snapshot,
        owner_id: context.user_id.clone(),
        project_id: context.project_id.clone(),
        status: "ACTIVE".to_string(),
        created_at: Utc::now(),
        created_by: context.user_id.clone(),
        updated_at: Utc::now(),
        updated_by: context.user_id.clone(),
        deleted_at: None,
        deleted_by: None,
    };

    add_image(image.clone())
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
pub fn add_image(image: ImageEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::images::dsl::*;

    // a backup-target never stores normal images or snapshots. The host-selection already skips
    // them, but the host could have been marked in the meantime, so it is checked again within
    // the same transaction like the marking itself.
    conn.transaction(|conn| {
        if host_table::is_backup_target_address(conn, &image.onsen_address)? {
            return Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::CheckViolation,
                Box::new(format!(
                    "onsen-host '{}' is a backup-target and can not store images",
                    image.onsen_address
                )),
            ));
        }
        diesel::insert_into(images).values(image).execute(conn)
    })
}

/// Counts the active images and snapshots, which are stored on an onsen-host.
///
/// It takes the already locked connection, so it can be used within the transaction of another
/// table.
///
/// # Arguments
/// * `conn` - The locked database-connection
/// * `address` - Address of the onsen-host
///
/// # Returns
/// * `QueryResult<i64>` - The number of active images on the host
pub fn count_images_on_onsen(conn: &mut DbConnection, address: &str) -> QueryResult<i64> {
    use self::images::dsl::*;

    images
        .filter(status.eq("ACTIVE"))
        .filter(onsen_address.eq(address))
        .select(count_star())
        .first::<i64>(conn)
}

/// Checks, if an image with this UUID was ever registered, in any project and also if it is
/// already deleted.
///
/// # Arguments
/// * `image_uuid` - The UUID of the image
///
/// # Returns
/// * `QueryResult<bool>` - True, if the UUID is already used
pub fn is_uuid_taken(image_uuid: &Uuid) -> QueryResult<bool> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::images::dsl::*;

    images
        .filter(uuid.eq(image_uuid.to_string()))
        .select(count_star())
        .first::<i64>(&mut *conn)
        .map(|count| count > 0)
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

/// Counts the number of images of a whole project.
///
/// Unlike `count_images`, the images of all users of the project are counted, because the quota,
/// which is checked with this number, belongs to the project.
///
/// # Arguments
///
/// * `project` - The ID of the project, which is counted
///
/// # Returns
///
/// A QueryResult containing the count of images as an i64
pub fn count_images_of_project(project: &str) -> QueryResult<i64> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::images::dsl::*;

    images
        .filter(status.eq("ACTIVE"))
        .filter(project_id.eq(project))
        .select(count_star())
        .first::<i64>(&mut *conn)
}

/// Sets the state of the file of an image, optionally together with type and size of the file.
///
/// The file of an image doesn't exist yet, when the image is registered, so type and size are
/// only known after the upload or after the root-disk of a virtual_machine was copied. Only an
/// active image is updated, so an image, which was deleted during the upload, stays deleted.
///
/// # Arguments
/// * `image_uuid` - The UUID of the image to update
/// * `new_image_status` - The new state of the file of the image
/// * `file_info` - Type and size in bytes of the unencrypted file, if they are known
/// * `context` - The user context containing authentication information
///
/// # Returns
/// * `Result<(), enums::DbError>` - Success or an error
pub fn update_image_status(
    image_uuid: &Uuid,
    new_image_status: &ImageStatus,
    file_info: Option<(&str, i64)>,
    context: &UserContext,
) -> Result<(), enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    // First verify that the image exists and is accessible to the user
    get_image(image_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::images::dsl::*;

    let target = images.filter(uuid.eq(image_uuid.to_string()).and(status.eq("ACTIVE")));
    let result = match file_info {
        Some((new_file_type, new_file_size)) => diesel::update(target)
            .set((
                image_status.eq(new_image_status.as_str()),
                file_type.eq(new_file_type),
                file_size.eq(new_file_size),
                updated_at.eq(Utc::now().to_rfc3339()),
                updated_by.eq(context.user_id.clone()),
            ))
            .execute(&mut *conn),
        None => diesel::update(target)
            .set((
                image_status.eq(new_image_status.as_str()),
                updated_at.eq(Utc::now().to_rfc3339()),
                updated_by.eq(context.user_id.clone()),
            ))
            .execute(&mut *conn),
    };

    match result {
        // the image was deleted between the check above and the update
        Ok(0) => Err(enums::DbError::NotFound),
        Ok(_) => Ok(()),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
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
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

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
    use ainari_common::enums::ProjectRole;
    use serial_test::serial;

    fn hard_delete_image(image_uuid: &Uuid) {
        use self::images::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(images.filter(uuid.eq(image_uuid.to_string()))).execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_add_get_image() {
        let uuid1 = Uuid::new_v4();
        let onsen_address = "127.0.0.1:1234".to_string();
        let secret_uuid = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let image = ImageEntry {
            uuid: uuid1,
            name: "Alice".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            file_type: "qcow2".to_string(),
            file_size: 4096,
            image_status: ImageStatus::Ready.to_string(),
            secret_uuid,
            is_snapshot: true,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_image(&uuid1);

        add_image(image.clone()).unwrap();
        if let Ok(retrieved_image) = get_image(&uuid1, &context) {
            assert_eq!(retrieved_image.uuid, image.uuid);
            assert_eq!(retrieved_image.name, image.name);
            assert_eq!(retrieved_image.file_path, image.file_path);
            assert_eq!(retrieved_image.file_type, image.file_type);
            assert_eq!(retrieved_image.file_size, image.file_size);
            assert_eq!(retrieved_image.image_status, image.image_status);
            assert_eq!(retrieved_image.secret_uuid, image.secret_uuid);
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
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let onsen_address = "127.0.0.1:1234".to_string();
        let secret_uuid = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let image1 = ImageEntry {
            uuid: uuid1,
            name: "Alice".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            file_type: "qcow2".to_string(),
            file_size: 4096,
            image_status: ImageStatus::Ready.to_string(),
            secret_uuid,
            is_snapshot: false,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let image2 = ImageEntry {
            uuid: uuid2,
            name: "Bob".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            file_type: "qcow2".to_string(),
            file_size: 4096,
            image_status: ImageStatus::Ready.to_string(),
            secret_uuid,
            is_snapshot: false,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "DELETED".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_image(&uuid1);
        hard_delete_image(&uuid2);

        add_image(image1).unwrap();
        add_image(image2).unwrap();
        let images = list_images(&context).unwrap();
        assert_eq!(images.len(), 1);
        hard_delete_image(&uuid1);
        hard_delete_image(&uuid2);
    }

    #[test]
    #[serial]
    fn test_delete_image() {
        let uuid1 = Uuid::new_v4();
        let onsen_address = "127.0.0.1:1234".to_string();
        let secret_uuid = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let image = ImageEntry {
            uuid: uuid1,
            name: "Alice".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            file_type: "qcow2".to_string(),
            file_size: 4096,
            image_status: ImageStatus::Ready.to_string(),
            secret_uuid,
            is_snapshot: false,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_image(&uuid1);
        assert!(!is_uuid_taken(&uuid1).unwrap());

        add_image(image).unwrap();
        assert!(is_uuid_taken(&uuid1).unwrap());
        let _ = delete_image(&uuid1, &context);
        let result = get_image(&uuid1, &context);
        assert!(result.is_err());
        // also a deleted image keeps its UUID
        assert!(is_uuid_taken(&uuid1).unwrap());

        hard_delete_image(&uuid1);
    }

    #[test]
    #[serial]
    fn test_update_image_status() {
        let uuid1 = Uuid::new_v4();
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: "test-project".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        hard_delete_image(&uuid1);

        add_new_image(
            &uuid1,
            "Alice",
            "127.0.0.1:1234",
            "/tmp/bla",
            "unknown",
            0,
            &Uuid::new_v4(),
            true,
            &ImageStatus::Encrypting,
            &context,
        )
        .unwrap();
        let Ok(new_image) = get_image(&uuid1, &context) else {
            panic!("new image not found");
        };
        assert_eq!(new_image.image_status, "ENCRYPTING");
        assert_eq!(new_image.file_type, "unknown");
        assert_eq!(new_image.file_size, 0);

        // the file-info is set together with the status, like sakura does it for a snapshot
        assert!(
            update_image_status(
                &uuid1,
                &ImageStatus::Uploading,
                Some(("qcow2", 1 << 33)),
                &context
            )
            .is_ok()
        );
        // without file-info only the status is changed
        assert!(update_image_status(&uuid1, &ImageStatus::Ready, None, &context).is_ok());

        let Ok(retrieved_image) = get_image(&uuid1, &context) else {
            panic!("updated image not found");
        };
        assert_eq!(retrieved_image.image_status, "READY");
        assert_eq!(retrieved_image.file_type, "qcow2");
        assert_eq!(retrieved_image.file_size, 1 << 33);

        // a deleted image is not updated anymore
        assert!(delete_image(&uuid1, &context).is_ok());
        assert!(update_image_status(&uuid1, &ImageStatus::Ready, None, &context).is_err());

        hard_delete_image(&uuid1);
    }

    #[test]
    #[serial]
    fn test_count_images() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let name = "test-image".to_string();
        let onsen_address = "127.0.0.1:1234".to_string();
        let secret_uuid = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let image1 = ImageEntry {
            uuid: uuid1,
            name: name.clone(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            file_type: "qcow2".to_string(),
            file_size: 4096,
            image_status: ImageStatus::Ready.to_string(),
            secret_uuid,
            is_snapshot: false,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let image2 = ImageEntry {
            uuid: uuid2,
            name: name.clone(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            file_type: "qcow2".to_string(),
            file_size: 4096,
            image_status: ImageStatus::Ready.to_string(),
            secret_uuid,
            is_snapshot: false,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let image3 = ImageEntry {
            uuid: uuid3,
            name: name.clone(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            file_type: "qcow2".to_string(),
            file_size: 4096,
            image_status: ImageStatus::Ready.to_string(),
            secret_uuid,
            is_snapshot: false,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_image(&uuid1);
        hard_delete_image(&uuid2);
        hard_delete_image(&uuid3);

        add_image(image1).unwrap();
        add_image(image2).unwrap();
        add_image(image3).unwrap();

        let number = count_images(&context).unwrap();
        assert_eq!(number, 3);

        hard_delete_image(&uuid1);
        hard_delete_image(&uuid2);
        hard_delete_image(&uuid3);
    }

    #[test]
    #[serial]
    fn test_images_permissions() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let onsen_address = "127.0.0.1:1234".to_string();
        let secret_uuid = Uuid::new_v4();

        let image1 = ImageEntry {
            uuid: uuid1,
            name: "Alice".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            file_type: "qcow2".to_string(),
            file_size: 4096,
            image_status: ImageStatus::Ready.to_string(),
            secret_uuid,
            is_snapshot: false,
            owner_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let image2 = ImageEntry {
            uuid: uuid2,
            name: "Bob".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            file_type: "qcow2".to_string(),
            file_size: 4096,
            image_status: ImageStatus::Ready.to_string(),
            secret_uuid,
            is_snapshot: false,
            owner_id: "test-user-43".to_string(),
            project_id: "test_permissions_1".to_string(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let image3 = ImageEntry {
            uuid: uuid3,
            name: "Poi".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            file_type: "qcow2".to_string(),
            file_size: 4096,
            image_status: ImageStatus::Ready.to_string(),
            secret_uuid,
            is_snapshot: false,
            owner_id: "test-user-44".to_string(),
            project_id: "test_permissions_2".to_string(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_image(&uuid1);
        hard_delete_image(&uuid2);
        hard_delete_image(&uuid3);

        add_image(image1).unwrap();
        add_image(image2).unwrap();
        add_image(image3).unwrap();

        // list-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        let images = list_images(&context).unwrap();
        // members see all entries of their project, also the ones of other users
        assert_eq!(images.len(), 2);

        // list-test project-admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Admin.to_string(),
        };
        let images = list_images(&context).unwrap();
        assert_eq!(images.len(), 2);

        // list-test admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: true.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        let images = list_images(&context).unwrap();
        assert_eq!(images.len(), 3);

        // get-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        match get_image(&uuid1, &context) {
            Ok(retrieved_image) => {
                assert_eq!(retrieved_image.uuid, uuid1);
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
            project_role: ProjectRole::Member.to_string(),
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
            project_role: ProjectRole::Member.to_string(),
        };
        if delete_image(&uuid3, &context).is_ok() {
            assert_eq!(true, false);
        };

        hard_delete_image(&uuid1);
        hard_delete_image(&uuid2);
        hard_delete_image(&uuid3);
    }
}
