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

pub mod auth;
pub mod checkpoint;
pub mod cluster;
pub mod dataset;
pub mod endpoints;
pub mod host;
pub mod proxy;
pub mod quota;

use awc::http::StatusCode;
use awc::{Client, Connector};
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use std::time::Duration;

use ainari_common::error::AinariError;

pub fn prepare_client(use_ssl: bool, insecure: bool) -> Client {
    if use_ssl {
        let mut ssl_builder = SslConnector::builder(SslMethod::tls()).unwrap();

        if insecure {
            ssl_builder.set_verify(SslVerifyMode::NONE);
            ssl_builder.set_verify_callback(SslVerifyMode::NONE, |_, _| true);
        }

        let connector = Connector::new().openssl(ssl_builder.build());
        Client::builder()
            .connector(connector) // pass connector directly
            .timeout(Duration::from_secs(60))
            .finish()
    } else {
        Client::new()
    }
}

pub async fn handle_response<T>(
    response: Result<
        awc::ClientResponse<actix_web::dev::Decompress<actix_web::dev::Payload>>,
        awc::error::SendRequestError,
    >,
    obj: &str,
    uuid: &str,
) -> Result<T, AinariError>
where
    T: serde::de::DeserializeOwned,
{
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
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                    Err(AinariError::Unauthorized("Invalid token".to_string()))
                }
                StatusCode::BAD_REQUEST => Err(AinariError::InvalidInput(body_str)),
                StatusCode::OK | StatusCode::CREATED => {
                    let deserialized: T = match serde_json::from_str(&body_str) {
                        Ok(body) => body,
                        Err(e) => {
                            if uuid.len() == 0 {
                                let msg = format!("Error while converting response of {obj} : {e}");
                                return Err(AinariError::Error(msg));
                            } else {
                                let msg = format!(
                                    "Error while converting response of {obj} with uuid '{uuid}' : {e}"
                                );
                                return Err(AinariError::Error(msg));
                            }
                        }
                    };

                    Ok(deserialized)
                }
                code => {
                    if uuid.len() == 0 {
                        let msg = format!("Error while creating {obj}. Got response-code: {code}");
                        Err(AinariError::Error(msg))
                    } else {
                        let msg = format!(
                            "Error while getting {obj} with uuid '{uuid}'. Got response-code: {code}"
                        );
                        Err(AinariError::Error(msg))
                    }
                }
            }
        }
        Err(e) => {
            let msg = format!("Error while getting {obj} with uuid '{uuid}' : {e}");
            Err(AinariError::Error(msg))
        }
    }
}

pub async fn handle_empty_response(
    response: Result<
        awc::ClientResponse<actix_web::dev::Decompress<actix_web::dev::Payload>>,
        awc::error::SendRequestError,
    >,
    obj: &str,
    uuid: &str,
) -> Result<(), AinariError> {
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
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                    Err(AinariError::Unauthorized("Invalid token".to_string()))
                }
                StatusCode::BAD_REQUEST => Err(AinariError::InvalidInput(body_str)),
                StatusCode::NO_CONTENT => Ok(()),
                code => {
                    let msg = format!(
                        "Error while getting {obj} with uuid '{uuid}'. Got response-code: {code}"
                    );
                    Err(AinariError::Error(msg))
                }
            }
        }
        Err(e) => {
            let msg = format!("Error while getting {obj} with uuid '{uuid}' : {e}");
            Err(AinariError::Error(msg))
        }
    }
}
