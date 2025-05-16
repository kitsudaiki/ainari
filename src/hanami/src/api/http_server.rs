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

use actix_web::{App, HttpServer};
use actix_web::middleware::{Logger, from_fn};
use actix_web::web::PayloadConfig;
use apistos::app::OpenApiWrapper;
use apistos::spec::Spec;
use apistos::web::{post, put, get, delete, resource, scope, Scope};
use apistos::info::Info;
use apistos::info::{Contact, License};
use apistos::paths::ExternalDocumentation;
use std::error::Error;
use log::{info, debug};

use crate::api::http_endpoints::auth::*;
use crate::api::http_endpoints::user::*;
use crate::api::http_endpoints::project::*;
use crate::api::http_endpoints::dataset::*;
use crate::api::http_endpoints::checkpoint::*;
use crate::api::http_endpoints::cluster::*;
use crate::api::http_endpoints::cluster::task::*;
use crate::api::middleware::authorization_middleware;
use crate::config;

fn v1alpha_routes() -> Scope {
    scope("/v1alpha")
        .service(
            scope("/token")
                .service(
                    resource("")
                        .route(put().to(renew_token_v1_0::renew_token))
                        .route(post().to(create_token_v1_0::create_token))
                )
        )
        .service(
            scope("/project")
                .service(
                    resource("")
                        .route(post().to(create_project_v1_0::create_project))
                        .route(get().to(list_project_v1_0::list_project))
                )
                .service(
                    resource("/{project_id}")
                        .route(get().to(get_project_v1_0::get_project))
                        .route(delete().to(delete_project_v1_0::delete_project))
                )
        )
        .service(
            scope("/user")
                .service(
                    resource("")
                        .route(post().to(create_user_v1_0::create_user))
                        .route(get().to(list_user_v1_0::list_user))
                )
                .service(
                    resource("/{user_id}")
                        .route(get().to(get_user_v1_0::get_user))
                        .route(delete().to(delete_user_v1_0::delete_user))
                )
        )
        .service(
            scope("/dataset")
                .service(
                    resource("/{type}/{name}")
                        .route(post().to(create_dataset_v1_0::upload_binary))
                )
                .service(
                    resource("")
                        .route(get().to(list_dataset_v1_0::list_dataset))
                )
                .service(
                    resource("/{dataset_uuid}")
                        .route(get().to(get_dataset_v1_0::get_dataset))
                        .route(delete().to(delete_dataset_v1_0::delete_dataset))
                )
        )
        .service(
            scope("/checkpoint")
                .service(
                    resource("")
                        .route(get().to(list_checkpoint_v1_0::list_checkpoint))
                )
                .service(
                    resource("/{checkpoint_uuid}")
                        .route(get().to(get_checkpoint_v1_0::get_checkpoint))
                        .route(delete().to(delete_checkpoint_v1_0::delete_checkpoint))
                )
        )
        .service(
            scope("/cluster")
                .service(
                    resource("")
                        .route(post().to(create_cluster_v1_0::create_cluster))
                        .route(get().to(list_cluster_v1_0::list_cluster))
                )
                .service(
                    resource("/{cluster_uuid}")
                        .route(get().to(get_cluster_v1_0::get_cluster))
                        .route(delete().to(delete_cluster_v1_0::delete_cluster))
                )
                .service(
                    resource("/{cluster_uuid}/mode")
                        .route(put().to(switch_mode_v1_0::switch_mode))
                )
                .service(
                    resource("/{cluster_uuid}/request")
                        .route(put().to(request_cluster_v1_0::request_cluster))
                )
                .service(
                    resource("/{cluster_uuid}/train")
                        .route(put().to(train_cluster_v1_0::train_cluster))
                )
                .service(
                    scope("/{cluster_uuid}/task")
                        .service(
                            resource("/train")
                                .route(post().to(create_train_task_v1_0::create_train_task))
                        )
                        .service(
                            resource("/request")
                                .route(post().to(create_request_task_v1_0::create_request_task))
                        )
                        .service(
                            resource("/checkpoint_save")
                                .route(post().to(create_checkpoint_save_task_v1_0::create_checkpoint_save_task))
                        )
                        .service(
                            resource("/{task_uuid}")
                                .route(get().to(get_task_v1_0::get_task))
                        )
                        .service(
                            resource("/{task_uuid}/abort")
                                .route(put().to(get_task_v1_0::get_task))
                        )
                        .service(
                            resource("")
                                .route(get().to(list_task_v1_0::list_task))
                        )
                )
        )
}

#[actix_web::main]
pub async fn run_server() -> Result<(), impl Error> {
    debug!("initialize server");
    // get server-address from config
    let ip = config::CONFIG.api.ip.clone();
    let port = config::CONFIG.api.port.clone();
    info!("HTTP-server listen on {}:{}", ip, port);
    
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
              version: "0.8.0".to_string(),
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
            .wrap(from_fn(authorization_middleware))
            .wrap(Logger::default())
            .app_data(PayloadConfig::new(1 << 30))  // 1GB max payload-size
            .service(v1alpha_routes())
            .build("/openapi.json")
    })
    .bind((ip, port))?
    .run()
    .await
}