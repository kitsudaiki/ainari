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

use awc::http::StatusCode;

use ainari_api_structs::host_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::prepare_client;

pub async fn register_sakura_host(
    hanami_endpoint: &ainari_config::Endpoint,
    internal_api_key: &Secret,
    name: &str,
    sakura_address: &str,
    registration_key: &Secret,
    insecure_client: bool,
) -> Result<HostResp, AinariError> {
    let address = hanami_endpoint.internal_address.clone();
    let port = hanami_endpoint.internal_port;
    let https_connection = address.starts_with("https://");
    let client = prepare_client(https_connection, insecure_client);

    let body = HostCreateReq {
        name: name.to_owned(),
        sakura_address: sakura_address.to_owned(),
        registration_key: Secret::from(registration_key.reveal()),
    };
    let json_str = serde_json::to_string(&body).unwrap();

    let address_complete = format!("{address}:{port}/v1alpha/host/internal");
    let response = client
        .post(address_complete)
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    match response {
        Ok(mut resp) => {
            let body_str = match resp.body().await {
                Ok(body) => String::from_utf8_lossy(&body).into_owned(),
                Err(e) => {
                    log::error!("Error while getting token-validation-body: {e}");
                    return Err(AinariError::Error("".to_string()));
                }
            };

            match resp.status() {
                StatusCode::UNAUTHORIZED => {
                    Err(AinariError::Unauthorized("Invalid token".to_string()))
                }
                StatusCode::FORBIDDEN => {
                    Err(AinariError::Unauthorized("Invalid token".to_string()))
                }
                StatusCode::BAD_REQUEST => Err(AinariError::InvalidInput(body_str)),
                StatusCode::CREATED => {
                    let deserialized: HostResp = match serde_json::from_str(&body_str) {
                        Ok(body) => body,
                        Err(e) => {
                            let msg = format!("Error while creating host: {e}");
                            return Err(AinariError::Error(msg));
                        }
                    };

                    Ok(deserialized)
                }
                code => {
                    let msg = format!("Error while creating host. Got response-code: {code}");
                    Err(AinariError::Error(msg))
                }
            }
        }
        Err(e) => {
            let msg = format!("Error while creating host: {e}");
            Err(AinariError::Error(msg))
        }
    }
}
