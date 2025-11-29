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
use diesel::prelude::*;
use std::error::Error;
use uuid::Uuid;

use crate::database::db_handle;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;

// Define the schema
table! {
    proxys (uuid) {
        uuid -> Varchar,
        port -> Integer,
        target_address -> Varchar,
        cluster_uuid -> Varchar,
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
#[diesel(table_name = proxys)]
pub struct ProxyEntry {
    pub uuid: String,
    pub port: i32,
    pub target_address: String,
    pub cluster_uuid: String,
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

pub fn init_proxy_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS proxys (
        uuid VARCHAR(40) PRIMARY KEY,
        port INTEGER,
        target_address VARCHAR(256),
        cluster_uuid VARCHAR(40),
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

pub fn add_new_proxy(
    proxy_uuid: &Uuid,
    port: u16,
    target_address: &str,
    cluster_uuid: &Uuid,
    context: &UserContext,
) -> QueryResult<usize> {
    let proxy = ProxyEntry {
        uuid: proxy_uuid.to_string().clone(),
        port: port.into(),
        target_address: target_address.to_owned(),
        cluster_uuid: cluster_uuid.to_string().clone(),
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

    add_proxy(&proxy)
}

pub fn add_proxy(proxy: &ProxyEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::proxys::dsl::*;
    diesel::insert_into(proxys)
        .values(proxy)
        .execute(&mut *conn)
}

pub fn get_proxy(proxy_uuid: &Uuid, context: &UserContext) -> Result<ProxyEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::proxys::dsl::*;

    let mut query = proxys
        .filter(uuid.eq(proxy_uuid.to_string()).and(status.eq("ACTIVE")))
        .into_boxed();

    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    match query
        .select(ProxyEntry::as_select())
        .first::<ProxyEntry>(&mut *conn)
    {
        Ok(proxy) => Ok(proxy),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

pub fn get_free_proxy(min_port: u16, max_port: u16) -> Result<u16, ErrorResponse> {
    let proxys = list_all_proxys_sorted().map_err(|e| map_db_list_error("proxys", e))?;

    let mut prev_port = min_port;

    // iterate over the port-sorted list to find a free port
    for proxy in proxys {
        if proxy.port as u16 > prev_port {
            return Ok(prev_port);
        }

        prev_port = proxy.port as u16 + 1;
    }

    if prev_port < max_port {
        return Ok(prev_port);
    }

    Err(ErrorResponse::Conflict(
        "Maximum number of proxies reached.".to_string(),
    ))
}

fn list_all_proxys_sorted() -> QueryResult<Vec<ProxyEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::proxys::dsl::*;

    let query = proxys
        .filter(status.eq("ACTIVE"))
        .into_boxed()
        .order(port.asc());
    query.select(ProxyEntry::as_select()).load(&mut *conn)
}

pub fn list_proxys(context: &UserContext) -> QueryResult<Vec<ProxyEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::proxys::dsl::*;

    let mut query = proxys.filter(status.eq("ACTIVE")).into_boxed();

    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    query.select(ProxyEntry::as_select()).load(&mut *conn)
}

pub fn delete_proxy(proxy_uuid: &Uuid, context: &UserContext) -> Result<(), enums::DbError> {
    get_proxy(proxy_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::proxys::dsl::*;
    match diesel::update(proxys.filter(uuid.eq(proxy_uuid.to_string())))
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

pub fn delete_all_proxy() -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::proxys::dsl::*;
    match diesel::update(proxys.filter(status.eq("ACTIVE")))
        .set((
            status.eq("DELETED"),
            deleted_at.eq(Utc::now().to_rfc3339()),
            deleted_by.eq("HANAMI_START"),
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

    fn hard_delete_proxy(proxy_uuid: &Uuid) {
        use self::proxys::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(proxys.filter(uuid.eq(proxy_uuid.to_string()))).execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_add_get_proxy() {
        let _ = init_proxy_table();
        let proxy_uuid1 = Uuid::new_v4();
        let target_address1: String = "127.0.0.1:443".to_string();
        let cluster_uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let proxy = ProxyEntry {
            uuid: proxy_uuid1.to_string(),
            port: 42,
            target_address: target_address1.clone(),
            cluster_uuid: cluster_uuid1.to_string(),
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

        hard_delete_proxy(&proxy_uuid1);

        add_proxy(&proxy).unwrap();
        match get_proxy(&proxy_uuid1, &context) {
            Ok(retrieved_proxy) => {
                assert_eq!(retrieved_proxy.uuid, proxy.uuid);
                assert_eq!(retrieved_proxy.port, proxy.port);
                assert_eq!(retrieved_proxy.target_address, proxy.target_address);
                assert_eq!(retrieved_proxy.cluster_uuid, proxy.cluster_uuid);
                assert_eq!(retrieved_proxy.status, proxy.status);
                assert_eq!(retrieved_proxy.created_by, proxy.created_by);
                assert_eq!(retrieved_proxy.updated_by, proxy.updated_by);
                assert_eq!(retrieved_proxy.deleted_at, proxy.deleted_at);
                assert_eq!(retrieved_proxy.deleted_by, proxy.deleted_by);
            }
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        hard_delete_proxy(&proxy_uuid1);
    }

    #[test]
    #[serial]
    fn test_list_proxys() {
        let _ = init_proxy_table();
        let proxy_uuid1 = Uuid::new_v4();
        let proxy_uuid2 = Uuid::new_v4();
        let target_address1: String = "127.0.0.1:443".to_string();
        let cluster_uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let proxy1 = ProxyEntry {
            uuid: proxy_uuid1.to_string(),
            port: 42,
            target_address: target_address1.clone(),
            cluster_uuid: cluster_uuid1.to_string(),
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

        let proxy2 = ProxyEntry {
            uuid: proxy_uuid2.to_string(),
            port: 43,
            target_address: target_address1.clone(),
            cluster_uuid: cluster_uuid1.to_string(),
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

        hard_delete_proxy(&proxy_uuid1);
        hard_delete_proxy(&proxy_uuid2);

        add_proxy(&proxy1).unwrap();
        add_proxy(&proxy2).unwrap();
        let proxys = list_proxys(&context).unwrap();
        assert_eq!(proxys.len(), 1);
        hard_delete_proxy(&proxy_uuid1);
        hard_delete_proxy(&proxy_uuid2);
    }

    #[test]
    #[serial]
    fn test_delete_proxy() {
        let _ = init_proxy_table();
        let proxy_uuid1 = Uuid::new_v4();
        let target_address1: String = "127.0.0.1:443".to_string();
        let cluster_uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let proxy = ProxyEntry {
            uuid: proxy_uuid1.to_string(),
            port: 42,
            target_address: target_address1.clone(),
            cluster_uuid: cluster_uuid1.to_string(),
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

        hard_delete_proxy(&proxy_uuid1);

        add_proxy(&proxy).unwrap();
        let _ = delete_proxy(&proxy_uuid1, &context);
        let result = get_proxy(&proxy_uuid1, &context);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_proxys_permissions() {
        let _ = init_proxy_table();
        let proxy_uuid1 = Uuid::new_v4();
        let proxy_uuid2 = Uuid::new_v4();
        let proxy_uuid3 = Uuid::new_v4();
        let target_address1: String = "127.0.0.1:443".to_string();
        let cluster_uuid1 = Uuid::new_v4();

        let proxy1 = ProxyEntry {
            uuid: proxy_uuid1.to_string(),
            port: 42,
            target_address: target_address1.clone(),
            cluster_uuid: cluster_uuid1.to_string(),
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

        let proxy2 = ProxyEntry {
            uuid: proxy_uuid2.to_string(),
            port: 43,
            target_address: target_address1.clone(),
            cluster_uuid: cluster_uuid1.to_string(),
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

        let proxy3 = ProxyEntry {
            uuid: proxy_uuid3.to_string(),
            port: 44,
            target_address: target_address1.clone(),
            cluster_uuid: cluster_uuid1.to_string(),
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

        hard_delete_proxy(&proxy_uuid1);
        hard_delete_proxy(&proxy_uuid2);
        hard_delete_proxy(&proxy_uuid3);

        add_proxy(&proxy1).unwrap();
        add_proxy(&proxy2).unwrap();
        add_proxy(&proxy3).unwrap();

        // list-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        let proxys = list_proxys(&context).unwrap();
        assert_eq!(proxys.len(), 1);

        // list-test project-admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: true.to_string(),
        };
        let proxys = list_proxys(&context).unwrap();
        assert_eq!(proxys.len(), 2);

        // list-test admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: true.to_string(),
            is_project_admin: false.to_string(),
        };
        let proxys = list_proxys(&context).unwrap();
        assert_eq!(proxys.len(), 3);

        // get-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        match get_proxy(&proxy_uuid1, &context) {
            Ok(retrieved_proxy) => {
                assert_eq!(retrieved_proxy.uuid, proxy_uuid1.to_string());
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
        if get_proxy(&proxy_uuid3, &context).is_ok() {
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
        if delete_proxy(&proxy_uuid3, &context).is_ok() {
            assert_eq!(true, false);
        };

        hard_delete_proxy(&proxy_uuid1);
        hard_delete_proxy(&proxy_uuid2);
        hard_delete_proxy(&proxy_uuid3);
    }

    #[test]
    #[serial]
    fn test_get_free_proxy() {
        let _ = init_proxy_table();
        let proxy_uuid1 = Uuid::new_v4();
        let proxy_uuid2 = Uuid::new_v4();
        let proxy_uuid3 = Uuid::new_v4();
        let target_address1: String = "127.0.0.1:443".to_string();
        let cluster_uuid1 = Uuid::new_v4();

        let proxy1 = ProxyEntry {
            uuid: proxy_uuid1.to_string(),
            port: 42,
            target_address: target_address1.clone(),
            cluster_uuid: cluster_uuid1.to_string(),
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

        let proxy2 = ProxyEntry {
            uuid: proxy_uuid2.to_string(),
            port: 43,
            target_address: target_address1.clone(),
            cluster_uuid: cluster_uuid1.to_string(),
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

        let proxy3 = ProxyEntry {
            uuid: proxy_uuid3.to_string(),
            port: 44,
            target_address: target_address1.clone(),
            cluster_uuid: cluster_uuid1.to_string(),
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

        hard_delete_proxy(&proxy_uuid1);
        hard_delete_proxy(&proxy_uuid2);
        hard_delete_proxy(&proxy_uuid3);

        assert_eq!(get_free_proxy(42, 45).unwrap(), 42);

        add_proxy(&proxy1).unwrap();

        assert_eq!(get_free_proxy(42, 45).unwrap(), 43);

        add_proxy(&proxy3).unwrap();

        assert_eq!(get_free_proxy(42, 45).unwrap(), 43);

        add_proxy(&proxy2).unwrap();

        assert!(get_free_proxy(42, 45).is_err());

        hard_delete_proxy(&proxy_uuid1);
        hard_delete_proxy(&proxy_uuid2);
        hard_delete_proxy(&proxy_uuid3);
    }
}
