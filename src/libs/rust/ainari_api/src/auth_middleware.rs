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

use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    http::Method,
    middleware::Next,
    web,
};

use crate::errors::ErrorResponse;

use ainari_clients::auth::check_token;
use ainari_common::config::MikoEndpoint;
use ainari_common::error::AinariError;
use ainari_common::functions::split_bearer_token;

#[derive(Debug, Clone)]
pub struct MikoConfig {
    pub address: String,
    pub port: u16,
    pub insecure_connection: bool,
}

impl MikoConfig {
    pub fn new(conn: &MikoEndpoint, insecure_connection: bool) -> Self {
        MikoConfig {
            address: conn.address.clone(),
            port: conn.port,
            insecure_connection,
        }
    }
}

pub async fn authorization_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let mut skip_check = false;
    let uri = req.uri();
    let miko_config = req
        .app_data::<web::Data<MikoConfig>>()
        .expect("Miko-config missing!");

    // skip check for specific endpoints
    skip_check |= uri == "/openapi.json";
    skip_check |= *req.method() == Method::OPTIONS;

    if !skip_check {
        log::debug!("Check token for request against {uri}");
        // get token from header
        let auth_header = match req.headers().get("Authorization") {
            Some(value) => value,
            _ => {
                return Err(ErrorResponse::Unauthorized(
                    "Authorization-header not set".to_string(),
                )
                .into());
            }
        };

        // convert into string
        let auth_header_str = match auth_header.to_str() {
            Ok(auth_header_str) => auth_header_str,
            Err(_) => {
                return Err(ErrorResponse::Unauthorized("Bad auth-header".to_string()).into());
            }
        };

        // parse token from the auth-header
        let token = match split_bearer_token(auth_header_str) {
            Some(token) => token,
            None => {
                println!("Invalid token format");
                return Err(
                    ErrorResponse::Unauthorized("Missing token in header".to_string()).into(),
                );
            }
        };

        let miko_address = miko_config.address.clone();
        let miko_port: u16 = miko_config.port;
        let complete_address = format!("{miko_address}:{miko_port}");
        let response = check_token(
            complete_address,
            token.to_string(),
            miko_config.insecure_connection,
        )
        .await;

        match response {
            Ok(_) => {
                // println!("Success: {body_str}");
            }
            Err(AinariError::Unauthorized(msg)) => {
                return Err(ErrorResponse::Unauthorized(msg).into());
            }
            Err(AinariError::InvalidInput(msg)) => {
                return Err(ErrorResponse::Unauthorized(msg).into());
            }
            Err(AinariError::Error(msg)) => {
                log::error!("Failed to load mnist-images with error: '{msg}'");
                return Err(ErrorResponse::InternalError("".to_string()).into());
            }
        }
    }
    //else {
    //    log::debug!("skip token-check");
    //}

    log::info!("Api-call against URI: {uri}");

    let resp = next.call(req).await;

    match resp {
        Ok(_) => {}
        Err(ref e) => {
            log::info!("{e}");
        }
    };

    resp
}
