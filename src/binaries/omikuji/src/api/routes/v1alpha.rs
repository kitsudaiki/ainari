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

use apistos::web::{Scope, get, post, resource, scope};

use ainari_api::endpoints::*;

use crate::api::http_endpoints::audit_log::*;
use crate::api::http_endpoints::error_log::*;

/// Builds the `/v1alpha`-scope with all endpoints of the omikuji.
///
/// The entries are only added by the other components over the internal endpoints and only
/// read by admins. They can neither be changed nor deleted over the API.
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
            scope("/error_log")
                .service(
                    resource("/internal")
                        .route(post().to(add_error_log_internal_v1_0::add_error_log_internal)),
                )
                .service(
                    resource("/admin")
                        .route(get().to(list_error_log_admin_v1_0::list_error_log_admin)),
                )
                .service(
                    resource("/{error_log_uuid}/admin")
                        .route(get().to(get_error_log_admin_v1_0::get_error_log_admin)),
                ),
        )
        .service(
            scope("/audit_log")
                .service(
                    resource("/internal")
                        .route(post().to(add_audit_log_internal_v1_0::add_audit_log_internal)),
                )
                .service(
                    resource("/admin")
                        .route(get().to(list_audit_log_admin_v1_0::list_audit_log_admin)),
                )
                .service(
                    resource("/{audit_log_uuid}/admin")
                        .route(get().to(get_audit_log_admin_v1_0::get_audit_log_admin)),
                ),
        )
}
