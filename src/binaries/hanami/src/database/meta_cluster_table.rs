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

use chrono::Utc;
use diesel::connection::SimpleConnection;
use diesel::dsl::count_star;
use diesel::prelude::*;
use std::error::Error;
use uuid::Uuid;

use crate::database::db_handle;

use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;

// Define the schema
table! {
    meta_clusters (uuid) {
        uuid -> Varchar,
        sakura_host_uuid -> Varchar,
        proxy_uuid -> Varchar,
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
#[diesel(table_name = meta_clusters)]
pub struct MetaClusterEntry {
    pub uuid: String,
    pub sakura_host_uuid: String,
    pub proxy_uuid: String,
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

pub fn init_meta_cluster_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS meta_clusters (
        uuid VARCHAR(40) PRIMARY KEY,
        sakura_host_uuid VARCHAR(40),
        proxy_uuid VARCHAR(40),
        owner_id VARCHAR(256),
        project_id VARCHAR(256),
        status VARCHAR(10),
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

pub fn add_new_meta_cluster(
    meta_cluster_uuid: &Uuid,
    sakura_host_uuid: &Uuid,
    proxy_uuid: &Uuid,
    context: &UserContext,
) -> QueryResult<usize> {
    let meta_cluster = MetaClusterEntry {
        uuid: meta_cluster_uuid.to_string().clone(),
        sakura_host_uuid: sakura_host_uuid.to_string().clone(),
        proxy_uuid: proxy_uuid.to_string().clone(),
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

    add_meta_cluster(&meta_cluster)
}

pub fn add_meta_cluster(meta_cluster: &MetaClusterEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::meta_clusters::dsl::*;
    diesel::insert_into(meta_clusters)
        .values(meta_cluster)
        .execute(&mut *conn)
}

pub fn get_meta_cluster(
    meta_cluster_uuid: &Uuid,
    context: &UserContext,
) -> Result<MetaClusterEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::meta_clusters::dsl::*;

    let mut query = meta_clusters
        .filter(
            uuid.eq(meta_cluster_uuid.to_string())
                .and(status.eq("ACTIVE")),
        )
        .into_boxed();

    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    match query
        .select(MetaClusterEntry::as_select())
        .first::<MetaClusterEntry>(&mut *conn)
    {
        Ok(meta_cluster) => Ok(meta_cluster),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

#[allow(dead_code)]
pub fn list_meta_clusters(context: &UserContext) -> QueryResult<Vec<MetaClusterEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::meta_clusters::dsl::*;

    let mut query = meta_clusters.filter(status.eq("ACTIVE")).into_boxed();

    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    query.select(MetaClusterEntry::as_select()).load(&mut *conn)
}

pub fn count_meta_clusters(context: &UserContext) -> QueryResult<i64> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::meta_clusters::dsl::*;

    let mut query = meta_clusters.filter(status.eq("ACTIVE")).into_boxed();

    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    query.select(count_star()).first::<i64>(&mut *conn)
}

pub fn delete_meta_cluster(
    meta_cluster_uuid: &Uuid,
    context: &UserContext,
) -> Result<(), enums::DbError> {
    get_meta_cluster(meta_cluster_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::meta_clusters::dsl::*;
    match diesel::update(meta_clusters.filter(uuid.eq(meta_cluster_uuid.to_string())))
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

#[allow(dead_code)]
pub fn delete_all_meta_cluster() -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::meta_clusters::dsl::*;
    match diesel::update(meta_clusters.filter(status.eq("ACTIVE")))
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

    fn hard_delete_meta_cluster(meta_cluster_uuid: &Uuid) {
        use self::meta_clusters::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(meta_clusters.filter(uuid.eq(meta_cluster_uuid.to_string())))
            .execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_add_get_meta_cluster() {
        let _ = init_meta_cluster_table();
        let uuid1 = Uuid::new_v4();
        let sakura_host_uuid1 = Uuid::new_v4();
        let proxy_uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let meta_cluster = MetaClusterEntry {
            uuid: uuid1.to_string(),
            sakura_host_uuid: sakura_host_uuid1.to_string(),
            proxy_uuid: proxy_uuid1.to_string(),
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

        hard_delete_meta_cluster(&uuid1);

        add_meta_cluster(&meta_cluster).unwrap();
        match get_meta_cluster(&uuid1, &context) {
            Ok(retrieved_meta_cluster) => {
                assert_eq!(retrieved_meta_cluster.uuid, meta_cluster.uuid);
                assert_eq!(retrieved_meta_cluster.proxy_uuid, meta_cluster.proxy_uuid);
                assert_eq!(
                    retrieved_meta_cluster.sakura_host_uuid,
                    meta_cluster.sakura_host_uuid
                );
                assert_eq!(retrieved_meta_cluster.owner_id, meta_cluster.owner_id);
                assert_eq!(retrieved_meta_cluster.project_id, meta_cluster.project_id);
                assert_eq!(retrieved_meta_cluster.status, meta_cluster.status);
                assert_eq!(retrieved_meta_cluster.created_by, meta_cluster.created_by);
                assert_eq!(retrieved_meta_cluster.updated_by, meta_cluster.updated_by);
                assert_eq!(retrieved_meta_cluster.deleted_at, meta_cluster.deleted_at);
                assert_eq!(retrieved_meta_cluster.deleted_by, meta_cluster.deleted_by);
            }
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        hard_delete_meta_cluster(&uuid1);
    }

    #[test]
    #[serial]
    fn test_list_meta_clusters() {
        let _ = init_meta_cluster_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let sakura_host_uuid1 = Uuid::new_v4();
        let proxy_uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let meta_cluster1 = MetaClusterEntry {
            uuid: uuid1.to_string(),
            sakura_host_uuid: sakura_host_uuid1.to_string(),
            proxy_uuid: proxy_uuid1.to_string(),
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

        let meta_cluster2 = MetaClusterEntry {
            uuid: uuid2.to_string(),
            sakura_host_uuid: sakura_host_uuid1.to_string(),
            proxy_uuid: proxy_uuid1.to_string(),
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

        hard_delete_meta_cluster(&uuid1);
        hard_delete_meta_cluster(&uuid2);

        add_meta_cluster(&meta_cluster1).unwrap();
        add_meta_cluster(&meta_cluster2).unwrap();
        let meta_clusters = list_meta_clusters(&context).unwrap();
        assert_eq!(meta_clusters.len(), 1);
        hard_delete_meta_cluster(&uuid1);
        hard_delete_meta_cluster(&uuid2);
    }

    #[test]
    #[serial]
    fn test_delete_meta_cluster() {
        let _ = init_meta_cluster_table();
        let uuid1 = Uuid::new_v4();
        let sakura_host_uuid1 = Uuid::new_v4();
        let proxy_uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let meta_cluster = MetaClusterEntry {
            uuid: uuid1.to_string(),
            sakura_host_uuid: sakura_host_uuid1.to_string(),
            proxy_uuid: proxy_uuid1.to_string(),
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

        hard_delete_meta_cluster(&uuid1);

        add_meta_cluster(&meta_cluster).unwrap();
        let _ = delete_meta_cluster(&uuid1, &context);
        let result = get_meta_cluster(&uuid1, &context);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_count_meta_clusters() {
        let _ = init_meta_cluster_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let sakura_host_uuid1 = Uuid::new_v4();
        let proxy_uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let meta_cluster1 = MetaClusterEntry {
            uuid: uuid1.to_string(),
            sakura_host_uuid: sakura_host_uuid1.to_string(),
            proxy_uuid: proxy_uuid1.to_string(),
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

        let meta_cluster2 = MetaClusterEntry {
            uuid: uuid2.to_string(),
            sakura_host_uuid: sakura_host_uuid1.to_string(),
            proxy_uuid: proxy_uuid1.to_string(),
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

        let meta_cluster3 = MetaClusterEntry {
            uuid: uuid3.to_string(),
            sakura_host_uuid: sakura_host_uuid1.to_string(),
            proxy_uuid: proxy_uuid1.to_string(),
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

        hard_delete_meta_cluster(&uuid1);
        hard_delete_meta_cluster(&uuid2);
        hard_delete_meta_cluster(&uuid3);

        add_meta_cluster(&meta_cluster1).unwrap();
        add_meta_cluster(&meta_cluster2).unwrap();
        add_meta_cluster(&meta_cluster3).unwrap();

        let number = count_meta_clusters(&context).unwrap();
        assert_eq!(number, 3);

        hard_delete_meta_cluster(&uuid1);
        hard_delete_meta_cluster(&uuid2);
        hard_delete_meta_cluster(&uuid3);
    }

    #[test]
    #[serial]
    fn test_meta_clusters_permissions() {
        let _ = init_meta_cluster_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let sakura_host_uuid1 = Uuid::new_v4();
        let proxy_uuid1 = Uuid::new_v4();

        let meta_cluster1 = MetaClusterEntry {
            uuid: uuid1.to_string(),
            sakura_host_uuid: sakura_host_uuid1.to_string(),
            proxy_uuid: proxy_uuid1.to_string(),
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

        let meta_cluster2 = MetaClusterEntry {
            uuid: uuid2.to_string(),
            sakura_host_uuid: sakura_host_uuid1.to_string(),
            proxy_uuid: proxy_uuid1.to_string(),
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

        let meta_cluster3 = MetaClusterEntry {
            uuid: uuid3.to_string(),
            sakura_host_uuid: sakura_host_uuid1.to_string(),
            proxy_uuid: proxy_uuid1.to_string(),
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

        hard_delete_meta_cluster(&uuid1);
        hard_delete_meta_cluster(&uuid2);
        hard_delete_meta_cluster(&uuid3);

        add_meta_cluster(&meta_cluster1).unwrap();
        add_meta_cluster(&meta_cluster2).unwrap();
        add_meta_cluster(&meta_cluster3).unwrap();

        // list-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        let meta_clusters = list_meta_clusters(&context).unwrap();
        assert_eq!(meta_clusters.len(), 1);

        // list-test project-admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: true.to_string(),
        };
        let meta_clusters = list_meta_clusters(&context).unwrap();
        assert_eq!(meta_clusters.len(), 2);

        // list-test admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: true.to_string(),
            is_project_admin: false.to_string(),
        };
        let meta_clusters = list_meta_clusters(&context).unwrap();
        assert_eq!(meta_clusters.len(), 3);

        // get-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        match get_meta_cluster(&uuid1, &context) {
            Ok(retrieved_meta_cluster) => {
                assert_eq!(retrieved_meta_cluster.uuid, uuid1.to_string());
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
        if get_meta_cluster(&uuid3, &context).is_ok() {
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
        if delete_meta_cluster(&uuid3, &context).is_ok() {
            assert_eq!(true, false);
        };

        hard_delete_meta_cluster(&uuid1);
        hard_delete_meta_cluster(&uuid2);
        hard_delete_meta_cluster(&uuid3);
    }
}
