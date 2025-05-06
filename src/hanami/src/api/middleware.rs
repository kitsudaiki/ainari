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
    http::Method
};
use log::{info, debug};

use crate::api::{token_handling, errors::ErrorResponse};
use hanami_common::functions::split_bearer_token;

pub async fn authorization_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {

    let mut skip_check = false;

    // skip check for specific endpoints
    skip_check |= req.uri() == "/v1alpha/token" && req.method() == &Method::POST;
    skip_check |= req.uri() == "/openapi.json" && req.method() == &Method::GET;

    if skip_check == false {
        debug!("Check token for request against {}", req.uri());
        // get token from header
        let token: &str;
        match req.headers().get("Authorization") {
            Some(value) => {
                match split_bearer_token(value.to_str().unwrap()) {
                    Some(val) => token = val,
                    None => {
                        println!("Invalid token format");
                        return Err(ErrorResponse::Unauthorized("Missing token in header".to_string()).into());
                    },
                }
            }
            _ => {
                return Err(ErrorResponse::Unauthorized("Missing token in header".to_string()).into());
            }
        }

        match token_handling::validate_token(token) {
            Ok(_) => {},
            Err(e) => {
                debug!("{}", e);
                return Err(ErrorResponse::Unauthorized(e).into());
            }
        }
    }
    //else {
    //    debug!("skip token-check");
    //}

    info!("Api-call against URI: {}", req.uri());

    let resp = next.call(req).await;

    match resp {
        Ok(_) => {},
        Err(ref e) => {
            info!("{}", e);
        },
    };

    return resp;
}