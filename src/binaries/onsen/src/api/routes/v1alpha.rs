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

use apistos::web::{Scope, delete, get, post, put, resource, scope};

use ainari_api::endpoints::*;

use crate::api::http_endpoints::backend::*;
use crate::api::http_endpoints::directory::*;
use crate::api::http_endpoints::file::*;
use crate::api::http_endpoints::volume::*;

/// Builds the `/v1alpha`-scope with all endpoints of the onsen.
///
/// Beside the version and the ready-status, all endpoints are internal, because only the ryokan
/// and the sakura-hosts store and read the files and manage the volumes.
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
            scope("/file")
                .service(
                    resource("/internal")
                        .route(put().to(upload_file_internal_v1_0::upload_file_internal))
                        .route(get().to(download_file_internal_v1_0::download_file_internal))
                        .route(delete().to(delete_file_internal_v1_0::delete_file_internal)),
                )
                .service(
                    resource("/info/internal")
                        .route(get().to(get_file_info_internal_v1_0::get_file_info_internal)),
                ),
        )
        .service(
            scope("/directory").service(
                resource("/internal")
                    .route(get().to(list_directory_internal_v1_0::list_directory_internal))
                    .route(delete().to(delete_directory_internal_v1_0::delete_directory_internal)),
            ),
        )
        .service(
            scope("/volume")
                .service(
                    resource("/internal")
                        .route(post().to(create_volume_internal_v1_0::create_volume_internal)),
                )
                .service(
                    resource("/{backend_id}/internal")
                        .route(get().to(get_volume_internal_v1_0::get_volume_internal))
                        .route(delete().to(delete_volume_internal_v1_0::delete_volume_internal)),
                )
                .service(
                    resource("/{backend_id}/size/internal")
                        .route(put().to(resize_volume_internal_v1_0::resize_volume_internal)),
                )
                .service(
                    resource("/{backend_id}/copy/internal")
                        .route(post().to(copy_volume_internal_v1_0::copy_volume_internal)),
                )
                .service(resource("/{backend_id}/connection/internal").route(
                    get().to(get_volume_connection_internal_v1_0::get_volume_connection_internal),
                )),
        )
        .service(scope("/backend").service(
            resource("/internal").route(get().to(get_backend_internal_v1_0::get_backend_internal)),
        ))
}
