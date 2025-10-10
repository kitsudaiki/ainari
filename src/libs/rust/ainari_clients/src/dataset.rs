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
use uuid::Uuid;

use ainari_api_structs::dataset_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;

use crate::prepare_client;

pub async fn init_dataset(
    bento_endpoint: &ainari_config::BentoEndpoints,
    token: &String,
    dataset_uuid: &Uuid,
    name: &str,
    insecure_client: bool,
) -> Result<DatasetInternalResp, AinariError> {
    let address = bento_endpoint.public_address.clone();
    let port = bento_endpoint.public_port;
    let https_connection = address.starts_with("https://");
    let client = prepare_client(https_connection, insecure_client);
    let address_complete = format!("{address}:{port}/v1alpha/dataset/internal");

    let body = DatasetCreateReq {
        uuid: *dataset_uuid,
        name: name.to_owned(),
        dataset_type: "csv".to_string(),
    };
    let json_str = serde_json::to_string(&body).unwrap();

    let response = client
        .post(address_complete)
        .insert_header(("Authorization", format!("Bearer {}", token)))
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
                    let deserialized: DatasetInternalResp = match serde_json::from_str(&body_str) {
                        Ok(body) => body,
                        Err(e) => {
                            let msg = format!("Error while creating dataset: {e}");
                            return Err(AinariError::Error(msg));
                        }
                    };

                    Ok(deserialized)
                }
                code => {
                    let msg = format!("Error while creating dataset. Got response-code: {code}");
                    Err(AinariError::Error(msg))
                }
            }
        }
        Err(e) => {
            let msg = format!("Error while creating dataset: {e}");
            Err(AinariError::Error(msg))
        }
    }
}

pub async fn get_dataset(
    bento_endpoint: &ainari_config::BentoEndpoints,
    token: &String,
    dataset_uuid: &Uuid,
    insecure_client: bool,
) -> Result<DatasetInternalResp, AinariError> {
    let address = bento_endpoint.public_address.clone();
    let port = bento_endpoint.public_port;
    let https_connection = address.starts_with("https://");
    let client = prepare_client(https_connection, insecure_client);
    let address_complete = format!("{address}:{port}/v1alpha/dataset/{dataset_uuid}/internal");

    let response = client
        .get(address_complete)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .send()
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
                StatusCode::OK => {
                    let deserialized: DatasetInternalResp = match serde_json::from_str(&body_str) {
                        Ok(body) => body,
                        Err(e) => {
                            let msg = format!("Error while creating dataset {dataset_uuid} : {e}");
                            return Err(AinariError::Error(msg));
                        }
                    };

                    Ok(deserialized)
                }
                code => {
                    let msg = format!(
                        "Error while getting dataset {dataset_uuid}. Got response-code: {code}"
                    );
                    Err(AinariError::Error(msg))
                }
            }
        }
        Err(e) => {
            let msg = format!("Error while getting dataset {dataset_uuid} : {e}");
            Err(AinariError::Error(msg))
        }
    }
}
