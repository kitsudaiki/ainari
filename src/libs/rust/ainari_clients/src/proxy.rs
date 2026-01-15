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

use uuid::Uuid;

use ainari_api_structs::proxy_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::prepare_client;
use crate::{handle_empty_response, handle_response};

pub async fn create_proxy(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    model_uuid: &Uuid,
    target_address: &str,
    insecure_client: bool,
) -> Result<ProxyResp, AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/proxy/internal");

    let body = ProxyCreateReq {
        target_address: target_address.to_owned(),
        model_uuid: *model_uuid,
    };
    let json_str = serde_json::to_string(&body).unwrap();

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    let resp: Result<ProxyResp, AinariError> = handle_response(response, "proxy", "").await;
    resp
}

pub async fn get_proxy(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    proxy_uuid: &Uuid,
    insecure_client: bool,
) -> Result<ProxyResp, AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/proxy/{proxy_uuid}");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .send()
        .await;

    let resp: Result<ProxyResp, AinariError> =
        handle_response(response, "proxy", &proxy_uuid.to_string()).await;
    resp
}

pub async fn list_proxy(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    insecure_client: bool,
) -> Result<ProxyResp, AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/proxy");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .send()
        .await;

    let resp: Result<ProxyResp, AinariError> = handle_response(response, "proxy", "").await;
    resp
}

pub async fn delete_proxy(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    proxy_uuid: &Uuid,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/proxy/{proxy_uuid}/internal");

    let response = client
        .delete(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_empty_response(response, "proxy", &proxy_uuid.to_string()).await
}
