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

use crate::api::http_endpoints::floating_ip::*;
use crate::api::http_endpoints::network::*;
use crate::api::http_endpoints::sakura_host::*;
use crate::api::http_endpoints::virtual_machine::*;

/// Builds the `/v1alpha`-scope with all endpoints of the hanami.
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
                    resource("")
                        .route(post().to(reserve_virtual_machine_v1_0::reserve_virtual_machine))
                        .route(get().to(list_virtual_machine_v1_0::list_virtual_machine)),
                )
                .service(
                    resource("/count")
                        .route(get().to(get_virtual_machine_count_v1_0::get_virtual_machine_count)),
                )
                .service(
                    resource("/{virtual_machine_uuid}")
                        .route(get().to(get_virtual_machine_v1_0::get_virtual_machine))
                        .route(delete().to(delete_virtual_machine_v1_0::delete_virtual_machine)),
                ),
        )
        .service(
            scope("/network")
                .service(
                    resource("")
                        .route(post().to(create_network_v1_0::create_network))
                        .route(get().to(list_network_v1_0::list_network)),
                )
                .service(
                    resource("/{network_uuid}")
                        .route(get().to(get_network_v1_0::get_network))
                        .route(delete().to(delete_network_v1_0::delete_network)),
                ),
        )
        .service(
            scope("/floating_ip")
                .service(
                    resource("")
                        .route(post().to(create_floating_ip_v1_0::create_floating_ip))
                        .route(get().to(list_floating_ip_v1_0::list_floating_ip)),
                )
                .service(
                    resource("/{floating_ip_uuid}")
                        .route(get().to(get_floating_ip_v1_0::get_floating_ip))
                        .route(delete().to(delete_floating_ip_v1_0::delete_floating_ip)),
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
