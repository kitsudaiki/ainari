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
use std::env;
use std::error::Error;
use rand::{distr::Alphanumeric, Rng};
use uuid::Uuid;

use crate::database::db_handle;
use hanami_common::functions::sha256_hash;
use hanami_common::enums;

// Define the schema
table! {
    clusters (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        template -> Text,
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
#[diesel(table_name = clusters)]
pub struct cluster {
    pub uuid: String,
    pub name: String,
    pub template: String,
    pub status: String,
    pub created_at: String,
    pub created_by: String,
    pub updated_at: String,
    pub updated_by: String,
    pub deleted_at: Option<String>,
    pub deleted_by: Option<String>,
}

pub fn init_cluster_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    let _ = conn.batch_execute("CREATE TABLE IF NOT EXISTS clusters (
        uuid VARCHAR(40) PRIMARY KEY,
        name VARCHAR(256),
        template TEXT,
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

pub fn add_new_cluster(cluster_uuid: &Uuid, cluster_name: &String, cluster_template: &String, creator_id: &String) -> QueryResult<usize> {
    let cluster = cluster{
        uuid: cluster_uuid.to_string().clone(),
        name: cluster_name.clone(),
        template: cluster_template.clone(),
        status: "".to_string(),
        created_at: "".to_string(),
        created_by: creator_id.clone(),
        updated_at: "".to_string(),
        updated_by: creator_id.clone(),
        deleted_at: None,
        deleted_by: None,
    };

    add_cluster(&cluster)
}

pub fn add_cluster(cluster: &cluster) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::clusters::dsl::*;

    let mut new_cluster = cluster.clone();
    new_cluster.created_at = Utc::now().to_rfc3339();
    new_cluster.updated_at = Utc::now().to_rfc3339();
    new_cluster.status = "ACTIVE".to_string();

    diesel::insert_into(clusters).values(new_cluster).execute(&mut *conn)
}

pub fn get_cluster(cluster_uuid: &Uuid) -> Result<cluster, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::clusters::dsl::*;
    match clusters
        .filter(uuid.eq(cluster_uuid.to_string()).and(status.eq("ACTIVE")))
        .select(cluster::as_select())
        .first::<cluster>(&mut *conn)
    {
        Ok(cluster) => Ok(cluster),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            error!("Database-error: {}", e);
            Err(enums::DbError::InternalError)
        }
    }
}

pub fn list_clusters() -> QueryResult<Vec<cluster>> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::clusters::dsl::*;
    clusters.filter(status.eq("ACTIVE")).select(cluster::as_select()).load(&mut *conn)
}

pub fn delete_cluster(cluster_uuid: &Uuid) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::clusters::dsl::*;
    match diesel::update(clusters.filter(uuid.eq(cluster_uuid.to_string())))
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

    fn hard_delete_cluster(cluster_uuid: &Uuid) {
        use self::clusters::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().unwrap();
        let _ = diesel::delete(clusters.filter(uuid.eq(cluster_uuid.to_string()))).execute(&mut *conn);
    }
    
    #[test]
    fn test_add_get_cluster() {
        let _ = init_cluster_table();
        let uuid1 = Uuid::new_v4();

        let cluster: cluster = cluster {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            template: "asdf".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_cluster(&uuid1);

        add_cluster(&cluster).unwrap();
        match get_cluster(&uuid1) {
            Ok(retrieved_cluster) => {
                assert_eq!(retrieved_cluster.uuid, cluster.uuid);
                assert_eq!(retrieved_cluster.name, cluster.name);
                assert_eq!(retrieved_cluster.template, cluster.template);
                assert_eq!(retrieved_cluster.status, cluster.status);
                assert_eq!(retrieved_cluster.created_by, cluster.created_by);
                assert_eq!(retrieved_cluster.updated_by, cluster.updated_by);
                assert_eq!(retrieved_cluster.deleted_at, cluster.deleted_at);
                assert_eq!(retrieved_cluster.deleted_by, cluster.deleted_by);
            },
            Err(_) => {}
        };

        let _ = hard_delete_cluster(&uuid1);
    }

    #[test]
    fn test_list_clusters() {
        let _ = init_cluster_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let cluster1 = cluster {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            template: "asdf".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };
        
        let cluster2 = cluster {
            uuid: uuid2.to_string(),
            name: "Bob".to_string(),
            template: "asdf".to_string(),
            status: "DELETED".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };
        
        hard_delete_cluster(&uuid1);
        hard_delete_cluster(&uuid2);

        add_cluster(&cluster1).unwrap();
        add_cluster(&cluster2).unwrap();
        let clusters = list_clusters().unwrap();
        assert_eq!(clusters.len(), 2);
        let _ = hard_delete_cluster(&uuid1);
        let _ = hard_delete_cluster(&uuid2);
    }

    #[test]
    fn test_delete_cluster() {
        let _ = init_cluster_table();
        let uuid1 = Uuid::new_v4();

        let cluster = cluster {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            template: "asdf".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_cluster(&uuid1);

        add_cluster(&cluster).unwrap();
        let _ = delete_cluster(&uuid1);
        let result = get_cluster(&uuid1);
        assert!(result.is_err());
    }
}
