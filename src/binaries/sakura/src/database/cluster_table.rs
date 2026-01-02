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

// Define the schema
table! {
    clusters (uuid) {
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

#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = clusters)]
pub struct ClusterEntry {
    pub uuid: String,
    pub name: String,
    pub inputs: String,
    pub outputs: String,
    pub template: String,
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

pub fn init_cluster_table() -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS clusters (
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

pub fn add_new_cluster(
    cluster_uuid: &Uuid,
    cluster_name: &str,
    cluster_template: &str,
    inputs: &Vec<String>,
    outputs: &Vec<String>,
    context: &UserContext,
) -> QueryResult<usize> {
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

    let cluster = ClusterEntry {
        uuid: cluster_uuid.to_string().clone(),
        name: cluster_name.to_owned(),
        inputs: inputs_str,
        outputs: outputs_str,
        template: cluster_template.to_owned(),
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

    add_cluster(&cluster)
}

pub fn add_cluster(cluster: &ClusterEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::clusters::dsl::*;
    diesel::insert_into(clusters)
        .values(cluster)
        .execute(&mut *conn)
}

pub fn get_cluster(
    cluster_uuid: &Uuid,
    context: &UserContext,
) -> Result<ClusterEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::clusters::dsl::*;

    let mut query = clusters
        .filter(uuid.eq(cluster_uuid.to_string()).and(status.eq("ACTIVE")))
        .into_boxed();

    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    match query
        .select(ClusterEntry::as_select())
        .first::<ClusterEntry>(&mut *conn)
    {
        Ok(cluster) => Ok(cluster),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

pub fn list_clusters(context: &UserContext) -> QueryResult<Vec<ClusterEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::clusters::dsl::*;

    let mut query = clusters.filter(status.eq("ACTIVE")).into_boxed();

    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    query.select(ClusterEntry::as_select()).load(&mut *conn)
}

pub fn delete_cluster(cluster_uuid: &Uuid, context: &UserContext) -> Result<(), enums::DbError> {
    get_cluster(cluster_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::clusters::dsl::*;
    match diesel::update(clusters.filter(uuid.eq(cluster_uuid.to_string())))
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

pub fn delete_all_cluster() -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::clusters::dsl::*;
    match diesel::update(clusters.filter(status.eq("ACTIVE")))
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

    fn hard_delete_cluster(cluster_uuid: &Uuid) {
        use self::clusters::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ =
            diesel::delete(clusters.filter(uuid.eq(cluster_uuid.to_string()))).execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_add_get_cluster() {
        let _ = init_cluster_table();
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

        let cluster = ClusterEntry {
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

        hard_delete_cluster(&uuid1);

        add_cluster(&cluster).unwrap();
        match get_cluster(&uuid1, &context) {
            Ok(retrieved_cluster) => {
                assert_eq!(retrieved_cluster.uuid, cluster.uuid);
                assert_eq!(retrieved_cluster.name, cluster.name);
                assert_eq!(retrieved_cluster.inputs, inputs);
                assert_eq!(retrieved_cluster.outputs, outputs);
                assert_eq!(retrieved_cluster.template, cluster.template);
                assert_eq!(retrieved_cluster.owner_id, cluster.owner_id);
                assert_eq!(retrieved_cluster.project_id, cluster.project_id);
                assert_eq!(retrieved_cluster.status, cluster.status);
                assert_eq!(retrieved_cluster.created_by, cluster.created_by);
                assert_eq!(retrieved_cluster.updated_by, cluster.updated_by);
                assert_eq!(retrieved_cluster.deleted_at, cluster.deleted_at);
                assert_eq!(retrieved_cluster.deleted_by, cluster.deleted_by);
            }
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        hard_delete_cluster(&uuid1);
    }

    #[test]
    #[serial]
    fn test_list_clusters() {
        let _ = init_cluster_table();
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

        let cluster1 = ClusterEntry {
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

        let cluster2 = ClusterEntry {
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

        hard_delete_cluster(&uuid1);
        hard_delete_cluster(&uuid2);

        add_cluster(&cluster1).unwrap();
        add_cluster(&cluster2).unwrap();
        let clusters = list_clusters(&context).unwrap();
        assert_eq!(clusters.len(), 1);
        hard_delete_cluster(&uuid1);
        hard_delete_cluster(&uuid2);
    }

    #[test]
    #[serial]
    fn test_delete_cluster() {
        let _ = init_cluster_table();
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

        let cluster = ClusterEntry {
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

        hard_delete_cluster(&uuid1);

        add_cluster(&cluster).unwrap();
        let _ = delete_cluster(&uuid1, &context);
        let result = get_cluster(&uuid1, &context);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_clusters_permissions() {
        let _ = init_cluster_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let inputs = "[\"input\"]".to_string();
        let outputs = "[\"output\"]".to_string();

        let cluster1 = ClusterEntry {
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

        let cluster2 = ClusterEntry {
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

        let cluster3 = ClusterEntry {
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

        hard_delete_cluster(&uuid1);
        hard_delete_cluster(&uuid2);
        hard_delete_cluster(&uuid3);

        add_cluster(&cluster1).unwrap();
        add_cluster(&cluster2).unwrap();
        add_cluster(&cluster3).unwrap();

        // list-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        let clusters = list_clusters(&context).unwrap();
        assert_eq!(clusters.len(), 1);

        // list-test project-admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: true.to_string(),
        };
        let clusters = list_clusters(&context).unwrap();
        assert_eq!(clusters.len(), 2);

        // list-test admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: true.to_string(),
            is_project_admin: false.to_string(),
        };
        let clusters = list_clusters(&context).unwrap();
        assert_eq!(clusters.len(), 3);

        // get-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        match get_cluster(&uuid1, &context) {
            Ok(retrieved_cluster) => {
                assert_eq!(retrieved_cluster.uuid, uuid1.to_string());
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
        if get_cluster(&uuid3, &context).is_ok() {
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
        if delete_cluster(&uuid3, &context).is_ok() {
            assert_eq!(true, false);
        };

        hard_delete_cluster(&uuid1);
        hard_delete_cluster(&uuid2);
        hard_delete_cluster(&uuid3);
    }
}
