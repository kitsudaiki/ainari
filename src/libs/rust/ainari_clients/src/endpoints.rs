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

use awc::http::StatusCode;

use ainari_api_structs::endpoints_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;

use crate::prepare_client;

/// Fetches endpoint information from the Miko service.
///
/// This function makes an HTTP GET request to the Miko service to retrieve endpoint information
/// for various components in the system. It handles the response, deserializes the JSON body,
/// and converts it into a configuration structure.
///
/// # Arguments
///
/// * `miko_endpoint` - Reference to the Miko service endpoint configuration
/// * `insecure_client` - Boolean indicating whether to create an insecure client (without TLS verification)
///
/// # Returns
///
/// * `Result<ainari_config::Endpoints, AinariError>` - On success, returns the deserialized endpoint configuration.
///   On failure, returns an appropriate AinariError variant.
pub async fn get_endpoints(
    miko_endpoint: &ainari_config::MikoEndpoint,
    insecure_client: bool,
) -> Result<ainari_config::Endpoints, AinariError> {
    // Clone the address from the endpoint configuration
    let address = miko_endpoint.address.clone();
    // Prepare the HTTP client with the given address and security settings
    let client = prepare_client(&address, insecure_client);
    // Construct the URL for the endpoints API
    let url = format!("{address}/v1alpha/endpoints");

    // Send the GET request to the Miko service
    let response = client.get(url).send().await;

    match response {
        Ok(mut resp) => {
            // Extract the response body as a string
            let body_str = match resp.body().await {
                Ok(body) => String::from_utf8_lossy(&body).into_owned(),
                Err(e) => {
                    log::error!("Error while getting endpoint-body: {e}");
                    return Err(AinariError::InternalError("".to_string()));
                }
            };

            // Handle different HTTP status codes from the response
            match resp.status() {
                StatusCode::BAD_REQUEST => {
                    // Return an error if the request was malformed
                    Err(AinariError::InvalidInput(body_str))
                }
                StatusCode::OK => {
                    // Deserialize the JSON response body into the EndpontsResp structure
                    let deserialized: EndpontsResp = match serde_json::from_str(&body_str) {
                        Ok(body) => body,
                        Err(e) => {
                            let msg = format!("Error while requesting endpoints from miko: {e}");
                            return Err(AinariError::InternalError(msg));
                        }
                    };

                    // Convert the deserialized response into the final configuration structure
                    let endpoints = ainari_config::Endpoints {
                        hanami: ainari_config::Endpoint {
                            public_address: deserialized.hanami.public_address,
                            internal_address: deserialized.hanami.internal_address,
                        },
                        ryokan: ainari_config::Endpoint {
                            public_address: deserialized.ryokan.public_address,
                            internal_address: deserialized.ryokan.internal_address,
                        },
                        torii: ainari_config::Endpoint {
                            public_address: deserialized.torii.public_address,
                            internal_address: deserialized.torii.internal_address,
                        },
                        omamori: ainari_config::Endpoint {
                            public_address: deserialized.omamori.public_address,
                            internal_address: deserialized.omamori.internal_address,
                        },
                    };

                    Ok(endpoints)
                }
                code => {
                    // Return an error for any unexpected status code
                    let msg = format!("Error while requesting endpoints from miko: {code}");
                    Err(AinariError::InternalError(msg))
                }
            }
        }
        Err(e) => {
            // Return an error if the HTTP request failed
            let msg = format!("Error while requesting endpoints from miko: {e}");
            Err(AinariError::InternalError(msg))
        }
    }
}
