// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimim.moe>

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
use crate::common::enums;

// Define the schema
table! {
    datasets (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        file_path -> Text,
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
pub struct dataset {
    pub uuid: String,
    pub name: String,
    pub file_path: String,
    pub status: String,
    pub created_at: String,
    pub created_by: String,
    pub updated_at: String,
    pub updated_by: String,
    pub deleted_at: Option<String>,
    pub deleted_by: Option<String>,
}

pub fn init_dataset_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    let _ = conn.batch_execute("CREATE TABLE IF NOT EXISTS datasets (
        uuid VARCHAR(40) PRIMARY KEY,
        name VARCHAR(256),
        file_path TEXT,
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

pub fn add_new_dataset(dataset_uuid: &String, dataset_name: &String, file_path: &String, creator_id: &String) -> QueryResult<usize> {
    let dataset = dataset{
        uuid: dataset_uuid.clone(),
        name: dataset_name.clone(),
        file_path: file_path.clone(),
        status: "".to_string(),
        created_at: "".to_string(),
        created_by: creator_id.clone(),
        updated_at: "".to_string(),
        updated_by: creator_id.clone(),
        deleted_at: None,
        deleted_by: None,
    };

    add_dataset(&dataset)
}

pub fn add_dataset(dataset: &dataset) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::datasets::dsl::*;

    let mut new_dataset = dataset.clone();
    new_dataset.created_at = Utc::now().to_rfc3339();
    new_dataset.updated_at = Utc::now().to_rfc3339();
    new_dataset.status = "ACTIVE".to_string();

    diesel::insert_into(datasets).values(new_dataset).execute(&mut *conn)
}

pub fn get_dataset(dataset_uuid: &String) -> Result<dataset, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::datasets::dsl::*;
    match datasets
        .filter(uuid.eq(dataset_uuid).and(status.eq("ACTIVE")))
        .select(dataset::as_select())
        .first::<dataset>(&mut *conn)
    {
        Ok(dataset) => Ok(dataset),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            error!("Database-error: {}", e);
            Err(enums::DbError::InternalError)
        }
    }
}

pub fn list_datasets() -> QueryResult<Vec<dataset>> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::datasets::dsl::*;
    datasets.filter(status.eq("ACTIVE")).select(dataset::as_select()).load(&mut *conn)
}

pub fn delete_dataset(dataset_uuid: &String) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::datasets::dsl::*;
    match diesel::update(datasets.filter(uuid.eq(dataset_uuid)))
        .set(status.eq("DELETED"))
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

    fn hard_delete_dataset(dataset_uuid: &String) {
        use self::datasets::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().unwrap();
        let _ = diesel::delete(datasets.filter(uuid.eq(dataset_uuid))).execute(&mut *conn);
    }
    
    #[test]
    fn test_add_get_dataset() {
        let _ = init_dataset_table();
        let dataset: dataset = dataset {
            uuid: Uuid::new_v4().to_string(),
            name: "Alice".to_string(),
            file_path: "/tmp/bla".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_dataset(&dataset.uuid);

        add_dataset(&dataset).unwrap();
        match get_dataset(&dataset.uuid) {
            Ok(retrieved_dataset) => {
                assert_eq!(retrieved_dataset.uuid, dataset.uuid);
                assert_eq!(retrieved_dataset.name, dataset.name);
                assert_eq!(retrieved_dataset.file_path, dataset.file_path);
                assert_eq!(retrieved_dataset.status, dataset.status);
                assert_eq!(retrieved_dataset.created_by, dataset.created_by);
                assert_eq!(retrieved_dataset.updated_by, dataset.updated_by);
                assert_eq!(retrieved_dataset.deleted_at, dataset.deleted_at);
                assert_eq!(retrieved_dataset.deleted_by, dataset.deleted_by);
            },
            Err(_) => {}
        };

        let _ = delete_dataset(&dataset.uuid);
    }

    #[test]
    fn test_list_datasets() {
        let _ = init_dataset_table();
        let dataset1 = dataset {
            uuid: Uuid::new_v4().to_string(),
            name: "Alice".to_string(),
            file_path: "/tmp/bla".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };
        
        let dataset2 = dataset {
            uuid: Uuid::new_v4().to_string(),
            name: "Bob".to_string(),
            file_path: "/tmp/bla".to_string(),
            status: "DELETED".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };
        
        hard_delete_dataset(&dataset1.uuid);
        hard_delete_dataset(&dataset2.uuid);

        add_dataset(&dataset1).unwrap();
        add_dataset(&dataset2).unwrap();
        let datasets = list_datasets().unwrap();
        assert_eq!(datasets.len(), 2);
        let _ = delete_dataset(&dataset1.uuid);
        let _ = delete_dataset(&dataset2.uuid);
    }

    #[test]
    fn test_delete_dataset() {
        let _ = init_dataset_table();
        let dataset = dataset {
            uuid: Uuid::new_v4().to_string(),
            name: "Alice".to_string(),
            file_path: "/tmp/bla".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_dataset(&dataset.uuid);

        add_dataset(&dataset).unwrap();
        let _ = delete_dataset(&dataset.uuid);
        let result = get_dataset(&dataset.uuid);
        assert!(result.is_err());
    }
}
