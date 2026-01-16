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

/// Represents the claims contained within a JSON Web Token (JWT).
///
/// This struct contains all the necessary claims for authentication and authorization
/// in the Ainari system. The fields map directly to JWT standard claims with some
/// additional custom claims specific to our application.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Claims {
    /// Unique identifier for the user
    pub user_id: String,
    /// Unique identifier for the project
    pub project_id: String,
    /// Flag indicating if the user has admin privileges
    pub is_admin: String,
    /// Flag indicating if the user has admin privileges for the specific project
    pub is_project_admin: String,
    /// Expiration time (as UTC timestamp in seconds)
    pub exp: usize,
    /// Issued at time (as UTC timestamp in seconds)
    pub iat: usize,
    /// Issuer of the token
    pub iss: String,
}

/// Validates a JSON Web Token (JWT) and extracts the user context.
///
/// This function takes a JWT string, validates it using the configured secret key,
/// and returns a UserContext if the token is valid. The token is validated against
/// the HS256 algorithm.
///
/// # Arguments
///
/// * `token` - The JWT string to validate
///
/// # Returns
///
/// * `Ok(UserContext)` - If the token is valid and contains a proper user context
/// * `Err(String)` - If the token is invalid or expired
pub fn validate_token(token: &str) -> Result<UserContext, String> {
    // validate token
    let key = DecodingKey::from_secret(TOKEN_KEY.reveal().as_bytes());
    let validation = Validation::new(Algorithm::HS256);
    match decode::<UserContext>(token, &key, &validation) {
        Ok(context) => {
            // Attach the original token to the user context for future use
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

/// Creates a new JSON Web Token (JWT) for a user.
///
/// This function generates a signed JWT containing the user's claims. The token
/// will expire after the time specified in the configuration.
///
/// # Arguments
///
/// * `user_id` - The unique identifier for the user
/// * `project_id` - The unique identifier for the project
/// * `is_admin` - Flag indicating if the user has admin privileges
/// * `is_project_admin` - Flag indicating if the user has admin privileges for the specific project
///
/// # Returns
///
/// * `Ok(String)` - The generated JWT string if successful
/// * `Err(())` - If token creation fails
pub fn create_token(
    user_id: &String,
    project_id: &String,
    is_admin: &str,
    is_project_admin: &str,
) -> Result<String, ()> {
    let token_expire_time = config::CONFIG.auth.token_expire_time;

    // get timestamps for token
    // current time in seconds since UNIX epoch
    let current = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    // expiration time is current time plus the configured token expiration time
    let expiration = current + token_expire_time;

    // create token-payload with all required claims
    let claims = Claims {
        user_id: user_id.clone(),
        project_id: project_id.clone(),
        is_admin: is_admin.to_string(),
        is_project_admin: is_project_admin.to_string(),
        exp: expiration as usize,
        iat: current as usize,
        iss: "miko".to_string(),
    };

    // create token with the specified header, claims, and secret key
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

/// Lazy-initialized secret key used for JWT signing and verification.
///
/// This static variable reads the token secret key from the configured file path
/// when first accessed. The key is stored securely using the Secret type from the
/// ainari_common crate.
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
