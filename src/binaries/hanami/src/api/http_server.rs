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

use actix_web::middleware::{Logger, from_fn};
use actix_web::web::{self, PayloadConfig};
use actix_web::{App, HttpServer};
use apistos::app::OpenApiWrapper;
use apistos::info::Info;
use apistos::info::{Contact, License};
use apistos::paths::ExternalDocumentation;
use apistos::spec::Spec;
use apistos::web::{Scope, delete, get, options, post, put, resource, scope};
use std::error::Error;

use ainari_api::auth_middleware::*;
use ainari_api::cors_middleware::cors_middleware;
use ainari_api::endpoints::*;

use crate::api::http_endpoints::checkpoint::*;
use crate::api::http_endpoints::cluster::task::*;
use crate::api::http_endpoints::cluster::*;
use crate::api::http_endpoints::dataset::*;
use crate::config;

fn v1alpha_routes() -> Scope {
    scope("/v1alpha")
        .service(
            scope("/version").service(resource("").route(get().to(get_version_v1_0::get_version))),
        )
        .service(
            scope("/dataset")
                .service(
                    resource("/{dataset_uuid}")
                        .route(options().to(options_check_v1_0::options_check))
                        .route(get().to(get_dataset_v1_0::get_dataset))
                        .route(delete().to(delete_dataset_v1_0::delete_dataset)),
                )
                .service(
                    resource("/{dataset_uuid}/check")
                        .route(options().to(options_check_v1_0::options_check))
                        .route(put().to(check_dataset_v1_0::check_dataset)),
                )
                .service(
                    resource("")
                        .route(options().to(options_check_v1_0::options_check))
                        .route(get().to(list_dataset_v1_0::list_dataset)),
                )
                .service(
                    resource("/{type}/{name}")
                        .route(options().to(options_check_v1_0::options_check))
                        .route(post().to(create_dataset_v1_0::upload_binary)),
                ),
        )
        .service(
            scope("/checkpoint")
                .service(
                    resource("")
                        .route(options().to(options_check_v1_0::options_check))
                        .route(get().to(list_checkpoint_v1_0::list_checkpoint)),
                )
                .service(
                    resource("/{checkpoint_uuid}")
                        .route(options().to(options_check_v1_0::options_check))
                        .route(get().to(get_checkpoint_v1_0::get_checkpoint))
                        .route(delete().to(delete_checkpoint_v1_0::delete_checkpoint)),
                ),
        )
        .service(
            scope("/cluster")
                .service(
                    resource("")
                        .route(options().to(options_check_v1_0::options_check))
                        .route(post().to(create_cluster_v1_0::create_cluster))
                        .route(get().to(list_cluster_v1_0::list_cluster)),
                )
                .service(
                    resource("/{cluster_uuid}")
                        .route(options().to(options_check_v1_0::options_check))
                        .route(get().to(get_cluster_v1_0::get_cluster))
                        .route(delete().to(delete_cluster_v1_0::delete_cluster)),
                )
                .service(
                    resource("/{cluster_uuid}/request")
                        .route(options().to(options_check_v1_0::options_check))
                        .route(put().to(request_cluster_v1_0::request_cluster)),
                )
                .service(
                    resource("/{cluster_uuid}/train")
                        .route(options().to(options_check_v1_0::options_check))
                        .route(put().to(train_cluster_v1_0::train_cluster)),
                )
                .service(
                    scope("/{cluster_uuid}/task")
                        .service(
                            resource("/train")
                                .route(options().to(options_check_v1_0::options_check))
                                .route(post().to(create_train_task_v1_0::create_train_task)),
                        )
                        .service(
                            resource("/request")
                                .route(options().to(options_check_v1_0::options_check))
                                .route(post().to(create_request_task_v1_0::create_request_task)),
                        )
                        .service(
                            resource("/checkpoint_save")
                                .route(options().to(options_check_v1_0::options_check))
                                .route(post().to(checkpoint_save_task_v1_0::checkpoint_save_task)),
                        )
                        .service(
                            resource("/checkpoint_restore")
                                .route(options().to(options_check_v1_0::options_check))
                                .route(
                                    post()
                                        .to(checkpoint_restore_task_v1_0::checkpoint_restore_task),
                                ),
                        )
                        .service(
                            resource("/{task_uuid}")
                                .route(options().to(options_check_v1_0::options_check))
                                .route(get().to(get_task_v1_0::get_task)),
                        )
                        .service(
                            resource("/{task_uuid}/abort")
                                .route(options().to(options_check_v1_0::options_check))
                                .route(put().to(get_task_v1_0::get_task)),
                        )
                        .service(
                            resource("")
                                .route(options().to(options_check_v1_0::options_check))
                                .route(get().to(list_task_v1_0::list_task)),
                        ),
                ),
        )
}

#[actix_web::main]
pub async fn run_server() -> Result<(), impl Error> {
    log::debug!("initialize server");
    // get server-address from config
    let ip = config::CONFIG.api.ip.clone();
    let port = config::CONFIG.api.port;
    log::info!("HTTP-server listen on {ip}:{port}");

    // init server with openapi-docu-generator
    HttpServer::new(move || {
        let spec = Spec {
            info: Info {
                title: "Hanami-API-Documentation".to_string(),
                contact: Some(Contact {
                    email: Some("tobias.anker@kitsunemimi.moe".to_string()),
                    ..Default::default()
                }),
                license: Some(License {
                    name: "Apache 2.0".to_string(),
                    url: Some("http://www.apache.org/licenses/LICENSE-2.0.html".to_string()),
                    ..Default::default()
                }),
                version: "0.9.0".to_string(),
                ..Default::default()
            },
            external_docs: Some(ExternalDocumentation {
                description: Some("Find out more about Swagger".to_string()),
                url: "http://swagger.io".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        App::new()
            .document(spec)
            .app_data(web::Data::new(config::CONFIG.torii.clone())) // to provide the address of the torii to the middleware
            .wrap(from_fn(authorization_middleware))
            .wrap(from_fn(cors_middleware))
            .wrap(Logger::default())
            .app_data(PayloadConfig::new(1 << 30)) // 1GB max payload-size
            .service(v1alpha_routes())
            .build("/openapi.json")
    })
    .bind((ip, port))?
    .run()
    .await
}
