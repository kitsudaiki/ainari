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
use actix_web::web::PayloadConfig;
use actix_web::{App, HttpServer};
use apistos::app::OpenApiWrapper;
use apistos::info::Info;
use apistos::info::{Contact, License};
use apistos::paths::ExternalDocumentation;
use apistos::spec::Spec;
use apistos::web::{Scope, delete, get, post, put, resource, scope};
use std::error::Error;

use ainari_api::cors_middleware::cors_middleware;
use ainari_api::endpoints::*;

use crate::api::http_endpoints::auth::*;
use crate::api::http_endpoints::endpoints::*;
use crate::api::http_endpoints::project::*;
use crate::api::http_endpoints::user::*;
use crate::api::miko_auth_middleware::authorization_middleware;
use crate::config;

fn v1alpha_routes() -> Scope {
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
                    resource("")
                        .route(post().to(create_project_v1_0::create_project))
                        .route(get().to(list_project_v1_0::list_project)),
                )
                .service(
                    resource("/{project_id}")
                        .route(get().to(get_project_v1_0::get_project))
                        .route(delete().to(delete_project_v1_0::delete_project)),
                ),
        )
        .service(
            scope("/user")
                .service(
                    resource("")
                        .route(post().to(create_user_v1_0::create_user))
                        .route(get().to(list_user_v1_0::list_user)),
                )
                .service(
                    resource("/{user_id}")
                        .route(get().to(get_user_v1_0::get_user))
                        .route(delete().to(delete_user_v1_0::delete_user)),
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
                title: "Miko-API-Documentation".to_string(),
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
