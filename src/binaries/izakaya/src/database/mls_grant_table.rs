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

//! Membership-grants of hanami, which izakaya keeps for the gateways of every network.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::database::db_handle;

use ainari_api_structs::mls_structs::MlsGrant;
use ainari_common::objects::*;

// Define the schema for mls_grants table
table! {
    mls_grants (uuid) {
        uuid -> Varchar,
        vni -> Integer,
        client_id -> Varchar,
        signature_key -> Varchar,
        payload -> Text,
        signature -> Varchar,
        expires_at -> BigInt,
        created_at -> Varchar,
    }
}

/// The grant of hanami, which allows a gateway to be member of the group of a network
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = mls_grants)]
pub struct MlsGrantEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    #[diesel(serialize_as = DbVni, deserialize_as = DbVni)]
    pub vni: u32,
    pub client_id: String,
    /// Base64-encoded MLS signature-key of the gateway
    pub signature_key: String,
    /// The signed grant as it came from hanami
    pub payload: String,
    pub signature: String,
    /// Unix-time in seconds, after which the grant doesn't allow to add the gateway anymore
    pub expires_at: i64,
    #[diesel(serialize_as = DbDateTime, deserialize_as = DbDateTime)]
    pub created_at: DateTime<Utc>,
}

impl MlsGrantEntry {
    /// Returns the grant in the form, which hanami signed.
    pub fn grant(&self) -> MlsGrant {
        MlsGrant {
            payload: self.payload.clone(),
            signature: self.signature.clone(),
        }
    }
}

/// Stores the grant of a gateway, which replaces an older grant of the same gateway and network.
///
/// # Arguments
/// * `entry` - The new grant
///
/// # Returns
/// `Ok(())` once the grant is stored
pub fn set_grant(entry: MlsGrantEntry) -> QueryResult<()> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.transaction(|conn| {
        use self::mls_grants::dsl::*;
        diesel::delete(
            mls_grants.filter(
                vni.eq(entry.vni as i32)
                    .and(client_id.eq(entry.client_id.clone())),
            ),
        )
        .execute(conn)?;
        diesel::insert_into(mls_grants)
            .values(entry)
            .execute(conn)?;
        Ok(())
    })
}

/// Returns the grant of a gateway for the group of a network.
///
/// # Arguments
/// * `grant_vni` - Tenant of the network
/// * `grant_client_id` - Identity of the gateway
///
/// # Returns
/// The grant, or `None` if the gateway has none
pub fn get_grant(grant_vni: u32, grant_client_id: &str) -> QueryResult<Option<MlsGrantEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::mls_grants::dsl::*;
    mls_grants
        .filter(vni.eq(grant_vni as i32).and(client_id.eq(grant_client_id)))
        .select(MlsGrantEntry::as_select())
        .first(&mut *conn)
        .optional()
}

/// Removes the grant of a gateway for the group of a network.
///
/// # Arguments
/// * `grant_vni` - Tenant of the network
/// * `grant_client_id` - Identity of the gateway
///
/// # Returns
/// The number of removed grants
pub fn delete_grant(grant_vni: u32, grant_client_id: &str) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::mls_grants::dsl::*;
    diesel::delete(mls_grants.filter(vni.eq(grant_vni as i32).and(client_id.eq(grant_client_id))))
        .execute(&mut *conn)
}
