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

use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm, errors::ErrorKind};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use log::{error, debug};

use crate::api::user_context::UserContext;
use crate::config;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Claims {
    pub user_id: String,
    pub project_id: String,
    pub is_admin: bool,
    pub is_project_admin: bool,
    //pub aud: String,         // Optional. Audience
    pub exp: usize,          // Required (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
    pub iat: usize,          // Optional. Issued at (as UTC timestamp)
    pub iss: String,         // Optional. Issuer
    //pub nbf: usize,          // Optional. Not Before (as UTC timestamp)
    //pub sub: String,         // Optional. Subject (whom token refers to)
}

pub fn validate_token(token: &str) -> Result<UserContext, String> {
    // validate token
    let secret = config::TOKEN_KEY.as_bytes();;
    let key = DecodingKey::from_secret(secret);
    let validation = Validation::new(Algorithm::HS256);
    match decode::<UserContext>(token, &key, &validation) {
        Ok(context) => Ok(context.claims),
        Err(e) => match *e.kind() {
            ErrorKind::ExpiredSignature => {
                Err(format!("Token expired"))
            },
            _ => {
                Err(format!("Invalid token"))
            }
        }
    }
}

pub fn create_token(user_id: &String, project_id: &String, is_admin: bool, is_project_admin: bool) -> Result<String, ()> {
    let token_expire_time = config::CONFIG.auth.token_expire_time.clone();

    // get timestamps for token
    let current = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let expiration = current + token_expire_time;

    // create token-payload
    let claims = Claims {
        user_id: user_id.clone(),
        project_id: project_id.clone(),
        is_admin: is_admin,
        is_project_admin: is_project_admin,
        exp: expiration as usize,
        iat: current as usize,
        iss: "hanami".to_string(),
    };

    // create token
    let secret = config::TOKEN_KEY.as_bytes();;
    match encode(&Header::default(), &claims, &EncodingKey::from_secret(secret)) {
        Ok(token) => {
            debug!("Successfully created token for user-id '{}' and project-id '{}'", user_id, project_id);
            return Ok(token);
        },
        Err(e) => {
            error!("Failed to create user-token {:?}", e);
            return Err(());
        }
    }
}