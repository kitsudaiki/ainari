// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use diesel::prelude::*;
use chrono::Utc;
use diesel::connection::SimpleConnection;
use log::{info, debug, error};
use std::error::Error;
use rand::{distr::Alphanumeric, Rng};
use uuid::Uuid;

use crate::database::db_handle;
use crate::api::user_context::UserContext;

use hanami_common::enums;

// Define the schema
table! {
    checkpoints (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        file_path -> Text,
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
#[diesel(table_name = checkpoints)]
pub struct checkpoint {
    pub uuid: String,
    pub name: String,
    pub file_path: String,
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

pub fn init_checkpoint_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    let _ = conn.batch_execute("CREATE TABLE IF NOT EXISTS checkpoints (
        uuid VARCHAR(40) PRIMARY KEY,
        name VARCHAR(256),
        file_path TEXT,
        owner_id VARCHAR(256),
        project_id VARCHAR(256),
        status VARCHAR(10),
        created_at VARCHAR(64),
        created_by VARCHAR(256),
        updated_at VARCHAR(64),
        updated_by VARCHAR(256),
        deleted_at VARCHAR(64),
        deleted_by VARCHAR(256)
    );")?;

    Ok(())
}

pub fn add_new_checkpoint(checkpoint_uuid: &Uuid, checkpoint_name: &String, file_path: &String, context: &UserContext) -> QueryResult<usize> {
    let checkpoint = checkpoint{
        uuid: checkpoint_uuid.to_string().clone(),
        name: checkpoint_name.clone(),
        file_path: file_path.clone(),
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

    add_checkpoint(&checkpoint)
}

pub fn add_checkpoint(checkpoint: &checkpoint) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::checkpoints::dsl::*;

    diesel::insert_into(checkpoints).values(checkpoint).execute(&mut *conn)
}

pub fn get_checkpoint(checkpoint_uuid: &Uuid, context: &UserContext) -> Result<checkpoint, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::checkpoints::dsl::*;
    
    let mut query = checkpoints
        .filter(uuid.eq(checkpoint_uuid.to_string()).and(status.eq("ACTIVE")))
        .into_boxed();

    if context.is_admin == false {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin == false {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        } 
    }

    match query
        .select(checkpoint::as_select())
        .first::<checkpoint>(&mut *conn)
    {
        Ok(checkpoint) => Ok(checkpoint),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            error!("Database-error: {}", e);
            Err(enums::DbError::InternalError)
        }
    }
}

pub fn list_checkpoints(context: &UserContext) -> QueryResult<Vec<checkpoint>> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::checkpoints::dsl::*;
    
    let mut query = checkpoints
        .filter(status.eq("ACTIVE"))
        .into_boxed();

    if context.is_admin == false {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin == false {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    query.select(checkpoint::as_select()).load(&mut *conn)
}

pub fn delete_checkpoint(checkpoint_uuid: &Uuid, context: &UserContext) -> Result<(), enums::DbError> {
    get_checkpoint(&checkpoint_uuid, &context)?;

    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::checkpoints::dsl::*;

    match diesel::update(checkpoints.filter(uuid.eq(checkpoint_uuid.to_string())))
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
            error!("Database-error: {}", e);
            Err(enums::DbError::InternalError)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hard_delete_checkpoint(checkpoint_uuid: &Uuid) {
        use self::checkpoints::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().unwrap();
        let _ = diesel::delete(checkpoints.filter(uuid.eq(checkpoint_uuid.to_string()))).execute(&mut *conn);
    }
    
    #[test]
    fn test_add_get_checkpoint() {
        let _ = init_checkpoint_table();
        let uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false,
            is_project_admin: false,
        };

        let checkpoint: checkpoint = checkpoint {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            file_path: "/tmp/bla".to_string(),
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

        hard_delete_checkpoint(&uuid1);

        add_checkpoint(&checkpoint).unwrap();
        match get_checkpoint(&uuid1, &context) {
            Ok(retrieved_checkpoint) => {
                assert_eq!(retrieved_checkpoint.uuid, checkpoint.uuid);
                assert_eq!(retrieved_checkpoint.name, checkpoint.name);
                assert_eq!(retrieved_checkpoint.file_path, checkpoint.file_path);
                assert_eq!(retrieved_checkpoint.status, checkpoint.status);
                assert_eq!(retrieved_checkpoint.created_by, checkpoint.created_by);
                assert_eq!(retrieved_checkpoint.updated_by, checkpoint.updated_by);
                assert_eq!(retrieved_checkpoint.deleted_at, checkpoint.deleted_at);
                assert_eq!(retrieved_checkpoint.deleted_by, checkpoint.deleted_by);
            },
            Err(_) => {}
        };

        let _ = hard_delete_checkpoint(&uuid1);
    }

    #[test]
    fn test_list_checkpoints() {
        let _ = init_checkpoint_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false,
            is_project_admin: false,
        };

        let checkpoint1 = checkpoint {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            file_path: "/tmp/bla".to_string(),
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
        
        let checkpoint2 = checkpoint {
            uuid: uuid2.to_string(),
            name: "Bob".to_string(),
            file_path: "/tmp/bla".to_string(),
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
        
        hard_delete_checkpoint(&uuid1);
        hard_delete_checkpoint(&uuid2);

        add_checkpoint(&checkpoint1).unwrap();
        add_checkpoint(&checkpoint2).unwrap();
        let checkpoints = list_checkpoints(&context).unwrap();
        assert_eq!(checkpoints.len(), 1);
        let _ = hard_delete_checkpoint(&uuid1);
        let _ = hard_delete_checkpoint(&uuid2);
    }

    #[test]
    fn test_delete_checkpoint() {
        let _ = init_checkpoint_table();
        let uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false,
            is_project_admin: false,
        };

        let checkpoint = checkpoint {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            file_path: "/tmp/bla".to_string(),
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

        hard_delete_checkpoint(&uuid1);

        add_checkpoint(&checkpoint).unwrap();
        let _ = delete_checkpoint(&uuid1, &context);
        let result = get_checkpoint(&uuid1, &context);
        assert!(result.is_err());
    }


    #[test]
    fn test_checkpoints_permissions() {
        let _ = init_checkpoint_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();

        let checkpoint1 = checkpoint {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            file_path: "/tmp/bla".to_string(),
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
        
        let checkpoint2 = checkpoint {
            uuid: uuid2.to_string(),
            name: "Bob".to_string(),
            file_path: "/tmp/bla".to_string(),
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
                
        let checkpoint3 = checkpoint {
            uuid: uuid3.to_string(),
            name: "Poi".to_string(),
            file_path: "/tmp/bla".to_string(),
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
        
        hard_delete_checkpoint(&uuid1);
        hard_delete_checkpoint(&uuid2);
        hard_delete_checkpoint(&uuid3);

        add_checkpoint(&checkpoint1).unwrap();
        add_checkpoint(&checkpoint2).unwrap();
        add_checkpoint(&checkpoint3).unwrap();

        // list-test normal user
        let context = UserContext {
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false,
            is_project_admin: false,
        };
        let checkpoints = list_checkpoints(&context).unwrap();
        assert_eq!(checkpoints.len(), 1);

        // list-test project-admin
        let context = UserContext {
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false,
            is_project_admin: true,
        };
        let checkpoints = list_checkpoints(&context).unwrap();
        assert_eq!(checkpoints.len(), 2);

        // list-test admin
        let context = UserContext {
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: true,
            is_project_admin: false,
        };
        let checkpoints = list_checkpoints(&context).unwrap();
        assert_eq!(checkpoints.len(), 3);

        // get-test normal user
        let context = UserContext {
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false,
            is_project_admin: false,
        };
        match get_checkpoint(&uuid1, &context) {
            Ok(retrieved_checkpoint) => {
                assert_eq!(retrieved_checkpoint.uuid, uuid1.to_string());
            },
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        // get-test normal user false uuid
        let context = UserContext {
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false,
            is_project_admin: false,
        };
        match get_checkpoint(&uuid3, &context) {
            Ok(_) => {
                assert_eq!(true, false);
            },
            Err(_) => {}
        };
        
        // delete-test normal user false uuid
        let context = UserContext {
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false,
            is_project_admin: false,
        };
        match delete_checkpoint(&uuid3, &context) {
            Ok(_) => {
                assert_eq!(true, false);
            },
            Err(_) => {}
        };

        let _ = hard_delete_checkpoint(&uuid1);
        let _ = hard_delete_checkpoint(&uuid2);
        let _ = hard_delete_checkpoint(&uuid3);
    }
}
