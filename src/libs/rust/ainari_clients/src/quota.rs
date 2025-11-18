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

use ainari_api_structs::quota_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;

use crate::handle_response;
use crate::prepare_client;

pub async fn get_quota(
    miko_endpoint: &ainari_config::MikoEndpoint,
    token: &String,
    user_id: &str,
    insecure_client: bool,
) -> Result<QuotaResp, AinariError> {
    let address = miko_endpoint.address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/quota");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .send()
        .await;

    let resp: Result<QuotaResp, AinariError> = handle_response(response, "quota", user_id).await;
    resp
}
