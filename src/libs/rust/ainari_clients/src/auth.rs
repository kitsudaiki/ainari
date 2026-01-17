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

use crate::prepare_client;
use ainari_common::error::AinariError;
use awc::http::StatusCode;

/// Asynchronously checks the validity of a token against a Miko server.
///
/// This function makes an HTTP GET request to the Miko server's token validation endpoint.
/// The request includes the provided token in the Authorization header.
///
/// # Arguments
///
/// * `address` - The base URL of the Miko server.
/// * `token` - The token to be validated.
/// * `insecure_client` - A boolean indicating whether to use an insecure client (for testing purposes).
///
/// # Returns
///
/// * `Result<String, AinariError>` - On success, returns the response body as a String.
///   On failure, returns an `AinariError` with appropriate error details.
pub async fn check_token(
    address: String,
    token: String,
    insecure_client: bool,
) -> Result<String, AinariError> {
    // Prepare the HTTP client with the given address and security settings
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/token");

    // Send the GET request with the token in the Authorization header
    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .send()
        .await;

    // Handle the response from the server
    match response {
        Ok(mut resp) => {
            // Check the HTTP status code of the response
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
                    return Err(AinariError::InternalError("".to_string()));
                }
            }

            // Extract and process the response body
            let body_str = match resp.body().await {
                Ok(body) => String::from_utf8_lossy(&body).into_owned(),
                Err(e) => {
                    log::error!("Error while getting token-validation-body: {e}");
                    return Err(AinariError::InternalError("".to_string()));
                }
            };
            Ok(body_str)
            // println!("Success: {body_str}");
        }
        Err(e) => {
            log::error!("Error while sending request for token-validation: {e}");
            Err(AinariError::InternalError("".to_string()))
        }
    }
}
