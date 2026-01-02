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

use crate::api::http_endpoints::auth::*;
use crate::api::http_endpoints::endpoints::*;
use crate::api::http_endpoints::project::*;
use crate::api::http_endpoints::quota::*;
use crate::api::http_endpoints::user::*;

pub fn v1alpha_routes() -> Scope {
    scope("/v1alpha")
        .service(
            scope("/version").service(resource("").route(get().to(get_version_v1_0::get_version))),
        )
        .service(
            scope("/token").service(
                resource("")
                    .route(put().to(renew_token_v1_0::renew_token))
                    .route(post().to(create_token_v1_0::create_token))
                    .route(get().to(validate_token_v1_0::validate_token)),
            ),
        )
        .service(
            scope("/endpoints")
                .service(resource("").route(get().to(get_endpoints_v1_0::get_endpoints))),
        )
        .service(
            scope("/project")
                .service(
                    resource("/admin")
                        .route(post().to(create_project_admin_v1_0::create_project_admin))
                        .route(get().to(list_project_admin_v1_0::list_project_admin)),
                )
                .service(
                    resource("/{project_id}/admin")
                        .route(get().to(get_project_admin_v1_0::get_project_admin))
                        .route(delete().to(delete_project_admin_v1_0::delete_project_admin)),
                ),
        )
        .service(
            scope("/user")
                .service(
                    resource("/admin")
                        .route(post().to(create_user_admin_v1_0::create_user_admin))
                        .route(get().to(list_user_admin_v1_0::list_user_admin)),
                )
                .service(
                    resource("/{user_id}/admin")
                        .route(get().to(get_user_admin_v1_0::get_user_admin))
                        .route(delete().to(delete_user_admin_v1_0::delete_user_admin)),
                ),
        )
        .service(
            scope("/quota")
                .service(resource("").route(get().to(get_quota_v1_0::get_quota)))
                .service(
                    resource("/admin").route(get().to(list_quota_admin_v1_0::list_quota_admin)),
                )
                .service(
                    resource("/{user_id}/admin")
                        .route(get().to(get_quota_admin_v1_0::get_quota_admin))
                        .route(put().to(set_quota_admin_v1_0::set_quota_admin)),
                ),
        )
}
