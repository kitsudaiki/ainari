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

use ainari_common::error::AinariError;

use crate::prepare_client;

pub async fn check_token(
    address: String,
    token: String,
    insecure: bool,
) -> Result<String, AinariError> {
    let https_connection = address.starts_with("https://");
    let client = prepare_client(https_connection, insecure);
    let address_complete = format!("{address}/v1alpha/token");

    let response = client
        .get(address_complete)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .send()
        .await;

    match response {
        Ok(mut resp) => {
            match resp.status() {
                StatusCode::UNAUTHORIZED => {
                    log::debug!("Invalid token with 401-error");
                    return Err(AinariError::InvalidInput("Invalid token".to_string()));
                }
                StatusCode::FORBIDDEN => {
                    log::debug!("Invalid token with 403-error");
                    return Err(AinariError::InvalidInput("Invalid token".to_string()));
                }
                StatusCode::OK => {
                    log::debug!("Successfully checked token against Miko");
                }
                code => {
                    log::error!(
                        "Error while sending request for token-validation. Got response-code: {code}"
                    );
                    return Err(AinariError::Error("".to_string()));
                }
            }

            let body_str = match resp.body().await {
                Ok(body) => String::from_utf8_lossy(&body).into_owned(),
                Err(e) => {
                    log::error!("Error while getting token-validation-body: {e}");
                    return Err(AinariError::Error("".to_string()));
                }
            };
            Ok(body_str)
            // println!("Success: {body_str}");
        }
        Err(e) => {
            log::error!("Error while sending request for token-validation: {e}");
            Err(AinariError::Error("".to_string()))
        }
    }
}
