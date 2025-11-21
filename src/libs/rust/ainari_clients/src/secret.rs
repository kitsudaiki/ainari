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

use uuid::Uuid;

use ainari_api_structs::secret_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;

use crate::handle_empty_response;
use crate::handle_response;
use crate::prepare_client;

pub async fn generate_secret(
    omamori_endpoint: &ainari_config::Endpoint,
    token: &String,
    name: &str,
    insecure_client: bool,
) -> Result<SecretResp, AinariError> {
    let address = omamori_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/secret/generate");

    let body = SecretGenerateReq {
        name: name.to_owned(),
    };
    let json_str = serde_json::to_string(&body).unwrap();

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    let resp: Result<SecretResp, AinariError> = handle_response(response, "secret", "").await;
    resp
}

pub async fn get_secret_payload(
    omamori_endpoint: &ainari_config::Endpoint,
    token: &String,
    secret_uuid: &Uuid,
    insecure_client: bool,
) -> Result<SecretWithPayloadResp, AinariError> {
    let address = omamori_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/secret/{secret_uuid}/payload");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .send()
        .await;

    let resp: Result<SecretWithPayloadResp, AinariError> =
        handle_response(response, "secret", &secret_uuid.to_string()).await;
    resp
}

pub async fn delete_secret(
    omamori_endpoint: &ainari_config::Endpoint,
    token: &String,
    secret_uuid: &Uuid,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let address = omamori_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/secret/{secret_uuid}");

    let response = client
        .delete(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .send()
        .await;

    handle_empty_response(response, "secret", &secret_uuid.to_string()).await
}
