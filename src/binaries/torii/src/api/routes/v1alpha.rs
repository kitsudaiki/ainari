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
use crate::api::http_endpoints::network_crypto::*;
use crate::api::http_endpoints::network_filter::*;
use crate::api::http_endpoints::network_interface::*;
use crate::api::http_endpoints::proxy::*;
use crate::api::http_endpoints::route::*;

/// Builds the `/v1alpha`-scope with all endpoints of the torii.
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
            scope("/proxy")
                .service(
                    resource("/internal")
                        .route(post().to(set_proxy_internal_v1_0::register_proxy_internal)),
                )
                .service(resource("").route(get().to(list_proxy_v1_0::list_proxy)))
                .service(resource("/{proxy_uuid}").route(get().to(get_proxy_v1_0::get_proxy)))
                .service(
                    resource("/{proxy_uuid}/internal")
                        .route(delete().to(delete_proxy_internal_v1_0::delete_proxy_internal)),
                ),
        )
        .service(
            scope("/floating_ip")
                .service(
                    resource("/internal").route(
                        post().to(add_floating_ip_internal_v1_0::register_floating_ip_internal),
                    ),
                )
                .service(resource("/{ip}/internal").route(
                    delete().to(delete_floating_ip_internal_v1_0::delete_floating_ip_internal),
                )),
        )
        .service(
            scope("/route")
                .service(
                    resource("/internal")
                        .route(get().to(list_route_internal_v1_0::list_route_internal))
                        .route(post().to(add_route_internal_v1_0::register_route_internal)),
                )
                .service(
                    resource("/{route_uuid}/internal")
                        .route(put().to(update_route_internal_v1_0::update_route_internal))
                        .route(delete().to(delete_route_internal_v1_0::delete_route_internal)),
                )
                .service(
                    resource("/{route_uuid}/filter/internal")
                        .route(get().to(get_filter_internal_v1_0::get_filter_internal))
                        .route(delete().to(clear_filter_internal_v1_0::clear_filter_internal)),
                )
                .service(
                    resource("/{route_uuid}/filter/ip_range/internal")
                        .route(
                            post().to(
                                add_filter_ip_range_internal_v1_0::add_filter_ip_range_internal,
                            ),
                        )
                        .route(delete().to(
                            delete_filter_ip_range_internal_v1_0::delete_filter_ip_range_internal,
                        )),
                )
                .service(
                    resource("/{route_uuid}/filter/port/internal")
                        .route(post().to(add_filter_port_internal_v1_0::add_filter_port_internal))
                        .route(
                            delete()
                                .to(delete_filter_port_internal_v1_0::delete_filter_port_internal),
                        ),
                ),
        )
        .service(
            scope("/network_filter").service(
                resource("").route(get().to(list_filter_internal_v1_0::list_filter_internal)),
            ),
        )
        .service(
            scope("/network_crypto")
                .service(resource("/key"))
                .service(
                    resource("/key/internal")
                        .route(get().to(list_crypto_key_internal_v1_0::list_crypto_key_internal))
                        .route(
                            post().to(add_crypto_key_internal_v1_0::register_crypto_key_internal),
                        ),
                )
                .service(resource("/key/{direction}/{spi}/internal").route(
                    delete().to(delete_crypto_key_internal_v1_0::delete_crypto_key_internal),
                ))
                .service(
                    resource("/toggle/internal")
                        .route(post().to(toggle_crypto_internal_v1_0::toggle_crypto_internal)),
                )
                .service(
                    resource("/connection/internal")
                        .route(get().to(list_connection_internal_v1_0::list_connection_internal)),
                ),
        )
        .service(
            scope("/network_interface")
                .service(
                    resource("/config/internal").route(
                        post().to(config_interface_internal_v1_0::config_interface_internal),
                    ),
                )
                .service(
                    resource("/tap/internal")
                        .route(post().to(register_tap_internal_v1_0::register_tap_internal)),
                ),
        )
}
