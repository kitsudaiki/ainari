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

use actix_web::dev::Payload;
use actix_web::http::Error;
use actix_web::{FromRequest, HttpRequest};
use apistos::ApiSecurity;
use futures::future::{ready, Ready};
use serde::{Deserialize, Serialize};

use ainari_common::functions::split_bearer_token;
use crate::api::token_handling;

#[derive(ApiSecurity, Debug, Serialize, Deserialize)]
#[openapi_security(
    scheme(
        security_type(
            http(
                scheme = "bearer", 
                bearer_format = "JWT"
            )
        )
    )
)]
pub struct UserContext{
    pub user_id: String,
    pub project_id: String,
    pub is_admin: bool,
    pub is_project_admin: bool,
}

impl FromRequest for UserContext {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let mut token: &str = "";
        match req.headers().get("Authorization") {
            Some(value) => {
                match split_bearer_token(value.to_str().unwrap()) {
                    Some(val) => token = val,
                    None => {
                        println!("Invalid token format");
                    },
                }
            }
            _ => {
                println!("❌ Invalid or missing X-Auth-Token.");
            }
        }

        match token_handling::validate_token(token) {
            Ok(context) => {
                ready(
                    Ok(context)
                )
            },
            Err(e) => {
                log::debug!("{}", e);
                // should never be the case, because the middleware already checks the token
                return ready(Ok(UserContext {
                    user_id: "".to_string(),
                    project_id: "".to_string(),
                    is_admin: false,
                    is_project_admin: false,
                }));
            }
        }
    }
}
