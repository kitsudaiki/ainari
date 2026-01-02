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

use crate::api::http_endpoints::cluster::task::*;
use crate::api::http_endpoints::cluster::*;

pub fn v1alpha_routes() -> Scope {
    scope("/v1alpha")
        .service(
            scope("/version").service(resource("").route(get().to(get_version_v1_0::get_version))),
        )
        .service(
            scope("/cluster")
                .service(
                    resource("/internal")
                        .route(post().to(create_cluster_internal_v1_0::create_cluster_internal))
                        .route(get().to(list_cluster_internal_v1_0::list_cluster_internal)),
                )
                .service(
                    resource("/{cluster_uuid}/internal")
                        .route(get().to(get_cluster_internal_v1_0::get_cluster_internal))
                        .route(delete().to(delete_cluster_internal_v1_0::delete_cluster_internal)),
                )
                .service(
                    resource("/{cluster_uuid}/request")
                        .route(put().to(request_cluster_v1_0::request_cluster)),
                )
                .service(
                    resource("/{cluster_uuid}/train")
                        .route(put().to(train_cluster_v1_0::train_cluster)),
                )
                .service(
                    scope("/{cluster_uuid}/task")
                        .service(
                            resource("/train")
                                .route(post().to(create_train_task_v1_0::create_train_task)),
                        )
                        .service(
                            resource("/request")
                                .route(post().to(create_request_task_v1_0::create_request_task)),
                        )
                        .service(
                            resource("/checkpoint_save")
                                .route(post().to(checkpoint_save_task_v1_0::checkpoint_save_task)),
                        )
                        .service(resource("/checkpoint_restore").route(
                            post().to(checkpoint_restore_task_v1_0::checkpoint_restore_task),
                        ))
                        .service(resource("/{task_uuid}").route(get().to(get_task_v1_0::get_task)))
                        .service(
                            resource("/{task_uuid}/abort").route(put().to(get_task_v1_0::get_task)),
                        )
                        .service(resource("").route(get().to(list_task_v1_0::list_task))),
                ),
        )
}
