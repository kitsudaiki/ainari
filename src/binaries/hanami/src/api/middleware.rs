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
    middleware::Next,
};
use awc::Client;
use awc::http::StatusCode;

use ainari_api::errors::ErrorResponse;
use ainari_common::functions::split_bearer_token;

use crate::config;

pub async fn authorization_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let mut skip_check = false;
    let uri = req.uri();

    // skip check for specific endpoints
    skip_check |= uri == "/openapi.json";

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

        let client = Client::new();
        let torii_address = config::CONFIG.torii.address.clone();
        let torii_port = config::CONFIG.torii.port;
        let torii_address_complete = format!("{torii_address}:{torii_port}/v1alpha/token");

        let response = client
            .get(torii_address_complete)
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .send()
            .await;

        match response {
            Ok(mut resp) => {
                match resp.status() {
                    StatusCode::UNAUTHORIZED => {
                        log::debug!("Invalid token with 401-error");
                        return Err(ErrorResponse::Unauthorized("Invalid token".to_string()).into());
                    }
                    StatusCode::FORBIDDEN => {
                        log::debug!("Invalid token with 403-error");
                        return Err(ErrorResponse::Unauthorized("Invalid token".to_string()).into());
                    }
                    StatusCode::OK => {
                        log::debug!("Successfully checked token against Torii");
                    }
                    code => {
                        log::error!(
                            "Error while sending request for token-validation. Got response-code: {code}"
                        );
                        return Err(ErrorResponse::InternalError("".to_string()).into());
                    }
                }

                let _ = match resp.body().await {
                    Ok(body) => String::from_utf8_lossy(&body).into_owned(),
                    Err(e) => {
                        log::error!("Error while getting token-validation-body: {e}");
                        return Err(ErrorResponse::InternalError("".to_string()).into());
                    }
                };
                // println!("Success: {body_str}");
            }
            Err(e) => {
                log::error!("Error while sending request for token-validation: {e}");
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
