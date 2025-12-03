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

use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, errors::ErrorKind};
use jsonwebtoken::{EncodingKey, Header, encode};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config;

use ainari_api_structs::user_context::UserContext;
use ainari_common::secret::Secret;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Claims {
    pub user_id: String,
    pub project_id: String,
    pub is_admin: String,
    pub is_project_admin: String,
    //pub aud: String,         // Optional. Audience
    pub exp: usize, // Required (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
    pub iat: usize, // Optional. Issued at (as UTC timestamp)
    pub iss: String, // Optional. Issuer
                    //pub nbf: usize,          // Optional. Not Before (as UTC timestamp)
                    //pub sub: String,         // Optional. Subject (whom token refers to)
}

pub fn validate_token(token: &str) -> Result<UserContext, String> {
    // validate token
    let key = DecodingKey::from_secret(TOKEN_KEY.reveal().as_bytes());
    let validation = Validation::new(Algorithm::HS256);
    match decode::<UserContext>(token, &key, &validation) {
        Ok(context) => {
            let mut user_context = context.claims;
            user_context.token = token.to_owned();
            Ok(user_context)
        }
        Err(e) => match *e.kind() {
            ErrorKind::ExpiredSignature => Err("Token expired".to_string()),
            _ => Err("Invalid token".to_string()),
        },
    }
}

pub fn create_token(
    user_id: &String,
    project_id: &String,
    is_admin: &str,
    is_project_admin: &str,
) -> Result<String, ()> {
    let token_expire_time = config::CONFIG.auth.token_expire_time;

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
        is_admin: is_admin.to_string(),
        is_project_admin: is_project_admin.to_string(),
        exp: expiration as usize,
        iat: current as usize,
        iss: "miko".to_string(),
    };

    // create token
    match encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(TOKEN_KEY.reveal().as_bytes()),
    ) {
        Ok(token) => {
            log::debug!(
                "Successfully created token for user-id '{user_id}' and project-id '{project_id}'"
            );
            Ok(token)
        }
        Err(e) => {
            log::error!("Failed to create user-token {e:?}");
            Err(())
        }
    }
}

static TOKEN_KEY: Lazy<Secret> = Lazy::new(|| {
    let file_path = &config::CONFIG.auth.token_key_path;
    log::debug!("read token-key from file: '{file_path}'");

    match fs::read_to_string(file_path) {
        Ok(content) => {
            log::debug!("successfully read token-key-file '{file_path}'");
            Secret::from(content)
        }
        Err(e) => {
            log::error!("Failed read token-key-file '{file_path}'");
            log::error!("{e}");
            process::exit(1);
        }
    }
});
