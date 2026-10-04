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

use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;
use ainari_common::objects::*;

// Define the schema for key_packages table
table! {
    key_packages (uuid) {
        uuid -> Varchar,
        client_id -> Varchar,
        key_package -> Varchar,
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

/// Represents an entry in the key_packages table.
///
/// Each entry is one MLS key-package of a MLS-client, which can be claimed exactly once to invite
/// the client into a group.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = key_packages)]
pub struct KeyPackageEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    pub client_id: String,
    /// Base64-encoded TLS-serialized key-package
    pub key_package: String,
    pub owner_id: String,
    pub project_id: String,
    pub status: String,
    #[diesel(serialize_as = DbDateTime, deserialize_as = DbDateTime)]
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    #[diesel(serialize_as = DbDateTime, deserialize_as = DbDateTime)]
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
    #[diesel(serialize_as = DbOptDateTime, deserialize_as = DbOptDateTime)]
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<String>,
}

/// Adds new key-packages of a MLS-client to the database.
///
/// All key-packages are added within one transaction, so either all of them or none are
/// available afterwards.
///
/// # Arguments
/// * `key_package_client_id` - Identity of the MLS-client, which owns the key-packages
/// * `encoded_key_packages` - Base64-encoded TLS-serialized key-packages
/// * `context` - The user context containing information about the user and project
///
/// # Returns
/// The UUIDs of the new entries in the order of the given key-packages
pub fn add_key_packages(
    key_package_client_id: &str,
    encoded_key_packages: &[String],
    context: &UserContext,
) -> QueryResult<Vec<Uuid>> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::permission_denied_error());
    }

    let entries: Vec<KeyPackageEntry> = encoded_key_packages
        .iter()
        .map(|encoded| KeyPackageEntry {
            uuid: Uuid::new_v4(),
            client_id: key_package_client_id.to_string(),
            key_package: encoded.clone(),
            owner_id: context.user_id.clone(),
            project_id: context.project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: context.user_id.clone(),
            updated_at: Utc::now(),
            updated_by: context.user_id.clone(),
            deleted_at: None,
            deleted_by: None,
        })
        .collect();
    let uuids = entries.iter().map(|entry| entry.uuid).collect();

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.transaction::<_, diesel::result::Error, _>(|conn| {
        use self::key_packages::dsl::*;
        for entry in entries {
            diesel::insert_into(key_packages)
                .values(entry)
                .execute(conn)?;
        }
        Ok(())
    })?;

    Ok(uuids)
}

/// Claims the newest key-package of a MLS-client.
///
/// A key-package must only be used for one invitation, so it is marked as deleted with the
/// claim. The newest one is taken, because its private part is the most likely one to still be
/// known by the client. The deletion only succeeds, if the entry is still active, so two parallel
/// claims, also from different instances of the izakaya, never get the same key-package.
///
/// # Arguments
/// * `key_package_client_id` - Identity of the MLS-client, whose key-package is claimed
/// * `accept` - Filter for the key-packages, which may be claimed. Key-packages, which don't
///   pass it, stay where they are.
/// * `context` - The user context containing information about the user, who claims it
///
/// # Returns
/// The claimed entry, `NotFound` if the client has no key-package left, or another `DbError`
pub fn claim_key_package(
    key_package_client_id: &str,
    accept: impl Fn(&KeyPackageEntry) -> bool,
    context: &UserContext,
) -> Result<KeyPackageEntry, enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    let result = conn.transaction::<_, diesel::result::Error, _>(|conn| {
        use self::key_packages::dsl::*;

        let mut candidates = key_packages
            .filter(client_id.eq(key_package_client_id).and(status.eq("ACTIVE")))
            .select(KeyPackageEntry::as_select())
            .load(conn)?;
        candidates.sort_by_key(|entry| std::cmp::Reverse(entry.created_at));

        for candidate in candidates.into_iter().filter(|entry| accept(entry)) {
            let claimed = diesel::update(
                key_packages.filter(uuid.eq(candidate.uuid.to_string()).and(status.eq("ACTIVE"))),
            )
            .set((
                status.eq("DELETED"),
                deleted_at.eq(Utc::now().to_rfc3339()),
                deleted_by.eq(context.user_id.clone()),
            ))
            .execute(conn)?;

            if claimed == 1 {
                return Ok(Some(candidate));
            }
        }

        Ok(None)
    });

    match result {
        Ok(Some(entry)) => Ok(entry),
        Ok(None) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Counts the key-packages of a MLS-client, which can still be claimed.
///
/// # Arguments
/// * `key_package_client_id` - Identity of the MLS-client
///
/// # Returns
/// A QueryResult containing the number of active key-packages
pub fn count_key_packages(key_package_client_id: &str) -> QueryResult<i64> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::key_packages::dsl::*;

    key_packages
        .filter(client_id.eq(key_package_client_id).and(status.eq("ACTIVE")))
        .select(count_star())
        .first::<i64>(&mut *conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ainari_common::enums::ProjectRole;
    use serial_test::serial;

    /// Builds a UserContext for the tests.
    fn new_context(project_role: ProjectRole) -> UserContext {
        UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: "test-project".to_string(),
            is_admin: false.to_string(),
            project_role: project_role.to_string(),
        }
    }

    /// Builds a client-id, which no other test uses.
    fn new_client_id() -> String {
        format!("test-client-{}", Uuid::new_v4())
    }

    #[test]
    #[serial]
    fn a_key_package_can_only_be_claimed_once() {
        let context = new_context(ProjectRole::Member);
        let client = new_client_id();

        let uuids = add_key_packages(&client, &["first".to_string()], &context).unwrap();
        assert_eq!(count_key_packages(&client).unwrap(), 1);

        let claimed = claim_key_package(&client, |_| true, &context).ok().unwrap();
        assert_eq!(claimed.uuid, uuids[0]);
        assert_eq!(claimed.key_package, "first");

        assert_eq!(count_key_packages(&client).unwrap(), 0);
        assert!(matches!(
            claim_key_package(&client, |_| true, &context),
            Err(enums::DbError::NotFound)
        ));
    }

    #[test]
    #[serial]
    fn the_newest_key_package_is_claimed_first() {
        let context = new_context(ProjectRole::Member);
        let client = new_client_id();

        add_key_packages(&client, &["old".to_string()], &context).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        add_key_packages(&client, &["new".to_string()], &context).unwrap();

        let first = claim_key_package(&client, |_| true, &context).ok().unwrap();
        let second = claim_key_package(&client, |_| true, &context).ok().unwrap();
        assert_eq!(first.key_package, "new");
        assert_eq!(second.key_package, "old");
    }

    #[test]
    #[serial]
    fn the_key_packages_of_other_clients_are_not_touched() {
        let context = new_context(ProjectRole::Member);
        let client = new_client_id();
        let other_client = new_client_id();

        add_key_packages(&other_client, &["other".to_string()], &context).unwrap();

        assert!(matches!(
            claim_key_package(&client, |_| true, &context),
            Err(enums::DbError::NotFound)
        ));
        assert_eq!(count_key_packages(&other_client).unwrap(), 1);
    }

    #[test]
    #[serial]
    fn only_an_accepted_key_package_is_claimed() {
        let context = new_context(ProjectRole::Member);
        let client = new_client_id();

        add_key_packages(&client, &["foreign".to_string()], &context).unwrap();
        assert!(matches!(
            claim_key_package(&client, |entry| entry.key_package == "own", &context),
            Err(enums::DbError::NotFound)
        ));
        // the refused key-package stays where it is
        assert_eq!(count_key_packages(&client).unwrap(), 1);
    }

    #[test]
    #[serial]
    fn an_observer_can_neither_upload_nor_claim() {
        let observer = new_context(ProjectRole::Observer);
        let client = new_client_id();

        assert!(add_key_packages(&client, &["kp".to_string()], &observer).is_err());
        assert!(matches!(
            claim_key_package(&client, |_| true, &observer),
            Err(enums::DbError::PermissionDenied)
        ));
    }
}
