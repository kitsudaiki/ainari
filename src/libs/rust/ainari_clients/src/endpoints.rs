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

use ainari_api_structs::endpoints_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;

use crate::prepare_client;

pub async fn get_endpoints(
    miko_endpoint: &ainari_config::MikoEndpoint,
    insecure_client: bool,
) -> Result<ainari_config::Endpoints, AinariError> {
    let address = miko_endpoint.address.clone();
    let port = miko_endpoint.port;
    let https_connection = address.starts_with("https://");
    let client = prepare_client(https_connection, insecure_client);
    let address_complete = format!("{address}:{port}/v1alpha/endpoints");

    let response = client.get(address_complete).send().await;

    match response {
        Ok(mut resp) => {
            let body_str = match resp.body().await {
                Ok(body) => String::from_utf8_lossy(&body).into_owned(),
                Err(e) => {
                    log::error!("Error while getting endpoint-body: {e}");
                    return Err(AinariError::Error("".to_string()));
                }
            };

            match resp.status() {
                StatusCode::BAD_REQUEST => Err(AinariError::InvalidInput(body_str)),
                StatusCode::OK => {
                    let deserialized: EndpontsResp = match serde_json::from_str(&body_str) {
                        Ok(body) => body,
                        Err(e) => {
                            let msg = format!("Error while requesting endpoints from miko: {e}");
                            return Err(AinariError::Error(msg));
                        }
                    };

                    // converting
                    let endpoints = ainari_config::Endpoints {
                        hanami: ainari_config::Endpoint {
                            public_address: deserialized.hanami.public_address,
                            public_port: deserialized.hanami.public_port,
                            internal_address: deserialized.hanami.internal_address,
                            internal_port: deserialized.hanami.internal_port,
                        },
                        bento: ainari_config::Endpoint {
                            public_address: deserialized.bento.public_address,
                            public_port: deserialized.bento.public_port,
                            internal_address: deserialized.bento.internal_address,
                            internal_port: deserialized.bento.internal_port,
                        },
                        torii: ainari_config::Endpoint {
                            public_address: deserialized.torii.public_address,
                            public_port: deserialized.torii.public_port,
                            internal_address: deserialized.torii.internal_address,
                            internal_port: deserialized.torii.internal_port,
                        },
                        omamori: ainari_config::Endpoint {
                            public_address: deserialized.omamori.public_address,
                            public_port: deserialized.omamori.public_port,
                            internal_address: deserialized.omamori.internal_address,
                            internal_port: deserialized.omamori.internal_port,
                        },
                    };

                    Ok(endpoints)
                }
                code => {
                    let msg = format!("Error while requesting endpoints from miko: {code}");
                    Err(AinariError::Error(msg))
                }
            }
        }
        Err(e) => {
            let msg = format!("Error while requesting endpoints from miko: {e}");
            Err(AinariError::Error(msg))
        }
    }
}
