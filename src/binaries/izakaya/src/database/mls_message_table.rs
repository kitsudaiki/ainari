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

use ainari_api_structs::mls_structs::{MlsGrant, MlsMessageReq};
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;
use ainari_common::objects::*;

// Define the schema for mls_messages table
table! {
    mls_messages (uuid) {
        uuid -> Varchar,
        group_id -> Varchar,
        epoch -> BigInt,
        message_type -> Varchar,
        sender -> Varchar,
        recipient -> Varchar,
        payload -> Varchar,
        owner_id -> Varchar,
        project_id -> Varchar,
        status -> Varchar,
        created_at -> Varchar,
        created_by -> Varchar,
        updated_at -> Varchar,
        updated_by -> Varchar,
        deleted_at -> Nullable<Varchar>,
        deleted_by -> Nullable<Varchar>,
        grants -> Nullable<Text>,
    }
}

/// Represents an entry in the mls_messages table.
///
/// A message, which has more than one recipient, is stored once per recipient, so each of them
/// can acknowledge its copy on its own.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = mls_messages)]
pub struct MlsMessageEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    pub group_id: String,
    pub epoch: i64,
    pub message_type: String,
    pub sender: String,
    pub recipient: String,
    /// Base64-encoded TLS-serialized MLS-message
    pub payload: String,
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
    /// JSON-encoded grants of the gateways, which a commit adds
    pub grants: Option<String>,
}

impl MlsMessageEntry {
    /// Reads the grants, which are attached to the message.
    ///
    /// # Returns
    /// The grants, or none, if the message has none or they can't be read
    pub fn grants(&self) -> Vec<MlsGrant> {
        self.grants
            .as_deref()
            .and_then(|grants| serde_json::from_str(grants).ok())
            .unwrap_or_default()
    }
}

/// Adds a MLS-message for all of its recipients to the database.
///
/// All copies are added within one transaction, so a message is either delivered to all of its
/// recipients or to none of them.
///
/// # Arguments
/// * `req` - The message together with its recipients
/// * `context` - The user context containing information about the user and project
///
/// # Returns
/// The UUIDs of the new entries in the order of the recipients
pub fn add_mls_message(req: &MlsMessageReq, context: &UserContext) -> QueryResult<Vec<Uuid>> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::permission_denied_error());
    }

    let message_epoch = i64::try_from(req.epoch).map_err(|_| {
        diesel::result::Error::SerializationError("epoch is out of range".to_string().into())
    })?;
    let message_grants = match req.grants.is_empty() {
        true => None,
        false => Some(
            serde_json::to_string(&req.grants)
                .map_err(|e| diesel::result::Error::SerializationError(e.into()))?,
        ),
    };

    let entries: Vec<MlsMessageEntry> = req
        .recipients
        .iter()
        .map(|message_recipient| MlsMessageEntry {
            uuid: Uuid::new_v4(),
            group_id: req.group_id.clone(),
            epoch: message_epoch,
            message_type: req.message_type.to_string(),
            sender: req.sender.clone(),
            recipient: message_recipient.clone(),
            payload: req.payload.clone(),
            owner_id: context.user_id.clone(),
            project_id: context.project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: context.user_id.clone(),
            updated_at: Utc::now(),
            updated_by: context.user_id.clone(),
            deleted_at: None,
            deleted_by: None,
            grants: message_grants.clone(),
        })
        .collect();
    let uuids = entries.iter().map(|entry| entry.uuid).collect();

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.transaction::<_, diesel::result::Error, _>(|conn| {
        use self::mls_messages::dsl::*;
        for entry in entries {
            diesel::insert_into(mls_messages)
                .values(entry)
                .execute(conn)?;
        }
        Ok(())
    })?;

    Ok(uuids)
}

/// Lists all messages, which wait for a MLS-client.
///
/// The messages are ordered by their group, their epoch and their creation, which is the order,
/// in which the client has to process them.
///
/// # Arguments
/// * `message_recipient` - Identity of the MLS-client
///
/// # Returns
/// A QueryResult containing the pending messages of the client
pub fn list_mls_messages(message_recipient: &str) -> QueryResult<Vec<MlsMessageEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::mls_messages::dsl::*;

    let mut entries = mls_messages
        .filter(recipient.eq(message_recipient).and(status.eq("ACTIVE")))
        .select(MlsMessageEntry::as_select())
        .load(&mut *conn)?;
    entries.sort_by(|a, b| {
        (&a.group_id, a.epoch, a.created_at).cmp(&(&b.group_id, b.epoch, b.created_at))
    });
    Ok(entries)
}

/// Marks a message of a MLS-client as delivered, which removes it from its pending messages.
///
/// # Arguments
/// * `message_recipient` - Identity of the MLS-client, which received the message
/// * `message_uuid` - UUID of the message
/// * `context` - The user context to record who acknowledged the message
///
/// # Returns
/// `Ok(())`, `NotFound` if the client has no such pending message, or another `DbError`
pub fn delete_mls_message(
    message_recipient: &str,
    message_uuid: &Uuid,
    context: &UserContext,
) -> Result<(), enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::mls_messages::dsl::*;
    match diesel::update(
        mls_messages.filter(
            uuid.eq(message_uuid.to_string())
                .and(recipient.eq(message_recipient))
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

#[cfg(test)]
mod tests {
    use super::*;
    use ainari_api_structs::mls_structs::MlsMessageType;
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

    fn new_message(group: &str, message_epoch: u64, recipients: &[&String]) -> MlsMessageReq {
        MlsMessageReq {
            group_id: group.to_string(),
            epoch: message_epoch,
            message_type: MlsMessageType::Commit,
            sender: "test-sender".to_string(),
            recipients: recipients.iter().map(|it| it.to_string()).collect(),
            payload: format!("payload-{message_epoch}"),
            grants: Vec::new(),
        }
    }

    #[test]
    #[serial]
    fn every_recipient_gets_its_own_copy() {
        let context = new_context(ProjectRole::Member);
        let first = new_client_id();
        let second = new_client_id();

        let uuids = add_mls_message(&new_message("g", 1, &[&first, &second]), &context).unwrap();
        assert_eq!(uuids.len(), 2);

        // acknowledging the copy of one recipient keeps the copy of the other one
        let first_messages = list_mls_messages(&first).unwrap();
        assert_eq!(first_messages.len(), 1);
        delete_mls_message(&first, &first_messages[0].uuid, &context)
            .ok()
            .unwrap();

        assert!(list_mls_messages(&first).unwrap().is_empty());
        assert_eq!(list_mls_messages(&second).unwrap().len(), 1);
    }

    #[test]
    #[serial]
    fn the_messages_are_listed_in_the_order_of_their_epochs() {
        let context = new_context(ProjectRole::Member);
        let client = new_client_id();

        add_mls_message(&new_message("g", 3, &[&client]), &context).unwrap();
        add_mls_message(&new_message("g", 2, &[&client]), &context).unwrap();

        let epochs: Vec<i64> = list_mls_messages(&client)
            .unwrap()
            .iter()
            .map(|entry| entry.epoch)
            .collect();
        assert_eq!(epochs, vec![2, 3]);
    }

    #[test]
    #[serial]
    fn only_the_recipient_can_acknowledge_a_message() {
        let context = new_context(ProjectRole::Member);
        let client = new_client_id();
        let other_client = new_client_id();

        let uuids = add_mls_message(&new_message("g", 1, &[&client]), &context).unwrap();

        assert!(matches!(
            delete_mls_message(&other_client, &uuids[0], &context),
            Err(enums::DbError::NotFound)
        ));
        assert!(delete_mls_message(&client, &uuids[0], &context).is_ok());
        assert!(matches!(
            delete_mls_message(&client, &uuids[0], &context),
            Err(enums::DbError::NotFound)
        ));
    }
}
