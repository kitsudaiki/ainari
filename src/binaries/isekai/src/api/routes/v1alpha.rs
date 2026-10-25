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

use apistos::web::{Scope, delete, get, post, resource, scope};

use ainari_api::endpoints::*;

use crate::api::http_endpoints::backup::*;
use crate::api::http_endpoints::job::*;
use crate::api::http_endpoints::restore::*;

/// Builds the `/v1alpha`-scope with all endpoints of isekai.
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
            scope("/backup")
                .service(
                    resource("")
                        .route(post().to(create_project_backup_v1_0::create_project_backup))
                        .route(get().to(list_backup_v1_0::list_backup)),
                )
                // registered before "/{backup_uuid}", which would match it as well
                .service(resource("/virtual_machine").route(
                    post().to(create_virtual_machine_backup_v1_0::create_virtual_machine_backup),
                ))
                .service(
                    resource("/{backup_uuid}")
                        .route(get().to(get_backup_v1_0::get_backup))
                        .route(delete().to(delete_backup_v1_0::delete_backup)),
                )
                .service(
                    resource("/{backup_uuid}/restore")
                        .route(post().to(restore_backup_v1_0::restore_backup)),
                ),
        )
        .service(
            scope("/job")
                .service(resource("").route(get().to(list_job_v1_0::list_job)))
                .service(resource("/{job_uuid}").route(get().to(get_job_v1_0::get_job))),
        )
}
