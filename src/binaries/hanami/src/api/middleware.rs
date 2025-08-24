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
};

use crate::api::{errors::ErrorResponse, token_handling};
use ainari_common::functions::split_bearer_token;

pub async fn authorization_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let mut skip_check = false;
    let uri = req.uri();

    // skip check for specific endpoints
    let is_post_req = *req.method() == Method::POST;
    let is_get_req = *req.method() == Method::GET;
    skip_check |= uri == "/v1alpha/token" && is_post_req;
    skip_check |= uri == "/openapi.json" && is_get_req;

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

        // check token
        match token_handling::validate_token(token) {
            Ok(_) => {}
            Err(e) => {
                log::debug!("{e}");
                return Err(ErrorResponse::Unauthorized(e).into());
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
