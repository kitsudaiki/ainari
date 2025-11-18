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
use std::error::Error;

use ainari_api::auth_middleware::*;
use ainari_api::cors_middleware::cors_middleware;

use crate::config;

use super::routes::v1alpha::v1alpha_routes;

#[actix_web::main]
pub async fn run_server() -> Result<(), impl Error> {
    log::debug!("initialize server");

    // get server-address from config
    let public_ip = config::CONFIG.api.public_ip.clone();
    let public_port = config::CONFIG.api.public_port;
    log::info!("HTTP-server listen public on {public_ip}:{public_port}");
    let internal_ip = config::CONFIG.api.internal_ip.clone();
    let internal_port = config::CONFIG.api.internal_port;
    log::info!("HTTP-server listen internally on {internal_ip}:{internal_port}");

    let api_validation_config = ApiValidationConfig::new(
        &config::CONFIG.miko,
        &config::CONFIG.api,
        config::CONFIG.insecure_clients,
    );

    // init server with openapi-docu-generator
    HttpServer::new(move || {
        let spec = Spec {
            info: Info {
                title: "Omamori-API-Documentation".to_string(),
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
            .app_data(web::Data::new(api_validation_config.clone())) // provide validation configs to the middleware
            .wrap(from_fn(authorization_middleware))
            .wrap(from_fn(cors_middleware))
            .wrap(Logger::default())
            .app_data(PayloadConfig::new(1 << 30)) // 1GB max payload-size
            .service(v1alpha_routes())
            .build("/openapi.json")
    })
    .bind((public_ip, public_port))?
    .bind((internal_ip, internal_port))?
    .run()
    .await
}
