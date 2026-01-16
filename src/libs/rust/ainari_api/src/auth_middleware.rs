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

use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    http::Method,
    middleware::Next,
    web,
};

use crate::errors::ErrorResponse;

use ainari_clients::auth::check_token;
use ainari_common::config::{Api, MikoEndpoint};
use ainari_common::error::AinariError;
use ainari_common::functions::split_bearer_token;
use ainari_common::secret::*;

/// Configuration structure for API validation middleware
///
/// This struct contains all necessary configuration parameters for validating API requests,
/// including Miko service address, internal IP, API key, and TLS verification settings.
#[derive(Debug, Clone)]
pub struct ApiValidationConfig {
    /// Address of the Miko service for token validation
    pub miko_address: String,
    /// Internal IP address used for request validation
    pub internal_ip: String,
    /// Secret API key for internal requests
    pub internal_api_key: Secret,
    /// Flag to skip TLS verification when communicating with Miko
    pub skip_tls_verification: bool,
}

impl ApiValidationConfig {
    /// Creates a new ApiValidationConfig instance
    ///
    /// # Arguments
    ///
    /// * `conn` - MikoEndpoint configuration containing the address
    /// * `api` - Api configuration containing the internal IP
    /// * `internal_api_key` - Secret API key for internal requests
    /// * `skip_tls_verification` - Flag to skip TLS verification
    ///
    /// # Returns
    ///
    /// Newly created ApiValidationConfig instance
    pub fn new(
        conn: &MikoEndpoint,
        api: &Api,
        internal_api_key: &Secret,
        skip_tls_verification: bool,
    ) -> Self {
        ApiValidationConfig {
            miko_address: conn.address.clone(),
            internal_ip: api.internal_ip.clone(),
            internal_api_key: internal_api_key.clone(),
            skip_tls_verification,
        }
    }
}

/// Middleware for authorizing incoming API requests
///
/// This middleware checks for valid authentication tokens and verifies internal requests.
/// It handles various special cases where authentication might be skipped.
///
/// # Arguments
///
/// * `req` - The incoming service request
/// * `next` - The next middleware or handler in the chain
///
/// # Returns
///
/// Either the service response or an error if authorization fails
pub async fn authorization_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let mut skip_internal_endpoint_check = false;
    let mut skip_token_check = false;
    let uri = req.uri();
    let api_validation_config = req
        .app_data::<web::Data<ApiValidationConfig>>()
        .expect("Api-validation-config missing!");

    log::debug!("call uri: '{uri}' for method: '{}'", *req.method());

    // request of ready-status can be done without token
    skip_token_check |= uri == "/v1alpha/is_ready" && *req.method() == Method::GET;
    // request of openapi-specs can be done without token
    skip_token_check |= uri == "/openapi.json";
    // sakura-hosts can call a registration without token, because it is triggered by themself
    // without user-interaction, but this call is saved by the internal-key and registration-key,
    // which are provided by the sakura-hosts and validated in the endpoint
    skip_token_check |= uri == "/v1alpha/host/internal" && *req.method() == Method::POST;
    // options-request used by browsers also need no checks to be done
    skip_token_check |= *req.method() == Method::OPTIONS;
    skip_internal_endpoint_check |= *req.method() == Method::OPTIONS;

    if !skip_internal_endpoint_check {
        check_internal_request(&req, api_validation_config)?;
    }
    if !skip_token_check {
        check_auth_header(&req, api_validation_config).await?;
    }
    //else {
    //    log::debug!("skip token-check");
    //}

    log::info!("Api-call against URI: {uri}");

    let resp = next.call(req).await;

    match resp {
        Ok(_) => {}
        Err(ref e) => {
            log::info!("{e}");
        }
    };

    resp
}

/// Validates that an internal request is coming from a trusted source
///
/// This function checks the X-Internal-API-Key header against the configured internal API key.
/// It's used to verify requests to internal endpoints.
///
/// # Arguments
///
/// * `req` - The incoming service request to validate
/// * `api_validation_config` - Configuration containing the valid internal API key
///
/// # Returns
///
/// Ok(()) if the request is valid, or an error if authorization fails
pub fn check_internal_request(
    req: &ServiceRequest,
    api_validation_config: &ApiValidationConfig,
) -> Result<(), actix_web::Error> {
    let uri = req.uri();
    let uri_str = format!("{uri}");

    // get interface-address, where the request came in
    let peer_addr = match req.connection_info().peer_addr() {
        Some(peer_addr) => peer_addr.to_owned(),
        _ => "unknown_peer".to_owned(),
    };
    let host_info = req.connection_info().host().to_owned();
    log::debug!(
        "call uri: '{uri}' over host '{}' and peer '{}' for method: '{}'",
        host_info,
        peer_addr,
        *req.method()
    );

    if uri_str.to_lowercase().ends_with("internal") {
        // get token from header
        let api_key_header = match req.headers().get("X-Internal-API-Key") {
            Some(value) => value,
            _ => {
                log::debug!(
                    "API-Key-header not set, even it is required for the internal API-call"
                );
                return Err(ErrorResponse::Unauthorized(
                    "API-Key-header not set, even it is required for the internal API-call"
                        .to_string(),
                )
                .into());
            }
        };

        // convert into string
        let api_key_str = match api_key_header.to_str() {
            Ok(api_key_str) => Secret::from(api_key_str),
            Err(_) => {
                log::debug!("Bad api-key-header");
                return Err(ErrorResponse::Unauthorized("Bad api-key-header".to_string()).into());
            }
        };

        // check key
        if api_key_str != api_validation_config.internal_api_key {
            return Err(ErrorResponse::Unauthorized("Invalid internal API-key".to_string()).into());
        }
    }

    Ok(())
}

/// Validates the authentication token in the Authorization header
///
/// This function extracts the token from the Authorization header, sends it to Miko for validation,
/// and handles the response. It's used to verify requests to non-internal endpoints.
///
/// # Arguments
///
/// * `req` - The incoming service request to validate
/// * `api_validation_config` - Configuration containing Miko address and other settings
///
/// # Returns
///
/// Ok(()) if the token is valid, or an error if authorization fails
async fn check_auth_header(
    req: &ServiceRequest,
    api_validation_config: &ApiValidationConfig,
) -> Result<(), actix_web::Error> {
    let uri = req.uri();

    log::debug!("Check token for request against {uri}");

    // get token from header
    let auth_header = match req.headers().get("Authorization") {
        Some(value) => value,
        _ => {
            return Err(
                ErrorResponse::Unauthorized("Authorization-header not set".to_string()).into(),
            );
        }
    };

    // convert into string
    let auth_header_str = match auth_header.to_str() {
        Ok(auth_header_str) => auth_header_str,
        Err(_) => {
            return Err(ErrorResponse::Unauthorized("Bad auth-header".to_string()).into());
        }
    };

    // parse token from the auth-header
    let token = match split_bearer_token(auth_header_str) {
        Some(token) => token,
        None => {
            log::debug!("Invalid token format");
            return Err(ErrorResponse::Unauthorized("Missing token in header".to_string()).into());
        }
    };

    let miko_address = api_validation_config.miko_address.clone();
    let response = check_token(
        miko_address,
        token.to_string(),
        api_validation_config.skip_tls_verification,
    )
    .await;

    match response {
        Ok(_) => {
            // println!("Success: {body_str}");
            Ok(())
        }
        Err(AinariError::Unauthorized(msg)) => Err(ErrorResponse::Unauthorized(msg).into()),
        Err(AinariError::InvalidInput(msg)) => Err(ErrorResponse::Unauthorized(msg).into()),
        Err(AinariError::InternalError(msg)) => {
            log::error!("Failed check token against Miko with error: '{msg}'");
            Err(ErrorResponse::InternalError("".to_string()).into())
        }
    }
}
