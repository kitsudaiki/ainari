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

use crate::api::http_endpoints::key_package::*;
use crate::api::http_endpoints::mls_grant::*;
use crate::api::http_endpoints::mls_group::*;
use crate::api::http_endpoints::mls_message::*;

/// Builds the `/v1alpha`-scope with all endpoints of the izakaya.
///
/// All endpoints beside the version and the ready-status are internal, because only the gateways
/// and hanami talk to the izakaya.
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
            scope("/key_package")
                .service(resource("/internal").route(
                    post().to(upload_key_package_internal_v1_0::upload_key_package_internal),
                ))
                .service(
                    resource("/{client_id}/claim/internal").route(
                        post().to(claim_key_package_internal_v1_0::claim_key_package_internal),
                    ),
                )
                .service(resource("/{client_id}/count/internal").route(
                    get().to(get_key_package_count_internal_v1_0::get_key_package_count_internal),
                )),
        )
        .service(
            scope("/mls_grant").service(
                resource("/internal")
                    .route(post().to(store_mls_grant_internal_v1_0::store_mls_grant_internal)),
            ),
        )
        .service(
            scope("/mls_group")
                .service(resource("/{vni}/subscribe/internal").route(
                    post().to(subscribe_mls_group_internal_v1_0::subscribe_mls_group_internal),
                ))
                .service(resource("/{vni}/unsubscribe/internal").route(
                    post().to(unsubscribe_mls_group_internal_v1_0::unsubscribe_mls_group_internal),
                ))
                .service(resource("/{vni}/operation/internal").route(
                    post().to(finish_mls_operation_internal_v1_0::finish_mls_operation_internal),
                ))
                .service(
                    resource("/{vni}/round/internal")
                        .route(post().to(ack_mls_round_internal_v1_0::ack_mls_round_internal)),
                ),
        )
        .service(
            scope("/mls_message")
                .service(
                    resource("/internal").route(
                        post().to(send_mls_message_internal_v1_0::send_mls_message_internal),
                    ),
                )
                .service(
                    resource("/{client_id}/internal")
                        .route(get().to(list_mls_message_internal_v1_0::list_mls_message_internal)),
                )
                .service(resource("/{client_id}/{message_uuid}/internal").route(
                    delete().to(delete_mls_message_internal_v1_0::delete_mls_message_internal),
                )),
        )
}
