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

use actix_web::web::JsonConfig;
use apistos::web::{Scope, delete, get, post, put, resource, scope};

use ainari_api::endpoints::*;
use ainari_api_structs::backup_structs::MAX_BACKUP_FILE_MESSAGE_SIZE;

use crate::api::http_endpoints::backup::*;
use crate::api::http_endpoints::image::*;
use crate::api::http_endpoints::onsen_host::*;
use crate::api::http_endpoints::project::*;

/// Builds the `/v1alpha`-scope with all endpoints of the ryokan.
///
/// # Returns
///
/// The scope, which is registered on the http-server.
pub fn v1alpha_routes() -> Scope {
    scope("/v1alpha")
        .service(
            scope("/version").service(resource("").route(get().to(get_version_v1_0::get_version))),
        )
        .service(
            scope("/is_ready")
                .service(resource("").route(get().to(is_ready_v1_0::get_ready_status))),
        )
        .service(
            scope("/image")
                .service(
                    resource("/internal").route(post().to(init_image_internal_v1_0::init_image)),
                )
                .service(resource("/count").route(get().to(get_image_count_v1_0::get_image_count)))
                .service(
                    resource("/{image_uuid}")
                        .route(get().to(get_image_v1_0::get_image))
                        .route(delete().to(delete_image_v1_0::delete_image)),
                )
                .service(
                    resource("/{image_uuid}/internal")
                        .route(get().to(get_image_internal_v1_0::get_image_internal))
                        .route(put().to(update_image_internal_v1_0::update_image_internal)),
                )
                .service(
                    resource("")
                        .route(get().to(list_image_v1_0::list_image))
                        .route(post().to(create_image_v1_0::upload_binary)),
                ),
        )
        .service(
            scope("/host")
                .service(
                    resource("/internal")
                        .route(post().to(register_host_internal_v1_0::register_host_internal)),
                )
                .service(resource("/admin").route(get().to(list_host_admin_v1_0::list_host_admin)))
                .service(
                    resource("/{host_uuid}/admin")
                        .route(get().to(get_host_admin_v1_0::get_host_admin))
                        .route(delete().to(delete_host_admin_v1_0::delete_host_admin)),
                )
                .service(
                    resource("/{host_uuid}/backup_target/admin")
                        .route(put().to(set_backup_target_admin_v1_0::set_backup_target_admin)),
                ),
        )
        .service(
            scope("/backup")
                .service(
                    resource("/internal")
                        .route(post().to(create_backup_internal_v1_0::create_backup_internal))
                        .route(get().to(list_backup_internal_v1_0::list_backup_internal)),
                )
                // registered before "/{backup_uuid}/internal", which would match it as well
                .service(
                    resource("/register/internal")
                        .route(post().to(register_backup_internal_v1_0::register_backup_internal)),
                )
                .service(
                    resource("/{backup_uuid}/file/internal")
                        .app_data(JsonConfig::default().limit(MAX_BACKUP_FILE_MESSAGE_SIZE))
                        .route(get().to(read_backup_file_internal_v1_0::read_backup_file_internal))
                        .route(
                            put().to(write_backup_file_internal_v1_0::write_backup_file_internal),
                        ),
                )
                .service(resource("/{backup_uuid}/file_info/internal").route(
                    get().to(get_backup_file_info_internal_v1_0::get_backup_file_info_internal),
                ))
                .service(
                    resource("/{backup_uuid}/copy/internal").route(
                        post().to(copy_backup_file_internal_v1_0::copy_backup_file_internal),
                    ),
                )
                .service(resource("/{backup_uuid}/disk/internal").route(
                    post().to(prepare_backup_disk_internal_v1_0::prepare_backup_disk_internal),
                ))
                .service(
                    resource("/{backup_uuid}/internal")
                        .route(get().to(get_backup_internal_v1_0::get_backup_internal))
                        .route(delete().to(delete_backup_internal_v1_0::delete_backup_internal)),
                )
                .service(
                    resource("/{backup_uuid}/finalize/internal")
                        .route(post().to(finalize_backup_internal_v1_0::finalize_backup_internal)),
                ),
        )
        .service(
            scope("/backup_target")
                .service(resource("/{host_uuid}/scan/internal").route(
                    get().to(scan_backup_target_internal_v1_0::scan_backup_target_internal),
                ))
                .service(resource("/{host_uuid}/manifest/internal").route(get().to(
                    read_unregistered_manifest_internal_v1_0::read_unregistered_manifest_internal,
                ))),
        )
        .service(
            scope("/project")
                .service(
                    resource("/{project_id}/resource_count/internal").route(get().to(
                        get_project_resource_count_internal_v1_0::get_project_resource_count_internal,
                    )),
                )
                .service(
                    resource("/{project_id}/export/internal")
                        .route(get().to(export_project_internal_v1_0::export_project_internal)),
                )
                .service(
                    resource("/{project_id}/import/internal")
                        .route(post().to(import_project_internal_v1_0::import_project_internal)),
                ),
        )
}
