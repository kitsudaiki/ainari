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

pub mod auth;
pub mod checkpoint;
pub mod dataset;
pub mod endpoints;
pub mod host;
pub mod model;
pub mod onsen_file_transfer;
pub mod proxy;
pub mod quota;
pub mod secret;

use actix_web::dev::{Decompress, Payload};
use awc::error::SendRequestError;
use awc::http::StatusCode;
use awc::{Client, Connector};
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use std::time::Duration;

use ainari_common::error::AinariError;

/// Prepares an HTTP client with optional SSL configuration.
///
/// # Arguments
///
/// * `address` - The address to connect to, used to determine if SSL should be used.
/// * `insecure` - If true, creates an insecure SSL connection that doesn't verify certificates.
///
/// # Returns
///
/// A configured `awc::Client` instance.
pub fn prepare_client(address: &str, insecure: bool) -> Client {
    // Determine if SSL should be used based on the address prefix
    let use_ssl = address.starts_with("https://");

    if use_ssl {
        // Create SSL connector with appropriate security settings
        let mut ssl_builder = SslConnector::builder(SslMethod::tls()).unwrap();

        // Configure insecure SSL if requested
        if insecure {
            ssl_builder.set_verify(SslVerifyMode::NONE);
            ssl_builder.set_verify_callback(SslVerifyMode::NONE, |_, _| true);
        }

        // Create connector with SSL configuration and build the client
        let connector = Connector::new().openssl(ssl_builder.build());
        Client::builder()
            .connector(connector) // pass connector directly
            .timeout(Duration::from_secs(60))
            .finish()
    } else {
        // Return a regular HTTP client for non-HTTPS connections
        Client::builder().timeout(Duration::from_secs(60)).finish()
    }
}

/// Handles an API response and attempts to deserialize it into the specified type.
///
/// # Arguments
///
/// * `response` - The response from the API call.
/// * `obj` - A string describing the type of object being requested.
/// * `uuid` - An optional UUID for the object being requested.
///
/// # Returns
///
/// A `Result` containing the deserialized object or an error.
///
/// # Type Parameters
///
/// * `T` - The type to deserialize the response into, must implement `serde::de::DeserializeOwned`.
pub async fn handle_response<T>(
    response: Result<awc::ClientResponse<Decompress<Payload>>, SendRequestError>,
    obj: &str,
    uuid: &str,
) -> Result<T, AinariError>
where
    T: serde::de::DeserializeOwned,
{
    match response {
        Ok(mut resp) => {
            // Extract the response body as a string
            let body_str = match resp.body().await {
                Ok(body) => String::from_utf8_lossy(&body).into_owned(),
                Err(e) => {
                    log::error!("Error while getting token-validation-body: {e}");
                    return Err(AinariError::InternalError("".to_string()));
                }
            };

            // Handle different HTTP status codes
            match resp.status() {
                // Handle unauthorized/forbidden responses
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                    Err(AinariError::Unauthorized("Invalid token".to_string()))
                }
                // Handle bad request responses
                StatusCode::BAD_REQUEST => Err(AinariError::InvalidInput(body_str)),
                // Handle successful responses
                StatusCode::OK | StatusCode::CREATED => {
                    // Attempt to deserialize the response body
                    let deserialized: T = match serde_json::from_str(&body_str) {
                        Ok(body) => body,
                        Err(e) => {
                            // Create error message with or without UUID
                            if uuid.is_empty() {
                                let msg = format!("Error while converting response of {obj} : {e}");
                                return Err(AinariError::InternalError(msg));
                            } else {
                                let msg = format!(
                                    "Error while converting response of {obj} with uuid '{uuid}' : {e}"
                                );
                                return Err(AinariError::InternalError(msg));
                            }
                        }
                    };

                    Ok(deserialized)
                }
                // Handle unexpected status codes
                code => {
                    // Create error message with or without UUID
                    if uuid.is_empty() {
                        let msg = format!("Error while creating {obj}. Got response-code: {code}");
                        Err(AinariError::InternalError(msg))
                    } else {
                        let msg = format!(
                            "Error while getting {obj} with uuid '{uuid}'. Got response-code: {code}"
                        );
                        Err(AinariError::InternalError(msg))
                    }
                }
            }
        }
        // Handle request errors
        Err(e) => {
            let msg = format!("Error while getting {obj} with uuid '{uuid}' : {e}");
            Err(AinariError::InternalError(msg))
        }
    }
}

/// Handles an API response that doesn't return a body.
///
/// # Arguments
///
/// * `response` - The response from the API call.
/// * `obj` - A string describing the type of object being requested.
/// * `uuid` - An optional UUID for the object being requested.
///
/// # Returns
///
/// A `Result` indicating success or an error.
pub async fn handle_empty_response(
    response: Result<awc::ClientResponse<Decompress<Payload>>, SendRequestError>,
    obj: &str,
    uuid: &str,
) -> Result<(), AinariError> {
    match response {
        Ok(mut resp) => {
            // Extract the response body as a string
            let body_str = match resp.body().await {
                Ok(body) => String::from_utf8_lossy(&body).into_owned(),
                Err(e) => {
                    log::error!("Error while getting token-validation-body: {e}");
                    return Err(AinariError::InternalError("".to_string()));
                }
            };

            // Handle different HTTP status codes
            match resp.status() {
                // Handle unauthorized/forbidden responses
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                    Err(AinariError::Unauthorized("Invalid token".to_string()))
                }
                // Handle bad request responses
                StatusCode::BAD_REQUEST => Err(AinariError::InvalidInput(body_str)),
                // Handle successful responses with no content
                StatusCode::NO_CONTENT => Ok(()),
                // Handle unexpected status codes
                code => {
                    let msg = format!(
                        "Error while getting {obj} with uuid '{uuid}'. Got response-code: {code}"
                    );
                    Err(AinariError::InternalError(msg))
                }
            }
        }
        // Handle request errors
        Err(e) => {
            let msg = format!("Error while getting {obj} with uuid '{uuid}' : {e}");
            Err(AinariError::InternalError(msg))
        }
    }
}
