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

use crate::database::db_handle;

use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;
use ainari_common::enums::ProjectRole;
use ainari_common::objects::*;

// Define the schema for the user-project-mapping table in the database.
// The combination of project and user is unique within the active entries, which is enforced by a
// unique index in the database, so it is also declared as primary key for diesel.
table! {
    user_project_mapping (project_id, user_id) {
        project_id -> Varchar,
        user_id -> Varchar,
        role -> Varchar,
        status -> Varchar,
        created_at -> Varchar,
        created_by -> Varchar,
        updated_at -> Varchar,
        updated_by -> Varchar,
        deleted_at -> Nullable<Varchar>,
        deleted_by -> Nullable<Varchar>,
    }
}

/// Represents a mapping between a user and a project in the database.
///
/// Each entry assigns a user with a specific role to a project.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = user_project_mapping)]
pub struct UserProjectMappingEntry {
    pub project_id: String,
    pub user_id: String,
    #[diesel(serialize_as = DbProjectRole, deserialize_as = DbProjectRole)]
    pub role: ProjectRole,
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

/// Adds a new user-project-mapping to the system.
///
/// The permissions are not checked here, so this has to be done by the caller.
///
/// # Arguments
///
/// * `mapping_project_id` - The ID of the project
/// * `mapping_user_id` - The ID of the user
/// * `mapping_role` - The role of the user within the project
/// * `context` - The user context containing authentication information
///
/// # Returns
///
/// Returns the number of rows affected by the insert operation.
pub fn add_new_mapping(
    mapping_project_id: &str,
    mapping_user_id: &str,
    mapping_role: ProjectRole,
    context: &UserContext,
) -> QueryResult<usize> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::permission_denied_error());
    }

    let mapping = UserProjectMappingEntry {
        project_id: mapping_project_id.to_owned(),
        user_id: mapping_user_id.to_owned(),
        role: mapping_role,
        status: "ACTIVE".to_string(),
        created_at: Utc::now(),
        created_by: context.user_id.clone(),
        updated_at: Utc::now(),
        updated_by: context.user_id.clone(),
        deleted_at: None,
        deleted_by: None,
    };

    add_mapping(mapping)
}

/// Inserts a user-project-mapping into the database.
///
/// # Arguments
///
/// * `mapping` - The mapping to insert
///
/// # Returns
///
/// Returns the number of rows affected by the insert operation.
pub fn add_mapping(mapping: UserProjectMappingEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::user_project_mapping::dsl::*;

    diesel::insert_into(user_project_mapping)
        .values(mapping)
        .execute(&mut *conn)
}

/// Retrieves the active mapping of a user to a project.
///
/// The permissions are not checked here, so this has to be done by the caller.
///
/// # Arguments
///
/// * `mapping_project_id` - The ID of the project
/// * `mapping_user_id` - The ID of the user
///
/// # Returns
///
/// Returns the mapping if found, or an appropriate DbError if not found
/// or if there's an internal error.
pub fn get_mapping(
    mapping_project_id: &String,
    mapping_user_id: &String,
) -> Result<UserProjectMappingEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::user_project_mapping::dsl::*;
    match user_project_mapping
        .filter(
            project_id
                .eq(mapping_project_id)
                .and(user_id.eq(mapping_user_id))
                .and(status.eq("ACTIVE")),
        )
        .select(UserProjectMappingEntry::as_select())
        .first::<UserProjectMappingEntry>(&mut *conn)
    {
        Ok(mapping) => Ok(mapping),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Sets the role of the active mapping of a user to a project.
///
/// Only an already existing active mapping is updated, so no new mapping is created here.
///
/// # Arguments
///
/// * `mapping_project_id` - The ID of the project
/// * `mapping_user_id` - The ID of the user
/// * `mapping_role` - The new role of the user within the project
/// * `context` - The user context containing authentication information
///
/// # Returns
///
/// Returns Ok(()) if the role was set, DbError::NotFound if there is no active mapping
/// or the user lacks permissions, or DbError::InternalError if there was an internal error.
pub fn set_mapping_role(
    mapping_project_id: &String,
    mapping_user_id: &String,
    mapping_role: ProjectRole,
    context: &UserContext,
) -> Result<(), enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::user_project_mapping::dsl::*;
    match diesel::update(
        user_project_mapping.filter(
            project_id
                .eq(mapping_project_id)
                .and(user_id.eq(mapping_user_id))
                .and(status.eq("ACTIVE")),
        ),
    )
    .set((
        role.eq(DbProjectRole::from(mapping_role)),
        updated_at.eq(Utc::now().to_rfc3339()),
        updated_by.eq(context.user_id.clone()),
    ))
    .execute(&mut *conn)
    {
        Ok(0) => Err(enums::DbError::NotFound),
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Deletes the active mapping of a user to a project.
///
/// This function marks the mapping as "DELETED" in the database. Older, already deleted
/// mappings of the same pair are not touched.
///
/// # Arguments
///
/// * `mapping_project_id` - The ID of the project
/// * `mapping_user_id` - The ID of the user
/// * `context` - The user context containing authentication information
///
/// # Returns
///
/// Returns Ok(()) if the mapping was deleted, DbError::NotFound if there is no active mapping
/// or the user lacks permissions, or DbError::InternalError if there was an internal error.
pub fn delete_mapping(
    mapping_project_id: &String,
    mapping_user_id: &String,
    context: &UserContext,
) -> Result<(), enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::user_project_mapping::dsl::*;
    match diesel::update(
        user_project_mapping.filter(
            project_id
                .eq(mapping_project_id)
                .and(user_id.eq(mapping_user_id))
                .and(status.eq("ACTIVE")),
        ),
    )
    .set((
        status.eq("DELETED"),
        deleted_at.eq(Utc::now().to_rfc3339()),
        deleted_by.eq(context.user_id.clone()),
    ))
    .execute(&mut *conn)
    {
        Ok(0) => Err(enums::DbError::NotFound),
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Lists all active mappings of a specific project.
///
/// # Arguments
///
/// * `mapping_project_id` - The ID of the project, whose mappings should be listed
///
/// # Returns
///
/// Returns a vector of all active mappings of the project.
pub fn list_mappings_of_project(
    mapping_project_id: &String,
) -> QueryResult<Vec<UserProjectMappingEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::user_project_mapping::dsl::*;
    user_project_mapping
        .filter(project_id.eq(mapping_project_id).and(status.eq("ACTIVE")))
        .select(UserProjectMappingEntry::as_select())
        .load(&mut *conn)
}

/// Lists all active mappings of a specific user.
///
/// # Arguments
///
/// * `mapping_user_id` - The ID of the user, whose mappings should be listed
///
/// # Returns
///
/// Returns a vector of all active mappings of the user.
pub fn list_mappings_of_user(
    mapping_user_id: &String,
) -> QueryResult<Vec<UserProjectMappingEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::user_project_mapping::dsl::*;
    user_project_mapping
        .filter(user_id.eq(mapping_user_id).and(status.eq("ACTIVE")))
        .select(UserProjectMappingEntry::as_select())
        .load(&mut *conn)
}

/// Deletes all mappings of a specific user.
///
/// This function marks all mappings of the user as "DELETED" in the database.
///
/// # Arguments
///
/// * `mapping_user_id` - The ID of the user, whose mappings should be deleted
///
/// # Returns
///
/// Returns Ok(()) if the mappings were deleted, also if there were none, or
/// DbError::InternalError if there was an internal error.
#[allow(dead_code)]
pub fn delete_mappings_of_user(mapping_user_id: &String) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::user_project_mapping::dsl::*;
    match diesel::update(user_project_mapping.filter(user_id.eq(mapping_user_id)))
        .set(status.eq("DELETED"))
        .execute(&mut *conn)
    {
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Deletes all mappings of a specific project.
///
/// This function marks all mappings of the project as "DELETED" in the database.
///
/// # Arguments
///
/// * `mapping_project_id` - The ID of the project, whose mappings should be deleted
///
/// # Returns
///
/// Returns Ok(()) if the mappings were deleted, also if there were none, or
/// DbError::InternalError if there was an internal error.
#[allow(dead_code)]
pub fn delete_mappings_of_project(mapping_project_id: &String) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::user_project_mapping::dsl::*;
    match diesel::update(user_project_mapping.filter(project_id.eq(mapping_project_id)))
        .set(status.eq("DELETED"))
        .execute(&mut *conn)
    {
        Ok(_) => Ok(()),
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

    fn test_context() -> UserContext {
        UserContext {
            token: "".to_string(),
            user_id: "admin".to_string(),
            project_id: "test-project-1".to_string(),
            is_admin: true.to_string(),
            project_role: ProjectRole::Member.to_string(),
        }
    }

    fn hard_delete_mappings_of_user(mapping_user_id: &String) {
        use self::user_project_mapping::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(user_project_mapping.filter(user_id.eq(mapping_user_id)))
            .execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_add_list_mapping() {
        let context = test_context();
        let user_id = "test-mapping-user-1".to_string();
        hard_delete_mappings_of_user(&user_id);

        add_new_mapping(
            "test-mapping-project-1",
            &user_id,
            ProjectRole::Admin,
            &context,
        )
        .unwrap();
        add_new_mapping(
            "test-mapping-project-2",
            &user_id,
            ProjectRole::Member,
            &context,
        )
        .unwrap();

        let mappings = list_mappings_of_user(&user_id).unwrap();
        assert_eq!(mappings.len(), 2);
        assert!(
            mappings
                .iter()
                .any(|m| m.project_id == "test-mapping-project-1"
                    && m.role == ProjectRole::Admin
                    && m.status == "ACTIVE"
                    && m.created_by == context.user_id)
        );
        assert!(
            mappings
                .iter()
                .any(|m| m.project_id == "test-mapping-project-2" && m.role == ProjectRole::Member)
        );

        hard_delete_mappings_of_user(&user_id);
    }

    #[test]
    #[serial]
    fn test_delete_mappings_of_user() {
        let context = test_context();
        let user_id1 = "test-mapping-user-2".to_string();
        let user_id2 = "test-mapping-user-3".to_string();
        hard_delete_mappings_of_user(&user_id1);
        hard_delete_mappings_of_user(&user_id2);

        add_new_mapping(
            "test-mapping-project-3",
            &user_id1,
            ProjectRole::Admin,
            &context,
        )
        .unwrap();
        add_new_mapping(
            "test-mapping-project-3",
            &user_id2,
            ProjectRole::Admin,
            &context,
        )
        .unwrap();

        assert!(delete_mappings_of_user(&user_id1).is_ok());
        assert!(list_mappings_of_user(&user_id1).unwrap().is_empty());
        assert_eq!(list_mappings_of_user(&user_id2).unwrap().len(), 1);

        hard_delete_mappings_of_user(&user_id1);
        hard_delete_mappings_of_user(&user_id2);
    }

    #[test]
    #[serial]
    fn test_delete_mappings_of_project() {
        let context = test_context();
        let user_id = "test-mapping-user-4".to_string();
        let project_id1 = "test-mapping-project-4".to_string();
        let project_id2 = "test-mapping-project-5".to_string();
        hard_delete_mappings_of_user(&user_id);

        add_new_mapping(&project_id1, &user_id, ProjectRole::Admin, &context).unwrap();
        add_new_mapping(&project_id2, &user_id, ProjectRole::Admin, &context).unwrap();

        assert!(delete_mappings_of_project(&project_id1).is_ok());
        let mappings = list_mappings_of_user(&user_id).unwrap();
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].project_id, project_id2);

        hard_delete_mappings_of_user(&user_id);
    }

    #[test]
    #[serial]
    fn test_unknown_role_is_rejected() {
        let mapping_user_id = "test-mapping-user-5".to_string();
        hard_delete_mappings_of_user(&mapping_user_id);

        // write the row directly, because the api of this table only accepts valid roles
        {
            use self::user_project_mapping::dsl::*;
            let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
            diesel::insert_into(user_project_mapping)
                .values((
                    project_id.eq("test-mapping-project-6"),
                    user_id.eq(&mapping_user_id),
                    role.eq("superuser"),
                    status.eq("ACTIVE"),
                    created_at.eq(Utc::now().to_rfc3339()),
                    created_by.eq("admin"),
                    updated_at.eq(Utc::now().to_rfc3339()),
                    updated_by.eq("admin"),
                ))
                .execute(&mut *conn)
                .unwrap();
        }

        assert!(list_mappings_of_user(&mapping_user_id).is_err());

        hard_delete_mappings_of_user(&mapping_user_id);
    }

    #[test]
    #[serial]
    fn test_mapping_pair_is_unique() {
        let context = test_context();
        let user_id1 = "test-mapping-user-6".to_string();
        let user_id2 = "test-mapping-user-7".to_string();
        hard_delete_mappings_of_user(&user_id1);
        hard_delete_mappings_of_user(&user_id2);

        add_new_mapping(
            "test-mapping-project-7",
            &user_id1,
            ProjectRole::Admin,
            &context,
        )
        .unwrap();

        // the same pair is rejected, also with a different role
        let result = add_new_mapping(
            "test-mapping-project-7",
            &user_id1,
            ProjectRole::Member,
            &context,
        );
        assert!(matches!(
            result,
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _
            ))
        ));

        // each value alone can exist multiple times
        add_new_mapping(
            "test-mapping-project-8",
            &user_id1,
            ProjectRole::Admin,
            &context,
        )
        .unwrap();
        add_new_mapping(
            "test-mapping-project-7",
            &user_id2,
            ProjectRole::Admin,
            &context,
        )
        .unwrap();
        assert_eq!(list_mappings_of_user(&user_id1).unwrap().len(), 2);
        assert_eq!(list_mappings_of_user(&user_id2).unwrap().len(), 1);

        // a deleted entry doesn't block the pair, because only the active entries are unique
        assert!(delete_mappings_of_user(&user_id2).is_ok());
        add_new_mapping(
            "test-mapping-project-7",
            &user_id2,
            ProjectRole::Member,
            &context,
        )
        .unwrap();
        assert_eq!(list_mappings_of_user(&user_id2).unwrap().len(), 1);

        hard_delete_mappings_of_user(&user_id1);
        hard_delete_mappings_of_user(&user_id2);
    }

    #[test]
    #[serial]
    fn test_get_mapping() {
        let context = test_context();
        let user_id = "test-mapping-user-8".to_string();
        let project_id = "test-mapping-project-9".to_string();
        hard_delete_mappings_of_user(&user_id);

        assert!(get_mapping(&project_id, &user_id).is_err());

        add_new_mapping(&project_id, &user_id, ProjectRole::Observer, &context).unwrap();
        let Ok(mapping) = get_mapping(&project_id, &user_id) else {
            panic!("mapping was not found");
        };
        assert_eq!(mapping.role, ProjectRole::Observer);

        // deleted mappings are not returned anymore
        assert!(delete_mappings_of_user(&user_id).is_ok());
        assert!(get_mapping(&project_id, &user_id).is_err());

        hard_delete_mappings_of_user(&user_id);
    }

    #[test]
    #[serial]
    fn test_observer_can_not_add_mapping() {
        let user_id = "test-mapping-user-9".to_string();
        let observer = UserContext {
            is_admin: false.to_string(),
            project_role: ProjectRole::Observer.to_string(),
            ..test_context()
        };
        let admin_observer = UserContext {
            is_admin: true.to_string(),
            ..observer.clone()
        };
        hard_delete_mappings_of_user(&user_id);

        let result = add_new_mapping(
            "test-mapping-project-10",
            &user_id,
            ProjectRole::Admin,
            &observer,
        );
        assert!(matches!(result, Err(e) if enums::is_permission_denied(&e)));
        assert!(list_mappings_of_user(&user_id).unwrap().is_empty());

        // an admin is not restricted, even as observer of the project
        add_new_mapping(
            "test-mapping-project-10",
            &user_id,
            ProjectRole::Admin,
            &admin_observer,
        )
        .unwrap();

        hard_delete_mappings_of_user(&user_id);
    }

    #[test]
    #[serial]
    fn test_delete_mapping() {
        let context = test_context();
        let user_id = "test-mapping-user-10".to_string();
        let project_id1 = "test-mapping-project-11".to_string();
        let project_id2 = "test-mapping-project-12".to_string();
        hard_delete_mappings_of_user(&user_id);

        add_new_mapping(&project_id1, &user_id, ProjectRole::Member, &context).unwrap();
        add_new_mapping(&project_id2, &user_id, ProjectRole::Member, &context).unwrap();

        // only the mapping of the given pair is deleted
        assert!(delete_mapping(&project_id1, &user_id, &context).is_ok());
        let mappings = list_mappings_of_user(&user_id).unwrap();
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].project_id, project_id2);

        // deleting it a second time finds no active mapping anymore
        assert!(matches!(
            delete_mapping(&project_id1, &user_id, &context),
            Err(enums::DbError::NotFound)
        ));

        // the pair can be assigned again after it was deleted
        add_new_mapping(&project_id1, &user_id, ProjectRole::Observer, &context).unwrap();
        assert!(get_mapping(&project_id1, &user_id).is_ok());

        hard_delete_mappings_of_user(&user_id);
    }

    #[test]
    #[serial]
    fn test_set_mapping_role() {
        let context = test_context();
        let user_id = "test-mapping-user-11".to_string();
        let project_id = "test-mapping-project-13".to_string();
        hard_delete_mappings_of_user(&user_id);

        // without an existing mapping, nothing is created
        assert!(matches!(
            set_mapping_role(&project_id, &user_id, ProjectRole::Admin, &context),
            Err(enums::DbError::NotFound)
        ));
        assert!(get_mapping(&project_id, &user_id).is_err());

        add_new_mapping(&project_id, &user_id, ProjectRole::Observer, &context).unwrap();
        assert!(set_mapping_role(&project_id, &user_id, ProjectRole::Member, &context).is_ok());
        let Ok(mapping) = get_mapping(&project_id, &user_id) else {
            panic!("mapping was not found");
        };
        assert_eq!(mapping.role, ProjectRole::Member);

        // a deleted mapping can not be changed anymore
        assert!(delete_mapping(&project_id, &user_id, &context).is_ok());
        assert!(matches!(
            set_mapping_role(&project_id, &user_id, ProjectRole::Admin, &context),
            Err(enums::DbError::NotFound)
        ));

        hard_delete_mappings_of_user(&user_id);
    }

    #[test]
    #[serial]
    fn test_list_mappings_of_project() {
        let context = test_context();
        let project_id = "test-mapping-project-14".to_string();
        let user_id1 = "test-mapping-user-12".to_string();
        let user_id2 = "test-mapping-user-13".to_string();
        hard_delete_mappings_of_user(&user_id1);
        hard_delete_mappings_of_user(&user_id2);

        add_new_mapping(&project_id, &user_id1, ProjectRole::Admin, &context).unwrap();
        add_new_mapping(&project_id, &user_id2, ProjectRole::Observer, &context).unwrap();
        add_new_mapping(
            "test-mapping-project-15",
            &user_id1,
            ProjectRole::Member,
            &context,
        )
        .unwrap();

        // only the active mappings of the given project are listed
        assert!(delete_mapping(&project_id, &user_id2, &context).is_ok());
        let mappings = list_mappings_of_project(&project_id).unwrap();
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].user_id, user_id1);
        assert_eq!(mappings[0].role, ProjectRole::Admin);

        hard_delete_mappings_of_user(&user_id1);
        hard_delete_mappings_of_user(&user_id2);
    }
}
