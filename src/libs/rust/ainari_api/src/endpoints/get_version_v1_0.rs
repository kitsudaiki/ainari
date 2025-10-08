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
use apistos::api_operation;
use once_cell::sync::Lazy;

use crate::errors::ErrorResponse;
use ainari_api_structs::common_structs::VersionResp;
use ainari_api_structs::user_context::UserContext;

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
        version: GIT_VERSION.clone(),
        commit_hash: COMMIT_HASH.clone(),
        timestamp: COMPILE_TIMESTAMP.clone(),
    };

    return Ok(Json(resp));
}

pub static GIT_VERSION: Lazy<String> = Lazy::new(|| {
    let git_version = option_env!("GIT_VERSION").unwrap_or("unknown");
    git_version.to_string()
});

pub static COMMIT_HASH: Lazy<String> = Lazy::new(|| {
    let commit_hash = option_env!("COMMIT_HASH").unwrap_or("unknown");
    commit_hash.to_string()
});

pub static COMPILE_TIMESTAMP: Lazy<String> = Lazy::new(|| {
    let compile_timestamp = option_env!("COMPILE_TIMESTAMP").unwrap_or("unknown");
    compile_timestamp.to_string()
});
