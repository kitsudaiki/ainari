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
use base64::{Engine as _, engine::general_purpose};
use futures::future::{Ready, ready};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use ainari_common::functions::split_bearer_token;

#[derive(ApiSecurity, Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[openapi_security(scheme(security_type(http(scheme = "bearer", bearer_format = "JWT"))))]
pub struct UserContext {
    #[serde(default = "default_token")]
    pub token: String,
    pub user_id: String,
    pub project_id: String,
    pub is_admin: bool,
    pub is_project_admin: bool,
}

fn default_token() -> String {
    "".to_owned()
}

fn decode_jwt_payload(token: &str) -> Result<UserContext, Box<dyn std::error::Error>> {
    // split into parts: header.payload.signature
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("Invalid token format".into());
    }

    // decode the payload (2nd part)
    let decoded = general_purpose::URL_SAFE_NO_PAD.decode(parts[1])?;
    let mut context: UserContext = serde_json::from_slice(&decoded)?;
    context.token = token.to_owned();

    Ok(context)
}

impl FromRequest for UserContext {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let mut token: &str = "";
        match req.headers().get("Authorization") {
            Some(value) => match split_bearer_token(value.to_str().unwrap_or("")) {
                Some(val) => token = val,
                None => {
                    log::debug!("Invalid token format");
                }
            },
            _ => {
                log::debug!("Invalid or missing Authorization-header.");
            }
        }

        match decode_jwt_payload(token) {
            Ok(context) => ready(Ok(context)),
            Err(_) => {
                // should never be the case, because the middleware already checks the token
                ready(Ok(UserContext {
                    token: "".to_string(),
                    user_id: "".to_string(),
                    project_id: "".to_string(),
                    is_admin: false,
                    is_project_admin: false,
                }))
            }
        }
    }
}
