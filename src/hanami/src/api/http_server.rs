// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimim.moe>

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
use apistos::app::OpenApiWrapper;
use apistos::spec::Spec;
use apistos::web::{post, put, get, delete, resource, scope, Scope};
use apistos::info::Info;
use apistos::info::{Contact, License};
use apistos::paths::ExternalDocumentation;
use std::error::Error;
use log::{info, debug};

use crate::api::http_endpoints::auth::{renew_token_v1_0, create_token_v1_0};
use crate::api::http_endpoints::user::{create_user_v1_0, list_user_v1_0, get_user_v1_0, delete_user_v1_0};
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
            scope("/user")
                .service(
                    resource("")
                        .route(post().to(create_user_v1_0::create_user))
                        .route(get().to(list_user_v1_0::list_user))
                )
                .service(
                    resource("/{id}")
                        .route(get().to(get_user_v1_0::get_user))
                        .route(delete().to(delete_user_v1_0::delete_user))
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
              title: "OpenHanami-API-Documentation".to_string(),
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
            .service(v1alpha_routes())
            .build("/openapi.json")
    })
    .bind((ip, port))?
    .run()
    .await
}