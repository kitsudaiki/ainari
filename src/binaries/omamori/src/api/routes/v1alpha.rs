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

use crate::api::http_endpoints::secret::*;

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
            scope("/secret")
                .service(
                    resource("/generate")
                        .route(post().to(create_generated_secret_v1_0::create_secret)),
                )
                .service(
                    resource("")
                        .route(post().to(create_secret_v1_0::create_secret))
                        .route(get().to(list_secret_v1_0::list_secret)),
                )
                .service(
                    resource("/count").route(get().to(get_secret_count_v1_0::get_secret_count)),
                )
                .service(
                    resource("/{secret_uuid}")
                        .route(get().to(get_secret_v1_0::get_secret))
                        .route(delete().to(delete_secret_v1_0::delete_secret)),
                )
                .service(
                    resource("/{secret_uuid}/payload")
                        .route(get().to(get_secret_payload_v1_0::get_secret_with_payload)),
                ),
        )
}
