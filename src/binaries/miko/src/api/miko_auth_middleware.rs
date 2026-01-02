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

use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    http::Method,
    middleware::Next,
    web,
};

use crate::api::token_handling;

use ainari_api::auth_middleware::{ApiValidationConfig, check_internal_request};
use ainari_api::errors::ErrorResponse;
use ainari_common::functions::split_bearer_token;

pub async fn authorization_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let mut skip_check = false;
    let uri = req.uri();
    let api_validation_config = req
        .app_data::<web::Data<ApiValidationConfig>>()
        .expect("Api-validation-config missing!");

    log::debug!("call uri: '{uri}' for method: '{}'", *req.method());

    // skip check for specific endpoints
    skip_check |= uri == "/v1alpha/token" && *req.method() == Method::POST;
    skip_check |= uri == "/openapi.json" && *req.method() == Method::GET;
    skip_check |= uri == "/v1alpha/endpoints" && *req.method() == Method::GET;
    skip_check |= *req.method() == Method::OPTIONS;

    if !skip_check {
        check_internal_request(&req, api_validation_config)?;
        check_auth_header(&req).await?;
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

async fn check_auth_header(req: &ServiceRequest) -> Result<(), actix_web::Error> {
    let uri = req.uri();

    log::debug!("Check token for request against {uri}");
    // get token from header
    let auth_header = match req.headers().get("Authorization") {
        Some(value) => value,
        _ => {
            return Err(
                ErrorResponse::Unauthorized("Authorization-header not set".to_string()).into(),
            );
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
            return Err(ErrorResponse::Unauthorized("Missing token in header".to_string()).into());
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

    Ok(())
}
