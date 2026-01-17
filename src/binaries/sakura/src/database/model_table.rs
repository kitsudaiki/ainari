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
use diesel::prelude::*;
use diesel::result::DatabaseErrorKind;
use uuid::Uuid;

use crate::database::db_handle;

use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;

// Define the schema for the models table
table! {
    models (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        inputs -> Text,
        outputs -> Text,
        template -> Text,
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

/// Represents a single entry in the models table
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = models)]
pub struct ModelEntry {
    /// Unique identifier for the model
    pub uuid: String,
    /// Name of the model
    pub name: String,
    /// Serialized JSON string of input specifications
    pub inputs: String,
    /// Serialized JSON string of output specifications
    pub outputs: String,
    /// Template content for the model
    pub template: String,
    /// ID of the user who owns the model
    pub owner_id: String,
    /// ID of the project the model belongs to
    pub project_id: String,
    /// Current status of the model (ACTIVE, DELETED, etc.)
    pub status: String,
    /// Timestamp when the model was created
    pub created_at: String,
    /// ID of the user who created the model
    pub created_by: String,
    /// Timestamp when the model was last updated
    pub updated_at: String,
    /// ID of the user who last updated the model
    pub updated_by: String,
    /// Timestamp when the model was deleted (if applicable)
    pub deleted_at: Option<String>,
    /// ID of the user who deleted the model (if applicable)
    pub deleted_by: Option<String>,
}

/// Initializes the models table in the database if it doesn't already exist
///
/// # Returns
/// * `Ok(())` if the table was created or already exists
/// * An error if there was a problem creating the table
pub fn init_model_table() -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS models (
        uuid VARCHAR(40) PRIMARY KEY,
        name VARCHAR(256),
        inputs TEXT,
        outputs TEXT,
        template TEXT,
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

/// Adds a new model to the database with the provided information
///
/// # Arguments
/// * `model_uuid` - Unique identifier for the new model
/// * `model_name` - Name for the new model
/// * `model_template` - Template content for the new model
/// * `inputs` - Vector of input specifications
/// * `outputs` - Vector of output specifications
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(usize)` with the number of rows inserted on success
/// * `Err` with an appropriate error on failure
pub fn add_new_model(
    model_uuid: &Uuid,
    model_name: &str,
    model_template: &str,
    inputs: &Vec<String>,
    outputs: &Vec<String>,
    context: &UserContext,
) -> QueryResult<usize> {
    // Serialize the input and output vectors to JSON strings
    let inputs_str = match serde_json::to_string(&inputs) {
        Ok(inputs_str) => inputs_str,
        Err(e) => {
            return Err(diesel::result::Error::DatabaseError(
                DatabaseErrorKind::SerializationFailure,
                Box::new(format!("Failed to serialize inputs with error: {e}")),
            ));
        }
    };
    let outputs_str = match serde_json::to_string(&outputs) {
        Ok(outputs_str) => outputs_str,
        Err(e) => {
            return Err(diesel::result::Error::DatabaseError(
                DatabaseErrorKind::SerializationFailure,
                Box::new(format!("Failed to serialize outputs with error: {e}")),
            ));
        }
    };

    // Create the new model entry
    let model = ModelEntry {
        uuid: model_uuid.to_string().clone(),
        name: model_name.to_owned(),
        inputs: inputs_str,
        outputs: outputs_str,
        template: model_template.to_owned(),
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

    // Insert the model into the database
    add_model(&model)
}

/// Adds a model entry to the database
///
/// # Arguments
/// * `model` - The model entry to insert
///
/// # Returns
/// * `Ok(usize)` with the number of rows inserted on success
/// * `Err` with an appropriate error on failure
pub fn add_model(model: &ModelEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::models::dsl::*;
    diesel::insert_into(models)
        .values(model)
        .execute(&mut *conn)
}

/// Retrieves a specific model from the database
///
/// # Arguments
/// * `model_uuid` - Unique identifier of the model to retrieve
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(ModelEntry)` with the model on success
/// * `Err(enums::DbError)` with an appropriate error on failure
pub fn get_model(model_uuid: &Uuid, context: &UserContext) -> Result<ModelEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::models::dsl::*;

    // Build the query with appropriate filters based on user permissions
    let mut query = models
        .filter(uuid.eq(model_uuid.to_string()).and(status.eq("ACTIVE")))
        .into_boxed();

    // Apply project and ownership filters for non-admin users
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    // Execute the query and return the result
    match query
        .select(ModelEntry::as_select())
        .first::<ModelEntry>(&mut *conn)
    {
        Ok(model) => Ok(model),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Lists all deleted models from the database
///
/// # Returns
/// * `Ok(Vec<ModelEntry>)` with the list of deleted models on success
/// * `Err` with an appropriate error on failure
pub fn list_deleted_models() -> QueryResult<Vec<ModelEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::models::dsl::*;

    // Create a query to find all models with "DELETED" status
    let query = models.filter(status.eq("DELETED")).into_boxed();

    // Execute the query and return the results
    query.select(ModelEntry::as_select()).load(&mut *conn)
}

/// Lists all active models from the database, applying appropriate filters based on user permissions
///
/// # Arguments
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(Vec<ModelEntry>)` with the list of models on success
/// * `Err` with an appropriate error on failure
pub fn list_models(context: &UserContext) -> QueryResult<Vec<ModelEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::models::dsl::*;

    // Build the query with appropriate filters based on user permissions
    let mut query = models.filter(status.eq("ACTIVE")).into_boxed();

    // Apply project and ownership filters for non-admin users
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    // Execute the query and return the results
    query.select(ModelEntry::as_select()).load(&mut *conn)
}

/// Marks a specific model as deleted in the database
///
/// # Arguments
/// * `model_uuid` - Unique identifier of the model to delete
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(enums::DbError)` with an appropriate error on failure
pub fn delete_model(model_uuid: &Uuid, context: &UserContext) -> Result<(), enums::DbError> {
    // First verify that the model exists and the user has permission to delete it
    get_model(model_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::models::dsl::*;

    // Update the model's status to "DELETED" and set the deletion timestamp and user
    match diesel::update(models.filter(uuid.eq(model_uuid.to_string())))
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

/// Marks all active models as deleted in the database
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(enums::DbError)` with an appropriate error on failure
pub fn delete_all_model() -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::models::dsl::*;

    // Update all active models to have "DELETED" status with a system user as the deleter
    match diesel::update(models.filter(status.eq("ACTIVE")))
        .set((
            status.eq("DELETED"),
            deleted_at.eq(Utc::now().to_rfc3339()),
            deleted_by.eq("AINARI_START"),
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

    fn hard_delete_model(model_uuid: &Uuid) {
        use self::models::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(models.filter(uuid.eq(model_uuid.to_string()))).execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_add_get_model() {
        let _ = init_model_table();
        let uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        let inputs = "[\"input\"]".to_string();
        let outputs = "[\"output\"]".to_string();

        let model = ModelEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            inputs: inputs.clone(),
            outputs: outputs.clone(),
            template: "asdf".to_string(),
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

        hard_delete_model(&uuid1);

        add_model(&model).unwrap();
        match get_model(&uuid1, &context) {
            Ok(retrieved_model) => {
                assert_eq!(retrieved_model.uuid, model.uuid);
                assert_eq!(retrieved_model.name, model.name);
                assert_eq!(retrieved_model.inputs, inputs);
                assert_eq!(retrieved_model.outputs, outputs);
                assert_eq!(retrieved_model.template, model.template);
                assert_eq!(retrieved_model.owner_id, model.owner_id);
                assert_eq!(retrieved_model.project_id, model.project_id);
                assert_eq!(retrieved_model.status, model.status);
                assert_eq!(retrieved_model.created_by, model.created_by);
                assert_eq!(retrieved_model.updated_by, model.updated_by);
                assert_eq!(retrieved_model.deleted_at, model.deleted_at);
                assert_eq!(retrieved_model.deleted_by, model.deleted_by);
            }
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        hard_delete_model(&uuid1);
    }

    #[test]
    #[serial]
    fn test_list_models() {
        let _ = init_model_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        let inputs = "[\"input\"]".to_string();
        let outputs = "[\"output\"]".to_string();

        let model1 = ModelEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            inputs: inputs.clone(),
            outputs: outputs.clone(),
            template: "asdf".to_string(),
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

        let model2 = ModelEntry {
            uuid: uuid2.to_string(),
            name: "Bob".to_string(),
            inputs: inputs.clone(),
            outputs: outputs.clone(),
            template: "asdf".to_string(),
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

        hard_delete_model(&uuid1);
        hard_delete_model(&uuid2);

        add_model(&model1).unwrap();
        add_model(&model2).unwrap();
        let models = list_models(&context).unwrap();
        assert_eq!(models.len(), 1);
        hard_delete_model(&uuid1);
        hard_delete_model(&uuid2);
    }

    #[test]
    #[serial]
    fn test_delete_model() {
        let _ = init_model_table();
        let uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        let inputs = "[\"input\"]".to_string();
        let outputs = "[\"output\"]".to_string();

        let model = ModelEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            inputs: inputs.clone(),
            outputs: outputs.clone(),
            template: "asdf".to_string(),
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

        hard_delete_model(&uuid1);

        add_model(&model).unwrap();
        let _ = delete_model(&uuid1, &context);
        let result = get_model(&uuid1, &context);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_models_permissions() {
        let _ = init_model_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let inputs = "[\"input\"]".to_string();
        let outputs = "[\"output\"]".to_string();

        let model1 = ModelEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            inputs: inputs.clone(),
            outputs: outputs.clone(),
            template: "asdf".to_string(),
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

        let model2 = ModelEntry {
            uuid: uuid2.to_string(),
            name: "Bob".to_string(),
            inputs: inputs.clone(),
            outputs: outputs.clone(),
            template: "asdf".to_string(),
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

        let model3 = ModelEntry {
            uuid: uuid3.to_string(),
            name: "Poi".to_string(),
            inputs: inputs.clone(),
            outputs: outputs.clone(),
            template: "asdf".to_string(),
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

        hard_delete_model(&uuid1);
        hard_delete_model(&uuid2);
        hard_delete_model(&uuid3);

        add_model(&model1).unwrap();
        add_model(&model2).unwrap();
        add_model(&model3).unwrap();

        // list-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        let models = list_models(&context).unwrap();
        assert_eq!(models.len(), 1);

        // list-test project-admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: true.to_string(),
        };
        let models = list_models(&context).unwrap();
        assert_eq!(models.len(), 2);

        // list-test admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: true.to_string(),
            is_project_admin: false.to_string(),
        };
        let models = list_models(&context).unwrap();
        assert_eq!(models.len(), 3);

        // get-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        match get_model(&uuid1, &context) {
            Ok(retrieved_model) => {
                assert_eq!(retrieved_model.uuid, uuid1.to_string());
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
        if get_model(&uuid3, &context).is_ok() {
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
        if delete_model(&uuid3, &context).is_ok() {
            assert_eq!(true, false);
        };

        hard_delete_model(&uuid1);
        hard_delete_model(&uuid2);
        hard_delete_model(&uuid3);
    }
}
