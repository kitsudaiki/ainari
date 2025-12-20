// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

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

use crate::api::http_endpoints::cluster::*;
use crate::api::http_endpoints::sakura_host::*;

pub fn v1alpha_routes() -> Scope {
    scope("/v1alpha")
        .service(
            scope("/version").service(resource("").route(get().to(get_version_v1_0::get_version))),
        )
        .service(
            scope("/cluster")
                .service(
                    resource("")
                        .route(post().to(create_cluster_v1_0::create_cluster))
                        .route(get().to(list_cluster_v1_0::list_cluster)),
                )
                .service(
                    resource("/count").route(get().to(get_cluster_count_v1_0::get_cluster_count)),
                )
                .service(
                    resource("/{cluster_uuid}")
                        .route(get().to(get_cluster_v1_0::get_cluster))
                        .route(delete().to(delete_cluster_v1_0::delete_cluster)),
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
                ),
        )
}
