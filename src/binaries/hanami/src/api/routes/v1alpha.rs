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

use crate::api::http_endpoints::floating_ip::*;
use crate::api::http_endpoints::network::*;
use crate::api::http_endpoints::network_filter::*;
use crate::api::http_endpoints::project::*;
use crate::api::http_endpoints::proxy::*;
use crate::api::http_endpoints::sakura_host::*;
use crate::api::http_endpoints::virtual_machine::*;
use crate::api::http_endpoints::vm_type::*;

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
            // read-only view on the proxies of the torii at the edge, whose api is only reachable
            // within the cluster
            scope("/proxy")
                .service(resource("").route(get().to(list_proxy_v1_0::list_proxy)))
                .service(resource("/{proxy_uuid}").route(get().to(get_proxy_v1_0::get_proxy))),
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
            // the packet filters are applied by the torii of the host of the virtual_machine, but
            // read from the database of hanami
            scope("/network_filter")
                .service(
                    resource("").route(get().to(list_network_filter_v1_0::list_network_filter)),
                )
                .service(
                    resource("/{virtual_machine_uuid}/{direction}")
                        .route(get().to(get_network_filter_v1_0::get_network_filter))
                        .route(delete().to(delete_network_filter_v1_0::delete_network_filter)),
                )
                .service(
                    resource("/{virtual_machine_uuid}/{direction}/ip_range")
                        .route(
                            post()
                                .to(add_network_filter_ip_range_v1_0::add_network_filter_ip_range),
                        )
                        .route(delete().to(
                            delete_network_filter_ip_range_v1_0::delete_network_filter_ip_range,
                        )),
                )
                .service(
                    resource("/{virtual_machine_uuid}/{direction}/port")
                        .route(post().to(add_network_filter_port_v1_0::add_network_filter_port))
                        .route(
                            delete()
                                .to(delete_network_filter_port_v1_0::delete_network_filter_port),
                        ),
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
                )
                .service(
                    resource("/{floating_ip_uuid}/attach")
                        .route(put().to(attach_floating_ip_v1_0::attach_floating_ip)),
                )
                .service(
                    resource("/{floating_ip_uuid}/detach")
                        .route(put().to(detach_floating_ip_v1_0::detach_floating_ip)),
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
        .service(
            // vm-types are global and can be read by all users, but only be changed by admins
            scope("/vm_type")
                .service(resource("").route(get().to(list_vm_type_v1_0::list_vm_type)))
                .service(
                    resource("/admin")
                        .route(post().to(create_vm_type_admin_v1_0::create_vm_type_admin)),
                )
                .service(resource("/{vm_type_uuid}").route(get().to(get_vm_type_v1_0::get_vm_type)))
                .service(
                    resource("/{vm_type_uuid}/admin")
                        .route(put().to(update_vm_type_admin_v1_0::update_vm_type_admin))
                        .route(delete().to(delete_vm_type_admin_v1_0::delete_vm_type_admin)),
                ),
        )
        .service(scope("/project").service(
            resource("/{project_id}/resource_count/internal").route(
                get().to(
                    get_project_resource_count_internal_v1_0::get_project_resource_count_internal,
                ),
            ),
        ))
}
