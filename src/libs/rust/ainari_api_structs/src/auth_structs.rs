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

use apistos::ApiComponent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::user_context::UserContext;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct UserTokenResp {
    pub access_token: String,
    pub token_type: String,
    pub expires: u64,
}

#[derive(Debug, Deserialize, Validate)]
pub struct OAuth2Request {
    pub token_format: String,
    pub grant_type: String,
    #[validate(length(min = 4, max = 127))]
    pub client_id: String,
    #[validate(length(min = 8, max = 4096))]
    pub client_secret: String,
    // pub username: Option<String>,
    // pub password: Option<String>,
    // pub scope: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct UserTokenValidateResp {
    pub context: UserContext,
}
