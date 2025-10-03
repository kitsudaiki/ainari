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

use actix_web::web::Json;
use apistos::ApiComponent;
use apistos::api_operation;
use once_cell::sync::Lazy;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct VersionResp {
    pub version: String,
}

#[api_operation(
    tag = "version",
    summary = "Get version",
    description = r###"Get the version of the binary consisting of Tag or Branch-name together with the commit-hash."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn get_version(_: UserContext) -> Result<Json<VersionResp>, ErrorResponse> {
    let resp = VersionResp {
        version: VERSION.clone(),
    };

    return Ok(Json(resp));
}

pub static VERSION: Lazy<String> = Lazy::new(|| {
    let git_version = option_env!("GIT_VERSION").unwrap_or("unknown");
    git_version.to_string()
});
