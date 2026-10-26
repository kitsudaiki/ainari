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
use std::path::{Component, Path as FilePath};
use uuid::Uuid;
use validator::Validate;

use crate::database::host_table;
use crate::database::image_table::{self, ImageEntry};

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::image_structs::{
    ImageStatus, RyokanProjectImportReq, RyokanProjectImportResp,
};
use ainari_api_structs::project_structs::ImportedResource;
use ainari_api_structs::user_context::UserContext;

/// Directory on the onsen, which contains the files of the images.
const IMAGE_DIRECTORY: &str = "images";

#[api_operation(
    tag = "project",
    summary = "Import project",
    description = r###"Import the images and snapshots of a project from a backup.

The files of the images must already be copied to a normal onsen, for example with
`POST /v1alpha/backup/{backup_uuid}/copy/internal`. Each image is registered as `READY` with the
location of its copy (`onsen_address` and `file_path` within `images/`). A backup-target or an
unknown onsen is rejected with `400`.

The images keep their UUIDs, unless the `id_map` gives them a new one, which is also applied to
their secrets. A UUID, which is used by another project, is rejected with `409`. An existing
image of the project with the same UUID is rejected with `409`, unless `force` is set, which
overwrites it and deletes its old file, if the location changed. All conflicts are checked, before
anything is changed. The quota of the project is not checked here.
Users, which are not admin, can only import into their own project.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 403,
    error_code = 409,
    error_code = 500
)]
pub async fn import_project_internal(
    project_id: Path<String>,
    body: Json<RyokanProjectImportReq>,
    context: UserContext,
) -> Result<Json<RyokanProjectImportResp>, ErrorResponse> {
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;
    check_project_access(&project_id, &context)?;
    if context.is_read_only() {
        return Err(permission_denied_response());
    }
    let new_uuid = |uuid: &Uuid| *body.id_map.get(uuid).unwrap_or(uuid);

    // check the locations of all files first
    for import in &body.images {
        check_image_location(&import.onsen_address, &import.file_path, &context)?;
    }

    // check all conflicts, before anything is changed
    let image_uuids: Vec<Uuid> = body
        .images
        .iter()
        .map(|i| new_uuid(&i.image.uuid))
        .collect();
    check_unique_uuids(&image_uuids)?;
    let mut existing_images = Vec::new();
    for uuid in &image_uuids {
        let existing = image_table::find_image(uuid)
            .map_err(|e| map_db_list_error("images", e, Some(&context)))?;
        check_import_target(
            existing.as_ref().map(|image| image.project_id.as_str()),
            &project_id,
            body.force,
            "Image",
            uuid,
        )?;
        existing_images.push(existing);
    }

    let mut imported = Vec::new();
    for ((import, uuid), existing) in body.images.iter().zip(image_uuids).zip(existing_images) {
        let image = &import.image;
        image_table::put_image(ImageEntry {
            uuid,
            name: image.name.clone(),
            onsen_address: import.onsen_address.clone(),
            file_path: import.file_path.clone(),
            file_type: image.file_type.clone(),
            file_size: i64::try_from(image.file_size).unwrap_or(i64::MAX),
            image_status: ImageStatus::Ready.to_string(),
            secret_uuid: new_uuid(&image.secret_uuid),
            is_snapshot: image.is_snapshot,
            owner_id: context.user_id.clone(),
            project_id: project_id.clone(),
            created_at: Utc::now(),
            created_by: context.user_id.clone(),
            updated_at: Utc::now(),
            updated_by: context.user_id.clone(),
        })
        .map_err(|e| {
            map_db_write_error(
                &format!("import image with UUID '{uuid}'"),
                e,
                Some(&context),
            )
        })?;

        // the file of the overwritten image is not used anymore. A failure is only logged,
        // because the image itself was already imported.
        if let Some(old) = &existing
            && (old.onsen_address != import.onsen_address || old.file_path != import.file_path)
            && delete_file_from_onsen(&old.onsen_address, &old.file_path, Some(&context))
                .await
                .is_err()
        {
            log::warn!("Failed to delete the old file of the overwritten image '{uuid}'");
        }

        imported.push(ImportedResource {
            source_uuid: image.uuid,
            uuid,
            overwritten: existing.is_some(),
        });
    }

    Ok(Json(RyokanProjectImportResp {
        project_id: project_id.clone(),
        images: imported,
    }))
}

/// Checks the location of the file of an imported image.
///
/// # Arguments
///
/// * `onsen_address` - Address of the onsen, which stores the file
/// * `file_path` - Path of the file on the onsen
/// * `context` - User-context of the request
///
/// # Returns
///
/// * `Ok(())` - The file is within `images/` of a registered onsen, which is no backup-target
/// * `Err(ErrorResponse::BadRequest)` - The location is invalid
fn check_image_location(
    onsen_address: &str,
    file_path: &str,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    if !is_image_path(file_path) {
        return Err(ErrorResponse::BadRequest(format!(
            "'{file_path}' is no valid path of a file within '{IMAGE_DIRECTORY}/'."
        )));
    }
    match host_table::get_host_by_address(&onsen_address.to_string(), context) {
        Ok(host) if !host.is_backup_target => Ok(()),
        Ok(_) => Err(ErrorResponse::BadRequest(format!(
            "Onsen '{onsen_address}' is a backup-target and can't store images."
        ))),
        Err(ainari_common::enums::DbError::NotFound) => Err(ErrorResponse::BadRequest(format!(
            "Onsen '{onsen_address}' is not registered."
        ))),
        Err(e) => Err(map_db_id_get_delete_error(
            "onsen-host",
            onsen_address,
            e,
            Some(context),
        )),
    }
}

/// Checks, that a path points to a file within the directory of the images.
fn is_image_path(file_path: &str) -> bool {
    let components: Vec<Component> = FilePath::new(file_path).components().collect();
    components.len() >= 2
        && components[0] == Component::Normal(IMAGE_DIRECTORY.as_ref())
        && components
            .iter()
            .all(|component| matches!(component, Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::http_endpoints::project::export_project_internal_v1_0::export_project_internal;
    use crate::database::db_handle;
    use crate::database::host_table::HostEntry;
    use ainari_api_structs::image_structs::{
        RyokanImageExport, RyokanImageImport, RyokanProjectExport,
    };
    use ainari_common::enums::ProjectRole;
    use diesel::prelude::*;
    use serial_test::serial;
    use std::collections::BTreeMap;

    fn context(project: &str, is_admin: bool, role: ProjectRole) -> UserContext {
        UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: project.to_string(),
            is_admin: is_admin.to_string(),
            project_role: role.to_string(),
        }
    }

    fn new_project() -> String {
        format!("test-project-{}", Uuid::new_v4())
    }

    fn add_host(is_backup_target: bool) -> HostEntry {
        let host = HostEntry {
            uuid: Uuid::new_v4(),
            name: "onsen".to_string(),
            address: format!("http://{}:50051", Uuid::new_v4()),
            is_backup_target,
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
        };
        host_table::add_host(host.clone()).unwrap();
        host
    }

    fn add_image(project: &str, onsen: &str, status: ImageStatus, is_snapshot: bool) -> Uuid {
        let uuid = Uuid::new_v4();
        image_table::add_image(ImageEntry {
            uuid,
            name: "image".to_string(),
            onsen_address: onsen.to_string(),
            file_path: format!("images/{uuid}"),
            file_type: "qcow2".to_string(),
            file_size: 4096,
            image_status: status.to_string(),
            secret_uuid: Uuid::new_v4(),
            is_snapshot,
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

    fn cleanup(projects: &[&str], hosts: &[&HostEntry]) {
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        for project in projects {
            use crate::database::image_table::images::dsl::*;
            let _ = diesel::delete(images.filter(project_id.eq(*project))).execute(&mut *conn);
        }
        for host in hosts {
            use crate::database::host_table::hosts::dsl::*;
            let _ =
                diesel::delete(hosts.filter(uuid.eq(host.uuid.to_string()))).execute(&mut *conn);
        }
    }

    async fn export(project: &str) -> RyokanProjectExport {
        export_project_internal(
            Path::from(project.to_string()),
            context(project, true, ProjectRole::Member),
        )
        .await
        .unwrap()
        .into_inner()
    }

    /// Imports the exported images like they were copied to the given onsen.
    async fn import(
        project: &str,
        images: &[RyokanImageExport],
        onsen: &str,
        force: bool,
        id_map: BTreeMap<Uuid, Uuid>,
        context: &UserContext,
    ) -> Result<RyokanProjectImportResp, ErrorResponse> {
        let body = RyokanProjectImportReq {
            images: images
                .iter()
                .map(|image| RyokanImageImport {
                    image: image.clone(),
                    onsen_address: onsen.to_string(),
                    file_path: format!("images/{}", Uuid::new_v4()),
                })
                .collect(),
            force,
            id_map,
        };
        import_project_internal(Path::from(project.to_string()), Json(body), context.clone())
            .await
            .map(|resp| resp.into_inner())
    }

    #[actix_rt::test]
    #[serial]
    async fn test_export_project() {
        let host = add_host(false);
        let project = new_project();
        let image = add_image(&project, &host.address, ImageStatus::Ready, false);
        let snapshot = add_image(&project, &host.address, ImageStatus::Ready, true);
        add_image(&project, &host.address, ImageStatus::Uploading, false);
        let other_project = new_project();
        add_image(&other_project, &host.address, ImageStatus::Ready, false);

        // only the complete images and snapshots of the project are exported
        let exported = export(&project).await;
        let uuids: Vec<Uuid> = exported.images.iter().map(|i| i.uuid).collect();
        assert_eq!(uuids.len(), 2);
        assert!(uuids.contains(&image) && uuids.contains(&snapshot));
        let exported_snapshot = exported.images.iter().find(|i| i.uuid == snapshot).unwrap();
        assert!(exported_snapshot.is_snapshot);
        assert_eq!(exported_snapshot.file_type, "qcow2");
        assert_eq!(exported_snapshot.file_size, 4096);

        // users can only export their own project
        let other = context(&other_project, false, ProjectRole::Admin);
        let result = export_project_internal(Path::from(project.clone()), other).await;
        assert!(matches!(result, Err(ErrorResponse::Forbidden(_))));

        cleanup(&[&project, &other_project], &[&host]);
    }

    #[actix_rt::test]
    #[serial]
    async fn test_round_trip_into_other_project() {
        let host = add_host(false);
        let source = new_project();
        let image = add_image(&source, &host.address, ImageStatus::Ready, true);
        let exported = export(&source).await;
        let secret = exported.images[0].secret_uuid;

        // the image and its secret get new UUIDs
        let target = new_project();
        let (new_image, new_secret) = (Uuid::new_v4(), Uuid::new_v4());
        let id_map = BTreeMap::from([(image, new_image), (secret, new_secret)]);
        let admin = context(&target, true, ProjectRole::Member);
        let resp = import(
            &target,
            &exported.images,
            &host.address,
            false,
            id_map,
            &admin,
        )
        .await
        .unwrap();
        assert_eq!(
            resp.images,
            vec![ImportedResource {
                source_uuid: image,
                uuid: new_image,
                overwritten: false,
            }]
        );

        let restored = image_table::find_image(&new_image).unwrap().unwrap();
        assert_eq!(restored.project_id, target);
        assert_eq!(restored.secret_uuid, new_secret);
        assert_eq!(restored.onsen_address, host.address);
        assert!(restored.file_path.starts_with("images/"));
        assert_eq!(restored.image_status, "READY");
        assert!(restored.is_snapshot);

        cleanup(&[&source, &target], &[&host]);
    }

    #[actix_rt::test]
    #[serial]
    async fn test_import_into_same_project() {
        let host = add_host(false);
        let project = new_project();
        let image = add_image(&project, &host.address, ImageStatus::Ready, false);
        let mut exported = export(&project).await;
        exported.images[0].name = "renamed".to_string();
        let context = context(&project, false, ProjectRole::Admin);
        let no_map = BTreeMap::new;

        // an existing image is only overwritten with force
        assert!(matches!(
            import(
                &project,
                &exported.images,
                &host.address,
                false,
                no_map(),
                &context
            )
            .await,
            Err(ErrorResponse::Conflict(_))
        ));
        let resp = import(
            &project,
            &exported.images,
            &host.address,
            true,
            no_map(),
            &context,
        )
        .await
        .unwrap();
        assert!(resp.images[0].overwritten);
        let overwritten = image_table::find_image(&image).unwrap().unwrap();
        assert_eq!(overwritten.name, "renamed");
        assert_ne!(overwritten.file_path, format!("images/{image}"));

        // a deleted image is imported again with its UUID, also without force
        assert!(image_table::delete_image(&image, &context).is_ok());
        let resp = import(
            &project,
            &exported.images,
            &host.address,
            false,
            no_map(),
            &context,
        )
        .await
        .unwrap();
        assert!(!resp.images[0].overwritten);
        assert!(image_table::find_image(&image).unwrap().is_some());

        cleanup(&[&project], &[&host]);
    }

    #[actix_rt::test]
    #[serial]
    async fn test_import_errors() {
        let host = add_host(false);
        let backup_target = add_host(true);
        let project = new_project();
        let image = add_image(&project, &host.address, ImageStatus::Ready, false);
        let exported = export(&project).await;
        let target = new_project();
        let admin = context(&target, true, ProjectRole::Member);
        let new_uuid = Uuid::new_v4();
        let id_map = || BTreeMap::from([(image, new_uuid)]);

        // the UUID of another project is never overwritten, also not with force
        assert!(matches!(
            import(
                &target,
                &exported.images,
                &host.address,
                true,
                BTreeMap::new(),
                &admin
            )
            .await,
            Err(ErrorResponse::Conflict(_))
        ));
        assert_eq!(
            image_table::find_image(&image).unwrap().unwrap().project_id,
            project
        );

        // files on a backup-target, an unknown onsen or outside of the images are rejected
        for onsen in [backup_target.address.as_str(), "http://unknown:50051"] {
            assert!(matches!(
                import(&target, &exported.images, onsen, false, id_map(), &admin).await,
                Err(ErrorResponse::BadRequest(_))
            ));
        }
        let body = RyokanProjectImportReq {
            images: vec![RyokanImageImport {
                image: exported.images[0].clone(),
                onsen_address: host.address.clone(),
                file_path: "backups/other/disk.enc".to_string(),
            }],
            force: false,
            id_map: id_map(),
        };
        let result =
            import_project_internal(Path::from(target.clone()), Json(body), admin.clone()).await;
        assert!(matches!(result, Err(ErrorResponse::BadRequest(_))));

        // users can only import into their own project, observers not at all
        let member = context(&project, false, ProjectRole::Member);
        assert!(matches!(
            import(
                &target,
                &exported.images,
                &host.address,
                false,
                id_map(),
                &member
            )
            .await,
            Err(ErrorResponse::Forbidden(_))
        ));
        let observer = context(&target, false, ProjectRole::Observer);
        assert!(matches!(
            import(
                &target,
                &exported.images,
                &host.address,
                false,
                id_map(),
                &observer
            )
            .await,
            Err(ErrorResponse::Forbidden(_))
        ));
        assert!(image_table::find_image(&new_uuid).unwrap().is_none());

        cleanup(&[&project, &target], &[&host, &backup_target]);
    }

    #[test]
    fn test_is_image_path() {
        assert!(is_image_path("images/abc"));
        assert!(is_image_path("images/sub/abc"));
        for path in [
            "images",
            "images/",
            "backups/x/disk.enc",
            "/images/abc",
            "images/../x",
            "./images/x",
            "",
        ] {
            assert!(!is_image_path(path), "{path}");
        }
    }
}
