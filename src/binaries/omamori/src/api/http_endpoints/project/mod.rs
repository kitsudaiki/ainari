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

pub mod export_project_internal_v1_0;
pub mod get_project_resource_count_internal_v1_0;
pub mod import_project_internal_v1_0;

/// Helpers for the tests of the export and import of projects.
#[cfg(test)]
pub(crate) mod test_utils {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use chrono::Utc;
    use diesel::prelude::*;
    use uuid::Uuid;

    use crate::core::crypto_trait::CryptoModule;
    use crate::core::simple_crypto::SimpleCrypto;
    use crate::database::db_handle;
    use crate::database::public_key_table::{self, PublicKeyEntry};
    use crate::database::secret_table::{self, SecretEntry};

    use ainari_api_structs::user_context::UserContext;
    use ainari_common::enums::ProjectRole;
    use ainari_common::secret::Secret;

    /// A valid ssh-public-key, from which a fingerprint can be created
    pub const PUBLIC_KEY: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIKjMONeVqsXz3KbidQwmPCY9pwZYiS/HutS6+DLhv1Cd test@ainari";

    /// Returns a base64-encoded wrap-key, which consists only of the given byte.
    pub fn wrap_key(byte: u8) -> Secret {
        Secret::from(STANDARD.encode([byte; 32]))
    }

    /// Returns a unique project-ID, so the tests don't see the entries of each other.
    pub fn new_project() -> String {
        format!("test-project-{}", Uuid::new_v4())
    }

    pub fn context(project: &str, is_admin: bool, role: ProjectRole) -> UserContext {
        UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: project.to_string(),
            is_admin: is_admin.to_string(),
            project_role: role.to_string(),
        }
    }

    /// Adds a secret together with its payload to a project.
    pub fn add_secret(
        project: &str,
        owned_by: &str,
        resource_uuid: Option<Uuid>,
        payload: &str,
    ) -> Uuid {
        let uuid = Uuid::new_v4();
        secret_table::add_secret(SecretEntry {
            uuid,
            name: format!("secret-{owned_by}"),
            owned_by: owned_by.to_string(),
            resource_uuid,
            owner_id: "test-user".to_string(),
            project_id: project.to_string(),
            created_at: Utc::now(),
            created_by: "test-user".to_string(),
            updated_at: Utc::now(),
            updated_by: "test-user".to_string(),
        })
        .unwrap();
        SimpleCrypto::new()
            .store(&uuid, &Secret::from(payload.to_string()))
            .unwrap();
        uuid
    }

    /// Adds a public-key to a project.
    pub fn add_public_key(project: &str) -> Uuid {
        let uuid = Uuid::new_v4();
        public_key_table::add_public_key(PublicKeyEntry {
            uuid,
            name: "public-key".to_string(),
            public_key: PUBLIC_KEY.to_string(),
            fingerprint: "fingerprint".to_string(),
            owner_id: "test-user".to_string(),
            project_id: project.to_string(),
            created_at: Utc::now(),
            created_by: "test-user".to_string(),
            updated_at: Utc::now(),
            updated_by: "test-user".to_string(),
        })
        .unwrap();
        uuid
    }

    /// Reads the plain payload of a secret.
    pub fn payload_of(uuid: &Uuid) -> String {
        SimpleCrypto::new()
            .retrieve(uuid)
            .unwrap()
            .reveal()
            .to_string()
    }

    /// Removes all secrets, payloads and public-keys of a project from the database.
    pub fn delete_project_entries(project: &str) {
        let secrets: Vec<SecretEntry> = {
            use crate::database::secret_table::secrets::dsl::*;
            let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
            secrets
                .filter(project_id.eq(project))
                .select(SecretEntry::as_select())
                .load(&mut *conn)
                .unwrap()
        };
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        for secret in secrets {
            use crate::database::simple_crypto_table::simple_crypto::dsl::*;
            let _ = diesel::delete(simple_crypto.filter(secret_uuid.eq(secret.uuid.to_string())))
                .execute(&mut *conn);
        }
        {
            use crate::database::secret_table::secrets::dsl::*;
            let _ = diesel::delete(secrets.filter(project_id.eq(project))).execute(&mut *conn);
        }
        use crate::database::public_key_table::public_keys::dsl::*;
        let _ = diesel::delete(public_keys.filter(project_id.eq(project))).execute(&mut *conn);
    }
}
