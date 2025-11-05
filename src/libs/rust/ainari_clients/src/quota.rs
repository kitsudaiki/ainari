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
use ainari_common::secret::Secret;

use crate::handle_response;
use crate::prepare_client;

pub async fn get_quota(
    miko_endpoint: &ainari_config::MikoEndpoint,
    token: &String,
    internal_api_key: &Secret,
    user_id: &String,
    insecure_client: bool,
) -> Result<QuotaResp, AinariError> {
    let address = miko_endpoint.address.clone();
    let port = miko_endpoint.port;
    let https_connection = address.starts_with("https://");
    let client = prepare_client(https_connection, insecure_client);
    let address_complete = format!("{address}:{port}/v1alpha/quota/{user_id}");

    let response = client
        .get(address_complete)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    let resp: Result<QuotaResp, AinariError> = handle_response(response, "quota", user_id).await;
    resp
}
