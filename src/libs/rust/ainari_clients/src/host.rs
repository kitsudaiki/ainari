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

use ainari_api_structs::host_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::handle_response;
use crate::prepare_client;

pub async fn register_sakura_host(
    hanami_endpoint: &ainari_config::Endpoint,
    internal_api_key: &Secret,
    name: &str,
    sakura_address: &str,
    deleted_uuids: UuidList,
    registration_key: &Secret,
    insecure_client: bool,
) -> Result<HostResp, AinariError> {
    let address = hanami_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/host/internal");

    let body = HostCreateReq {
        name: name.to_owned(),
        host_address: sakura_address.to_owned(),
        deleted_uuids,
        registration_key: Secret::from(registration_key.reveal()),
    };
    let json_str = serde_json::to_string(&body).unwrap();

    let response = client
        .post(url)
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    let resp: Result<HostResp, AinariError> = handle_response(response, "sakura-host", "").await;
    resp
}

pub async fn register_onsen_host(
    ryokan_endpoint: &ainari_config::Endpoint,
    internal_api_key: &Secret,
    name: &str,
    onsen_address: &str,
    registration_key: &Secret,
    insecure_client: bool,
) -> Result<HostResp, AinariError> {
    let address = ryokan_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/host/internal");
    let empty_default_uuid_list = UuidList::default();

    let body = HostCreateReq {
        name: name.to_owned(),
        host_address: onsen_address.to_owned(),
        deleted_uuids: empty_default_uuid_list,
        registration_key: Secret::from(registration_key.reveal()),
    };
    let json_str = serde_json::to_string(&body).unwrap();

    let response = client
        .post(url)
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    let resp: Result<HostResp, AinariError> = handle_response(response, "onsen-host", "").await;
    resp
}
