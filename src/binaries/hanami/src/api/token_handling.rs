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

use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};

use crate::api::user_context::UserContext;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Claims {
    pub user_id: String,
    pub project_id: String,
    pub is_admin: bool,
    pub is_project_admin: bool,
    //pub aud: String,         // Optional. Audience
    pub exp: usize, // Required (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
    pub iat: usize, // Optional. Issued at (as UTC timestamp)
    pub iss: String, // Optional. Issuer
                    //pub nbf: usize,          // Optional. Not Before (as UTC timestamp)
                    //pub sub: String,         // Optional. Subject (whom token refers to)
}

pub fn decode_jwt_payload(token: &str) -> Result<UserContext, Box<dyn std::error::Error>> {
    // split into parts: header.payload.signature
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("Invalid token format".into());
    }

    // decode the payload (2nd part)
    let decoded = general_purpose::URL_SAFE_NO_PAD.decode(parts[1])?;
    let claims: UserContext = serde_json::from_slice(&decoded)?;

    Ok(claims)
}
