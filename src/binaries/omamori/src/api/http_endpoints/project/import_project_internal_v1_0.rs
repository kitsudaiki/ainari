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
use chrono::Utc;
use uuid::Uuid;
use validator::Validate;

use crate::core::crypto_trait::CryptoModule;
use crate::core::key_wrapping::{WrapKey, unwrap_payload};
use crate::core::simple_crypto::SimpleCrypto;
use crate::database::public_key_table::{self, PublicKeyEntry};
use crate::database::secret_table::{self, SecretEntry};

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::project_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::functions::{create_ssh_key_fingerprint, is_valid_ssh_public_key};

#[api_operation(
    tag = "project",
    summary = "Import project",
    description = r###"Import the secrets and public-keys of a project from a backup.

The payloads of the secrets are unwrapped with `wrap_key` and stored encrypted by the omamori
again. The `wrap_key` is only used for this request and is never stored or logged. A wrong
`wrap_key` or a changed payload is rejected with `400`, before anything is changed.

The resources keep their UUIDs, unless the `id_map` gives them a new one, which is also applied to
the resources, which own the secrets. A UUID, which is used by another project, is rejected with
`409`. An existing secret or public-key of the project with the same UUID is rejected with `409`,
unless `force` is set, which overwrites it. All conflicts are checked, before anything is changed. The quota of the project is not checked here. Users, which are not
admin, can only import into their own project.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 403,
    error_code = 409,
    error_code = 500
)]
pub async fn import_project_internal(
    project_id: Path<String>,
    body: Json<OmamoriProjectImportReq>,
    context: UserContext,
) -> Result<Json<OmamoriProjectImportResp>, ErrorResponse> {
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;
    check_project_access(&project_id, &context)?;
    if context.is_read_only() {
        return Err(permission_denied_response());
    }
    let wrap_key = WrapKey::from_base64(&body.wrap_key)
        .map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;
    let new_uuid = |uuid: &Uuid| *body.id_map.get(uuid).unwrap_or(uuid);

    // unwrap all payloads and check all public-keys first, so a wrong wrap-key or a changed
    // backup is detected, before anything is changed
    let mut payloads = Vec::new();
    for secret in &body.secrets {
        let payload = unwrap_payload(&secret.wrapped_payload, &wrap_key, &secret.uuid)
            .map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;
        payloads.push(payload);
    }
    let mut fingerprints = Vec::new();
    for public_key in &body.public_keys {
        if !is_valid_ssh_public_key(&public_key.public_key) {
            return Err(ErrorResponse::BadRequest(format!(
                "Public-key '{}' is no valid ssh-public-key.",
                public_key.uuid
            )));
        }
        let fingerprint = create_ssh_key_fingerprint(&public_key.public_key).ok_or_else(|| {
            ErrorResponse::BadRequest(format!(
                "Failed to create the fingerprint of public-key '{}'.",
                public_key.uuid
            ))
        })?;
        fingerprints.push(fingerprint);
    }

    // check all conflicts, before anything is changed
    let secret_uuids: Vec<Uuid> = body.secrets.iter().map(|s| new_uuid(&s.uuid)).collect();
    let public_key_uuids: Vec<Uuid> = body.public_keys.iter().map(|k| new_uuid(&k.uuid)).collect();
    check_unique_uuids(secret_uuids.iter().chain(&public_key_uuids))?;
    let mut overwritten_secrets = Vec::new();
    for uuid in &secret_uuids {
        let existing = secret_table::find_secret(uuid)
            .map_err(|e| map_db_list_error("secrets", e, Some(&context)))?
            .map(|secret| secret.project_id);
        overwritten_secrets.push(check_import_target(
            existing.as_deref(),
            &project_id,
            body.force,
            "Secret",
            uuid,
        )?);
    }
    let mut overwritten_public_keys = Vec::new();
    for uuid in &public_key_uuids {
        let existing = public_key_table::find_public_key(uuid)
            .map_err(|e| map_db_list_error("public-keys", e, Some(&context)))?
            .map(|public_key| public_key.project_id);
        overwritten_public_keys.push(check_import_target(
            existing.as_deref(),
            &project_id,
            body.force,
            "Public-key",
            uuid,
        )?);
    }

    // store the secrets and their payloads
    let simple_crypto = SimpleCrypto::new();
    let mut imported_secrets = Vec::new();
    for ((secret, payload), (uuid, overwritten)) in body
        .secrets
        .iter()
        .zip(payloads)
        .zip(secret_uuids.into_iter().zip(overwritten_secrets))
    {
        secret_table::put_secret(SecretEntry {
            uuid,
            name: secret.name.clone(),
            owned_by: secret.owned_by.clone(),
            resource_uuid: secret.resource_uuid.as_ref().map(new_uuid),
            owner_id: context.user_id.clone(),
            project_id: project_id.clone(),
            created_at: Utc::now(),
            created_by: context.user_id.clone(),
            updated_at: Utc::now(),
            updated_by: context.user_id.clone(),
        })
        .map_err(|e| {
            map_db_write_error(
                &format!("import secret with UUID '{uuid}'"),
                e,
                Some(&context),
            )
        })?;
        simple_crypto
            .store(&uuid, &payload)
            .map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;

        imported_secrets.push(ImportedResource {
            source_uuid: secret.uuid,
            uuid,
            overwritten,
        });
    }

    // store the public-keys
    let mut imported_public_keys = Vec::new();
    for ((public_key, fingerprint), (uuid, overwritten)) in body
        .public_keys
        .iter()
        .zip(fingerprints)
        .zip(public_key_uuids.into_iter().zip(overwritten_public_keys))
    {
        public_key_table::put_public_key(PublicKeyEntry {
            uuid,
            name: public_key.name.clone(),
            public_key: public_key.public_key.clone(),
            fingerprint,
            owner_id: context.user_id.clone(),
            project_id: project_id.clone(),
            created_at: Utc::now(),
            created_by: context.user_id.clone(),
            updated_at: Utc::now(),
            updated_by: context.user_id.clone(),
        })
        .map_err(|e| {
            map_db_write_error(
                &format!("import public-key with UUID '{uuid}'"),
                e,
                Some(&context),
            )
        })?;

        imported_public_keys.push(ImportedResource {
            source_uuid: public_key.uuid,
            uuid,
            overwritten,
        });
    }

    Ok(Json(OmamoriProjectImportResp {
        project_id: project_id.clone(),
        secrets: imported_secrets,
        public_keys: imported_public_keys,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::http_endpoints::project::export_project_internal_v1_0::export_project_internal;
    use crate::api::http_endpoints::project::test_utils::*;
    use ainari_common::enums::ProjectRole;
    use serial_test::serial;
    use std::collections::BTreeMap;

    async fn export(project: &str) -> OmamoriProjectExport {
        let body = OmamoriProjectExportReq {
            wrap_key: wrap_key(1),
            backup_uuid: None,
        };
        export_project_internal(
            Path::from(project.to_string()),
            Json(body),
            context(project, true, ProjectRole::Member),
        )
        .await
        .unwrap()
        .into_inner()
    }

    async fn import(
        project: &str,
        exported: &OmamoriProjectExport,
        key: u8,
        force: bool,
        id_map: BTreeMap<Uuid, Uuid>,
        context: &UserContext,
    ) -> Result<OmamoriProjectImportResp, ErrorResponse> {
        let body = OmamoriProjectImportReq {
            wrap_key: wrap_key(key),
            secrets: exported.secrets.clone(),
            public_keys: exported.public_keys.clone(),
            force,
            id_map,
        };
        import_project_internal(Path::from(project.to_string()), Json(body), context.clone())
            .await
            .map(|resp| resp.into_inner())
    }

    fn admin(project: &str) -> UserContext {
        context(project, true, ProjectRole::Member)
    }

    #[actix_rt::test]
    #[serial]
    async fn test_round_trip_into_other_project() {
        let source = new_project();
        let image = Uuid::new_v4();
        let user_secret = add_secret(&source, "user", None, "user-payload");
        let image_secret = add_secret(&source, "image", Some(image), "image-payload");
        let public_key = add_public_key(&source);
        let exported = export(&source).await;

        // one secret and the image get new UUIDs, the rest keeps its UUIDs
        let target = new_project();
        let new_secret = Uuid::new_v4();
        let new_image = Uuid::new_v4();
        let id_map = BTreeMap::from([(image_secret, new_secret), (image, new_image)]);
        delete_project_entries(&source);

        let resp = import(&target, &exported, 1, false, id_map, &admin(&target))
            .await
            .unwrap();
        assert_eq!(resp.project_id, target);
        assert!(resp.secrets.contains(&ImportedResource {
            source_uuid: image_secret,
            uuid: new_secret,
            overwritten: false,
        }));
        assert!(resp.secrets.contains(&ImportedResource {
            source_uuid: user_secret,
            uuid: user_secret,
            overwritten: false,
        }));

        // payloads, owners and the project are restored, the fingerprint is calculated again
        assert_eq!(payload_of(&user_secret), "user-payload");
        assert_eq!(payload_of(&new_secret), "image-payload");
        let restored = secret_table::find_secret(&new_secret).unwrap().unwrap();
        assert_eq!(restored.project_id, target);
        assert_eq!(restored.owned_by, "image");
        assert_eq!(restored.resource_uuid, Some(new_image));
        let restored_key = public_key_table::find_public_key(&public_key)
            .unwrap()
            .unwrap();
        assert_eq!(restored_key.project_id, target);
        assert_eq!(
            Some(restored_key.fingerprint),
            create_ssh_key_fingerprint(PUBLIC_KEY)
        );

        delete_project_entries(&target);
    }

    #[actix_rt::test]
    #[serial]
    async fn test_import_into_same_project() {
        let project = new_project();
        let secret = add_secret(&project, "user", None, "payload");
        add_public_key(&project);
        let mut exported = export(&project).await;
        exported.secrets[0].name = "renamed".to_string();
        let context = context(&project, false, ProjectRole::Admin);

        // existing resources are only overwritten with force
        assert!(matches!(
            import(&project, &exported, 1, false, BTreeMap::new(), &context).await,
            Err(ErrorResponse::Conflict(_))
        ));
        assert_eq!(
            secret_table::find_secret(&secret).unwrap().unwrap().name,
            "secret-user"
        );
        let resp = import(&project, &exported, 1, true, BTreeMap::new(), &context)
            .await
            .unwrap();
        assert!(resp.secrets[0].overwritten);
        assert!(resp.public_keys[0].overwritten);
        assert_eq!(
            secret_table::find_secret(&secret).unwrap().unwrap().name,
            "renamed"
        );
        assert_eq!(payload_of(&secret), "payload");

        // a deleted secret is imported again with its UUID, also without force
        assert!(secret_table::delete_secret(&secret, &context).is_ok());
        SimpleCrypto::new().delete(&secret).unwrap();
        exported.public_keys.clear();
        let resp = import(&project, &exported, 1, false, BTreeMap::new(), &context)
            .await
            .unwrap();
        assert!(!resp.secrets[0].overwritten);
        assert!(secret_table::find_secret(&secret).unwrap().is_some());
        assert_eq!(payload_of(&secret), "payload");

        delete_project_entries(&project);
    }

    #[actix_rt::test]
    #[serial]
    async fn test_import_errors() {
        let project = new_project();
        let secret = add_secret(&project, "user", None, "payload");
        let exported = export(&project).await;
        let target = new_project();
        let new_uuid = Uuid::new_v4();
        let id_map = || BTreeMap::from([(secret, new_uuid)]);

        // a wrong wrap-key or a changed payload change nothing
        assert!(matches!(
            import(&target, &exported, 2, false, id_map(), &admin(&target)).await,
            Err(ErrorResponse::BadRequest(_))
        ));
        let mut changed = exported.clone();
        changed.secrets[0].uuid = Uuid::new_v4();
        assert!(matches!(
            import(
                &target,
                &changed,
                1,
                false,
                BTreeMap::new(),
                &admin(&target)
            )
            .await,
            Err(ErrorResponse::BadRequest(_))
        ));
        assert!(secret_table::find_secret(&new_uuid).unwrap().is_none());

        // the UUID of another project is never overwritten, also not with force
        assert!(matches!(
            import(
                &target,
                &exported,
                1,
                true,
                BTreeMap::new(),
                &admin(&target)
            )
            .await,
            Err(ErrorResponse::Conflict(_))
        ));
        assert_eq!(
            secret_table::find_secret(&secret)
                .unwrap()
                .unwrap()
                .project_id,
            project
        );

        // an id_map, which maps two resources to the same UUID
        let mut twice = exported.clone();
        twice.secrets.push(exported.secrets[0].clone());
        assert!(matches!(
            import(&target, &twice, 1, false, id_map(), &admin(&target)).await,
            Err(ErrorResponse::BadRequest(_))
        ));

        // users can only import into their own project, observers not at all
        let member = context(&project, false, ProjectRole::Member);
        assert!(matches!(
            import(&target, &exported, 1, false, id_map(), &member).await,
            Err(ErrorResponse::Forbidden(_))
        ));
        let observer = context(&target, false, ProjectRole::Observer);
        assert!(matches!(
            import(&target, &exported, 1, false, id_map(), &observer).await,
            Err(ErrorResponse::Forbidden(_))
        ));
        assert!(secret_table::find_secret(&new_uuid).unwrap().is_none());

        delete_project_entries(&project);
        delete_project_entries(&target);
    }
}
