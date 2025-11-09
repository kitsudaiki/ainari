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

use apistos::web::{Scope, delete, get, post, put, resource, scope};

use ainari_api::endpoints::*;

use crate::api::http_endpoints::checkpoint::*;
use crate::api::http_endpoints::dataset::*;

pub fn v1alpha_routes() -> Scope {
    scope("/v1alpha")
        .service(
            scope("/version").service(resource("").route(get().to(get_version_v1_0::get_version))),
        )
        .service(
            scope("/dataset")
                .service(
                    resource("/internal")
                        .route(post().to(init_dataset_internal_v1_0::init_dataset)),
                )
                .service(
                    resource("/{dataset_uuid}")
                        .route(get().to(get_dataset_v1_0::get_dataset))
                        .route(delete().to(delete_dataset_v1_0::delete_dataset)),
                )
                .service(
                    resource("/{dataset_uuid}/check")
                        .route(put().to(check_dataset_v1_0::check_dataset)),
                )
                .service(
                    resource("/{dataset_uuid}/internal")
                        .route(get().to(get_dataset_internal_v1_0::get_dataset_internal)),
                )
                .service(resource("").route(get().to(list_dataset_v1_0::list_dataset)))
                .service(
                    resource("/{type}/{name}").route(post().to(create_dataset_v1_0::upload_binary)),
                ),
        )
        .service(
            scope("/checkpoint")
                .service(
                    resource("/internal")
                        .route(post().to(init_checkpoint_internal_v1_0::init_checkpoint)),
                )
                .service(resource("").route(get().to(list_checkpoint_v1_0::list_checkpoint)))
                .service(
                    resource("/{checkpoint_uuid}")
                        .route(get().to(get_checkpoint_v1_0::get_checkpoint))
                        .route(delete().to(delete_checkpoint_v1_0::delete_checkpoint)),
                )
                .service(
                    resource("/{checkpoint_uuid}/internal")
                        .route(get().to(get_checkpoint_internal_v1_0::get_checkpoint_internal)),
                ),
        )
}
