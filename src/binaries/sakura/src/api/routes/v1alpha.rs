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

use crate::api::http_endpoints::task::*;
use crate::api::http_endpoints::virtual_machine::*;

/// Builds the `/v1alpha`-scope with all endpoints of the sakura.
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
            scope("/virtual_machine")
                .service(
                    resource("/internal")
                        .route(post().to(
                            reserve_virtual_machine_internal_v1_0::reserve_virtual_machine_internal,
                        ))
                        .route(
                            get().to(
                                list_virtual_machine_internal_v1_0::list_virtual_machine_internal,
                            ),
                        ),
                )
                .service(
                    resource("/{virtual_machine_uuid}/internal")
                        .route(
                            get().to(
                                get_virtual_machine_internal_v1_0::get_virtual_machine_internal,
                            ),
                        )
                        .route(delete().to(
                            delete_virtual_machine_internal_v1_0::delete_virtual_machine_internal,
                        )),
                )
                .service(
                    resource("/{virtual_machine_uuid}")
                        .route(post().to(create_virtual_machine_v1_0::create_virtual_machine)),
                )
                .service(
                    resource("/{virtual_machine_uuid}/snapshot_save")
                        .route(post().to(snapshot_save_v1_0::snapshot_save_task)),
                )
                .service(
                    resource("/{virtual_machine_uuid}/snapshot_restore")
                        .route(post().to(snapshot_restore_v1_0::snapshot_restore_task)),
                ),
        )
        .service(
            scope("/task")
                .service(resource("/{task_uuid}").route(get().to(get_task_v1_0::get_task)))
                .service(
                    resource("/{task_uuid}/abort").route(put().to(abort_task_v1_0::abort_task)),
                )
                .service(resource("").route(get().to(list_task_v1_0::list_task))),
        )
}
