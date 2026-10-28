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

use actix_web::web::Json;
use actix_web::web::Path;
use apistos::api_operation;

use crate::core::crypto_trait::CryptoModule;
use crate::core::key_wrapping::{WrapKey, wrap_payload};
use crate::core::simple_crypto::SimpleCrypto;
use crate::database::public_key_table;
use crate::database::secret_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::exports::{
    OmamoriProjectExport, OmamoriProjectExportReq, PublicKeyExport, SecretExport,
};
use ainari_api_structs::secret_structs::SECRET_OWNER_BACKUP;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "project",
    summary = "Export project",
    description = r###"Export the secrets and public-keys of a project for a backup.

The payloads of the secrets never leave the omamori in plain text: each payload is wrapped with
`wrap_key` (AES-256-GCM with a new nonce and the UUID of the secret as additional data). The
`wrap_key` is only used for this request and is never stored or logged. The secrets of backups are
not exported, except the ones of the backup `backup_uuid`, which encrypt its disks. Users, which are
not admin, can only export their own project.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 403,
    error_code = 500
)]
pub async fn export_project_internal(
    project_id: Path<String>,
    body: Json<OmamoriProjectExportReq>,
    context: UserContext,
) -> Result<Json<OmamoriProjectExport>, ErrorResponse> {
    check_project_access(&project_id, &context)?;
    let wrap_key = WrapKey::from_base64(&body.wrap_key)
        .map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;

    let secrets = secret_table::list_secrets_of_project(&project_id)
        .map_err(|e| map_db_list_error("secrets", e, Some(&context)))?;
    let simple_crypto = SimpleCrypto::new();
    let mut exported_secrets = Vec::new();
    for secret in secrets {
        // the secrets of other backups don't belong to this backup
        let is_own_backup = body.backup_uuid.is_some() && secret.resource_uuid == body.backup_uuid;
        if secret.owned_by == SECRET_OWNER_BACKUP && !is_own_backup {
            continue;
        }

        let payload = simple_crypto
            .retrieve(&secret.uuid)
            .map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;
        let wrapped_payload = wrap_payload(&payload, &wrap_key, &secret.uuid)
            .map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;

        exported_secrets.push(SecretExport {
            uuid: secret.uuid,
            name: secret.name,
            owned_by: secret.owned_by,
            resource_uuid: secret.resource_uuid,
            wrapped_payload,
        });
    }

    let public_keys = public_key_table::list_public_keys_of_project(&project_id)
        .map_err(|e| map_db_list_error("public-keys", e, Some(&context)))?;
    let exported_public_keys = public_keys
        .into_iter()
        .map(|public_key| PublicKeyExport {
            uuid: public_key.uuid,
            name: public_key.name,
            public_key: public_key.public_key,
        })
        .collect();

    Ok(Json(OmamoriProjectExport {
        secrets: exported_secrets,
        public_keys: exported_public_keys,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::http_endpoints::project::test_utils::*;
    use crate::core::key_wrapping::unwrap_payload;
    use ainari_common::enums::ProjectRole;
    use ainari_common::secret::Secret;
    use serial_test::serial;
    use uuid::Uuid;

    async fn export(
        project: &str,
        backup_uuid: Option<Uuid>,
        context: &UserContext,
    ) -> Result<OmamoriProjectExport, ErrorResponse> {
        let body = OmamoriProjectExportReq {
            wrap_key: wrap_key(1),
            backup_uuid,
        };
        export_project_internal(Path::from(project.to_string()), Json(body), context.clone())
            .await
            .map(|resp| resp.into_inner())
    }

    #[actix_rt::test]
    #[serial]
    async fn test_export_project() {
        let project = new_project();
        let backup = Uuid::new_v4();
        let user_secret = add_secret(&project, "user", None, "plain-user-payload");
        let image_secret = add_secret(&project, "image", Some(Uuid::new_v4()), "plain-image");
        let own_backup = add_secret(&project, SECRET_OWNER_BACKUP, Some(backup), "plain-own");
        let other_backup = add_secret(
            &project,
            SECRET_OWNER_BACKUP,
            Some(Uuid::new_v4()),
            "plain-other",
        );
        let public_key = add_public_key(&project);
        let other_project = new_project();
        let foreign_secret = add_secret(&other_project, "user", None, "plain-foreign");

        let context = context(&project, false, ProjectRole::Member);
        let exported = export(&project, Some(backup), &context).await.unwrap();

        // only the secrets of the project and of the own backup are exported
        let uuids: Vec<Uuid> = exported.secrets.iter().map(|s| s.uuid).collect();
        assert_eq!(uuids.len(), 3);
        for uuid in [user_secret, image_secret, own_backup] {
            assert!(uuids.contains(&uuid));
        }
        assert!(!uuids.contains(&other_backup));
        assert!(!uuids.contains(&foreign_secret));
        assert_eq!(exported.public_keys.len(), 1);
        assert_eq!(exported.public_keys[0].uuid, public_key);
        assert_eq!(exported.public_keys[0].public_key, PUBLIC_KEY);

        // no plain payload leaves the omamori, but all can be unwrapped with the wrap-key
        let json = serde_json::to_string(&exported).unwrap();
        assert!(!json.contains("plain-"));
        let key = WrapKey::from_base64(&wrap_key(1)).unwrap();
        for secret in &exported.secrets {
            let payload = unwrap_payload(&secret.wrapped_payload, &key, &secret.uuid).unwrap();
            assert_eq!(payload.reveal(), payload_of(&secret.uuid));
        }

        // without a backup, no secret of a backup is exported
        let exported = export(&project, None, &context).await.unwrap();
        assert_eq!(exported.secrets.len(), 2);
        assert!(
            exported
                .secrets
                .iter()
                .all(|s| s.owned_by != SECRET_OWNER_BACKUP)
        );

        delete_project_entries(&project);
        delete_project_entries(&other_project);
    }

    #[actix_rt::test]
    #[serial]
    async fn test_export_project_errors() {
        let project = new_project();

        // users can only export their own project
        let other = context(&new_project(), false, ProjectRole::Admin);
        assert!(matches!(
            export(&project, None, &other).await,
            Err(ErrorResponse::Forbidden(_))
        ));

        // invalid wrap-key
        let body = OmamoriProjectExportReq {
            wrap_key: Secret::from("no-key".to_string()),
            backup_uuid: None,
        };
        let result = export_project_internal(
            Path::from(project.clone()),
            Json(body),
            context(&project, false, ProjectRole::Member),
        )
        .await;
        assert!(matches!(result, Err(ErrorResponse::BadRequest(_))));
    }

    #[test]
    fn test_wrap_key_is_not_logged() {
        let key = wrap_key(1);
        let body = OmamoriProjectExportReq {
            wrap_key: key.clone(),
            backup_uuid: None,
        };
        // the request can be logged with debug, but never shows the key
        assert!(!format!("{body:?}").contains(key.reveal()));
    }
}
