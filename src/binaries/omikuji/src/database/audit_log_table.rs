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
use diesel::prelude::*;
use uuid::Uuid;

use crate::database::db_handle;

use ainari_api_structs::log_structs::AuditLogResp;
use ainari_common::enums;
use ainari_common::objects::*;

// Define the schema for audit_logs table
table! {
    audit_logs (uuid) {
        uuid -> Varchar,
        request_type -> Varchar,
        endpoint -> Varchar,
        component_name -> Varchar,
        host_name -> Varchar,
        user_id -> Varchar,
        project_id -> Varchar,
        created_at -> Varchar,
    }
}

/// Represents an entry in the audit_logs table.
///
/// Entries are only added and never changed or deleted.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = audit_logs)]
pub struct AuditLogEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    pub request_type: String,
    pub endpoint: String,
    pub component_name: String,
    pub host_name: String,
    pub user_id: String,
    pub project_id: String,
    #[diesel(serialize_as = DbDateTime, deserialize_as = DbDateTime)]
    pub created_at: DateTime<Utc>,
}

impl From<AuditLogEntry> for AuditLogResp {
    fn from(entry: AuditLogEntry) -> Self {
        AuditLogResp {
            uuid: entry.uuid,
            request_type: entry.request_type,
            endpoint: entry.endpoint,
            component_name: entry.component_name,
            host_name: entry.host_name,
            user_id: entry.user_id,
            project_id: entry.project_id,
            created_at: entry.created_at,
        }
    }
}

/// Adds a new audit-log to the database.
///
/// The timestamp is set to the current time.
///
/// # Arguments
/// * `audit_log_uuid` - The unique identifier for the new entry
/// * `request_type` - Type of the audited request, like `POST`
/// * `endpoint` - Endpoint, which was called by the audited request
/// * `component_name` - Name of the component, which reported the action
/// * `host_name` - Name of the host, where the action happened
/// * `user_id` - ID of the user, who triggered the action, or empty
/// * `project_id` - ID of the project, in which the action happened, or empty
///
/// # Returns
/// A QueryResult indicating the number of rows affected
pub fn add_new_audit_log(
    audit_log_uuid: &Uuid,
    request_type: &str,
    endpoint: &str,
    component_name: &str,
    host_name: &str,
    user_id: &str,
    project_id: &str,
) -> QueryResult<usize> {
    let new_audit_log = AuditLogEntry {
        uuid: *audit_log_uuid,
        request_type: request_type.to_string(),
        endpoint: endpoint.to_string(),
        component_name: component_name.to_string(),
        host_name: host_name.to_string(),
        user_id: user_id.to_string(),
        project_id: project_id.to_string(),
        created_at: Utc::now(),
    };

    add_audit_log(new_audit_log)
}

/// Adds an audit-log to the database.
///
/// This is a helper function that performs the actual insertion of an AuditLogEntry into the
/// database.
///
/// # Arguments
/// * `entry` - The AuditLogEntry to be inserted
///
/// # Returns
/// A QueryResult indicating the number of rows affected
pub fn add_audit_log(entry: AuditLogEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::audit_logs::dsl::*;
    diesel::insert_into(audit_logs)
        .values(entry)
        .execute(&mut *conn)
}

/// Retrieves an audit-log from the database.
///
/// There is no permission-based filtering, because the entries are only readable by admins,
/// which is already checked by the endpoints.
///
/// # Arguments
/// * `audit_log_uuid` - The UUID of the entry to retrieve
///
/// # Returns
/// A Result containing the AuditLogEntry if found, or a DbError if not found or an error occurs
pub fn get_audit_log(audit_log_uuid: &Uuid) -> Result<AuditLogEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::audit_logs::dsl::*;

    match audit_logs
        .filter(uuid.eq(audit_log_uuid.to_string()))
        .select(AuditLogEntry::as_select())
        .first::<AuditLogEntry>(&mut *conn)
    {
        Ok(entry) => Ok(entry),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Lists the audit-logs, the newest first.
///
/// Each filter is only applied, if it is set.
///
/// # Arguments
/// * `project_filter` - Only list the entries of this project
/// * `user_filter` - Only list the entries of this user
/// * `host_filter` - Only list the entries of this host
/// * `max_entries` - Maximum number of entries to return
///
/// # Returns
/// A QueryResult containing a vector of AuditLogEntry objects
pub fn list_audit_logs(
    project_filter: Option<&str>,
    user_filter: Option<&str>,
    host_filter: Option<&str>,
    max_entries: i64,
) -> QueryResult<Vec<AuditLogEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::audit_logs::dsl::*;

    let mut query = audit_logs.into_boxed();

    if let Some(project) = project_filter {
        query = query.filter(project_id.eq(project));
    }
    if let Some(user) = user_filter {
        query = query.filter(user_id.eq(user));
    }
    if let Some(host) = host_filter {
        query = query.filter(host_name.eq(host));
    }

    // the timestamps are RFC-3339 strings in UTC, whose string-order is the chronological order
    query
        .order(created_at.desc())
        .limit(max_entries)
        .select(AuditLogEntry::as_select())
        .load(&mut *conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serial_test::serial;

    fn hard_delete_audit_log(audit_log_uuid: &Uuid) {
        use self::audit_logs::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(audit_logs.filter(uuid.eq(audit_log_uuid.to_string())))
            .execute(&mut *conn);
    }

    /// Builds an AuditLogEntry for the tests with the given identity and timestamp.
    fn new_entry(
        entry_uuid: &Uuid,
        entry_user_id: &str,
        entry_project_id: &str,
        entry_host_name: &str,
        entry_created_at: DateTime<Utc>,
    ) -> AuditLogEntry {
        AuditLogEntry {
            uuid: *entry_uuid,
            request_type: "POST".to_string(),
            endpoint: "/v1alpha/test".to_string(),
            component_name: "test-component".to_string(),
            host_name: entry_host_name.to_string(),
            user_id: entry_user_id.to_string(),
            project_id: entry_project_id.to_string(),
            created_at: entry_created_at,
        }
    }

    /// Unwraps a get-result. `enums::DbError` implements neither `Debug` nor `PartialEq`,
    /// so the results can not be handled by `expect` and `assert_eq`.
    fn expect_entry(result: Result<AuditLogEntry, enums::DbError>) -> AuditLogEntry {
        match result {
            Ok(entry) => entry,
            Err(_) => panic!("audit-log was not found"),
        }
    }

    #[test]
    #[serial]
    fn test_add_get_audit_log() {
        let uuid1 = Uuid::new_v4();

        hard_delete_audit_log(&uuid1);

        add_new_audit_log(
            &uuid1,
            "POST",
            "/v1alpha/virtual_machine",
            "hanami",
            "host-1",
            "test-user",
            "test-project",
        )
        .unwrap();
        let retrieved = expect_entry(get_audit_log(&uuid1));

        assert_eq!(retrieved.uuid, uuid1);
        assert_eq!(retrieved.request_type, "POST");
        assert_eq!(retrieved.endpoint, "/v1alpha/virtual_machine");
        assert_eq!(retrieved.component_name, "hanami");
        assert_eq!(retrieved.host_name, "host-1");
        assert_eq!(retrieved.user_id, "test-user");
        assert_eq!(retrieved.project_id, "test-project");

        hard_delete_audit_log(&uuid1);
    }

    #[test]
    #[serial]
    fn test_get_audit_log_not_found() {
        let uuid1 = Uuid::new_v4();

        hard_delete_audit_log(&uuid1);

        assert!(matches!(
            get_audit_log(&uuid1),
            Err(enums::DbError::NotFound)
        ));
    }

    #[test]
    #[serial]
    fn test_list_audit_logs_sorted_newest_first() {
        let project = "test_audit_log_sort";
        let base = Utc.with_ymd_and_hms(2026, 10, 10, 12, 0, 0).unwrap();
        // the timestamps have a different number of fractional digits in their string-form,
        // which must not break the order
        let times = [
            base,
            base + chrono::Duration::milliseconds(100),
            base + chrono::Duration::microseconds(100_500),
            base + chrono::Duration::seconds(1),
        ];
        let uuids: Vec<Uuid> = times.iter().map(|_| Uuid::new_v4()).collect();

        // added in a mixed order, so the result is not just the order of insertion
        for i in [2, 0, 3, 1] {
            hard_delete_audit_log(&uuids[i]);
            add_audit_log(new_entry(&uuids[i], "user", project, "host", times[i])).unwrap();
        }

        let entries = list_audit_logs(Some(project), None, None, 100).unwrap();
        let listed: Vec<Uuid> = entries.iter().map(|e| e.uuid).collect();
        assert_eq!(listed, vec![uuids[3], uuids[2], uuids[1], uuids[0]]);
        assert_eq!(entries[0].created_at, times[3]);

        // the limit keeps the newest entries
        let entries = list_audit_logs(Some(project), None, None, 2).unwrap();
        let listed: Vec<Uuid> = entries.iter().map(|e| e.uuid).collect();
        assert_eq!(listed, vec![uuids[3], uuids[2]]);

        for entry_uuid in &uuids {
            hard_delete_audit_log(entry_uuid);
        }
    }

    #[test]
    #[serial]
    fn test_list_audit_logs_filters() {
        let project1 = "test_audit_log_filter_1";
        let project2 = "test_audit_log_filter_2";
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let now = Utc::now();

        let entry1 = new_entry(&uuid1, "test-user-42", project1, "host-a", now);
        let entry2 = new_entry(&uuid2, "test-user-43", project1, "host-b", now);
        let entry3 = new_entry(&uuid3, "test-user-42", project2, "host-a", now);

        hard_delete_audit_log(&uuid1);
        hard_delete_audit_log(&uuid2);
        hard_delete_audit_log(&uuid3);

        add_audit_log(entry1).unwrap();
        add_audit_log(entry2).unwrap();
        add_audit_log(entry3).unwrap();

        // the project-filter alone
        assert_eq!(
            list_audit_logs(Some(project1), None, None, 100)
                .unwrap()
                .len(),
            2
        );

        // the user-filter within a project
        let entries = list_audit_logs(Some(project1), Some("test-user-42"), None, 100).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].uuid, uuid1);

        // the host-filter across projects
        let entries = list_audit_logs(None, Some("test-user-42"), Some("host-a"), 100).unwrap();
        let listed: Vec<Uuid> = entries.iter().map(|e| e.uuid).collect();
        assert!(listed.contains(&uuid1));
        assert!(listed.contains(&uuid3));
        assert!(!listed.contains(&uuid2));

        // all filters together
        let entries =
            list_audit_logs(Some(project1), Some("test-user-43"), Some("host-b"), 100).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].uuid, uuid2);

        // no match
        assert!(
            list_audit_logs(Some(project2), None, Some("host-b"), 100)
                .unwrap()
                .is_empty()
        );

        hard_delete_audit_log(&uuid1);
        hard_delete_audit_log(&uuid2);
        hard_delete_audit_log(&uuid3);
    }
}
