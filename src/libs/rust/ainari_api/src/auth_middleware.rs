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
use crate::omikuji_logging::{RequestInfo, send_audit_log};

use ainari_api_structs::auth_structs::UserTokenValidateResp;
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
    /// Port of the internal connection. If set, the internal endpoints are only reachable over
    /// the connection with this port and requests over any other connection are rejected.
    pub internal_endpoints_port: Option<u16>,
    /// The internal endpoints are authorized by the internal API-key alone, without the token of
    /// a user. For services, whose internal endpoints are also called by background-jobs of other
    /// services, which act without a user.
    pub internal_endpoints_without_token: bool,
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
            internal_endpoints_port: None,
            internal_endpoints_without_token: false,
        }
    }

    /// Allows the internal endpoints only over the internal connection.
    ///
    /// The server listens on an internal and an external port, which are exposed separately, so
    /// requests from outside of the cluster only reach the external port. With this restriction
    /// the internal endpoints are not reachable over the external port, not even with a valid
    /// internal API-key.
    ///
    /// # Arguments
    ///
    /// * `internal_port` - Port of the internal connection
    ///
    /// # Returns
    ///
    /// The config with the restriction
    pub fn restrict_internal_endpoints(mut self, internal_port: u16) -> Self {
        self.internal_endpoints_port = Some(internal_port);
        self
    }

    /// Authorizes the internal endpoints by the internal API-key alone.
    ///
    /// They still need the internal API-key, and should be restricted to the internal connection
    /// with `restrict_internal_endpoints`, but no token of a user anymore. The endpoints get an
    /// empty user-context then.
    ///
    /// # Returns
    ///
    /// The config without the token-check for the internal endpoints
    pub fn authorize_internal_endpoints_by_api_key(mut self) -> Self {
        self.internal_endpoints_without_token = true;
        self
    }
}

/// Checks if a path belongs to an internal endpoint, which is marked by the suffix `internal`.
///
/// Only the path is checked, so a query-string can not hide the suffix.
pub(crate) fn is_internal_endpoint(path: &str) -> bool {
    path.to_lowercase().ends_with("internal")
}

/// Checks if a request is allowed to access the endpoint over its connection.
///
/// If the config restricts the internal endpoints, they are only allowed over the connection
/// with the internal port. The port is the one of the local socket, where the request came in, so
/// it can not be faked by the client.
///
/// # Arguments
///
/// * `req` - The incoming service request
/// * `api_validation_config` - Configuration with the port of the internal connection
///
/// # Returns
///
/// Ok(()) if the request is allowed, or a forbidden-error, if an internal endpoint was requested
/// over the external connection
pub fn check_internal_endpoint_access(
    req: &ServiceRequest,
    api_validation_config: &ApiValidationConfig,
) -> Result<(), actix_web::Error> {
    let Some(internal_port) = api_validation_config.internal_endpoints_port else {
        return Ok(());
    };

    let local_port = req.app_config().local_addr().port();
    if is_internal_endpoint(req.path()) && local_port != internal_port {
        log::warn!(
            "Rejected request against internal endpoint '{}' over the external port {local_port}",
            req.path()
        );
        return Err(ErrorResponse::Forbidden(
            "Internal endpoints are only reachable over the internal connection".to_string(),
        )
        .into());
    }

    Ok(())
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

    // done before anything else, so an internal endpoint is never reachable over the external
    // connection, also not for the requests, which skip the other checks below
    check_internal_endpoint_access(&req, api_validation_config)?;

    // request of ready-status can be done without token
    skip_token_check |= uri == "/v1alpha/is_ready" && *req.method() == Method::GET;
    // request of openapi-specs can be done without token
    skip_token_check |= uri == "/openapi.json";
    // sakura-hosts can call a registration without token, because it is triggered by themself
    // without user-interaction, but this call is saved by the internal-key and registration-key,
    // which are provided by the sakura-hosts and validated in the endpoint
    skip_token_check |= uri == "/v1alpha/host/internal" && *req.method() == Method::POST;
    // internal endpoints, which are called by background-jobs without a user, are protected by
    // the internal API-key alone, which is checked below
    skip_token_check |=
        api_validation_config.internal_endpoints_without_token && is_internal_endpoint(req.path());
    // options-request used by browsers also need no checks to be done
    skip_token_check |= *req.method() == Method::OPTIONS;
    skip_internal_endpoint_check |= *req.method() == Method::OPTIONS;

    if !skip_internal_endpoint_check {
        check_internal_request(&req, api_validation_config)?;
    }
    if !skip_token_check {
        let project_locked = check_auth_header(&req, api_validation_config).await?;
        check_project_lock(&req, api_validation_config, project_locked)?;
    }
    //else {
    //    log::debug!("skip token-check");
    //}

    log::info!("Api-call against URI: {uri}");
    send_audit_log(&RequestInfo::from_request(&req).await);

    let resp = next.call(req).await;

    match resp {
        Ok(_) => {}
        Err(ref e) => {
            log::info!("{e}");
        }
    };

    resp
}

/// Checks if the method of a request only reads, so it is allowed for a locked project.
fn is_read_request(method: &Method) -> bool {
    matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

/// Checks if a request carries the valid internal API-key, so it comes from another component and
/// not directly from a user.
///
/// # Arguments
///
/// * `req` - The incoming service request
/// * `api_validation_config` - Configuration containing the valid internal API key
///
/// # Returns
///
/// True, if the header `X-Internal-API-Key` contains the valid key
fn has_valid_internal_api_key(
    req: &ServiceRequest,
    api_validation_config: &ApiValidationConfig,
) -> bool {
    req.headers()
        .get("X-Internal-API-Key")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|key| Secret::from(key) == api_validation_config.internal_api_key)
}

/// Rejects all requests of a user to a locked project, which don't only read.
///
/// A project is locked during a backup or restore, so its resources don't change in the meantime.
/// The lock is checked here centrally for all components, so it doesn't have to be implemented in
/// every endpoint. Requests with the internal API-key come from the backup or restore itself or
/// from other components, which act on its behalf, so they are not blocked.
///
/// # Arguments
///
/// * `req` - The incoming service request
/// * `api_validation_config` - Configuration containing the valid internal API key
/// * `project_locked` - True, if the project of the token of the request is locked
///
/// # Returns
///
/// Ok(()) if the request is allowed, or the error `423 Locked`
pub fn check_project_lock(
    req: &ServiceRequest,
    api_validation_config: &ApiValidationConfig,
    project_locked: bool,
) -> Result<(), actix_web::Error> {
    if project_locked
        && !is_read_request(req.method())
        && !has_valid_internal_api_key(req, api_validation_config)
    {
        log::debug!(
            "Rejected '{}'-request against '{}', because the project is locked",
            req.method(),
            req.path()
        );
        return Err(ErrorResponse::Locked(
            "The project is locked by a backup or restore and can only be read at the moment."
                .to_string(),
        )
        .into());
    }

    Ok(())
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

    if is_internal_endpoint(req.path()) {
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
/// Ok(true) if the token is valid and its project is locked, Ok(false) if the token is valid and
/// its project is not locked, or an error if authorization fails
async fn check_auth_header(
    req: &ServiceRequest,
    api_validation_config: &ApiValidationConfig,
) -> Result<bool, actix_web::Error> {
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
        Ok(body) => match serde_json::from_str::<UserTokenValidateResp>(&body) {
            Ok(validation) => Ok(validation.project_locked),
            Err(e) => {
                // without the lock-state, the request can't be checked against the lock
                crate::report_internal_error!(
                    None,
                    "Failed to read token-validation of Miko with error: '{e}'"
                );
                Err(ErrorResponse::InternalError("Internal Error".to_string()).into())
            }
        },
        Err(AinariError::Unauthorized(msg)) => Err(ErrorResponse::Unauthorized(msg).into()),
        Err(AinariError::InvalidInput(msg)) => Err(ErrorResponse::Unauthorized(msg).into()),
        // miko doesn't know the user of the token anymore
        Err(AinariError::NotFound(msg)) => Err(ErrorResponse::Unauthorized(msg).into()),
        Err(AinariError::Forbidden(msg)) => Err(ErrorResponse::Unauthorized(msg).into()),
        Err(e @ (AinariError::Conflict(_) | AinariError::InternalError(_))) => {
            // the token is not validated yet, so the error can not be assigned to its user
            crate::report_internal_error!(
                None,
                "Failed check token against Miko with error: '{e}'"
            );
            Err(ErrorResponse::InternalError("Internal Error".to_string()).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::middleware::from_fn;
    use actix_web::{App, HttpResponse, HttpServer};
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};

    #[test]
    fn test_is_internal_endpoint() {
        assert!(is_internal_endpoint("/v1alpha/host/internal"));
        assert!(is_internal_endpoint("/v1alpha/image/abc/Internal"));
        assert!(!is_internal_endpoint("/v1alpha/host"));
        assert!(!is_internal_endpoint("/v1alpha/internal/host"));
    }

    /// Middleware, which only runs the check of the connection, so the test needs no miko.
    async fn connection_check_middleware(
        req: ServiceRequest,
        next: Next<impl MessageBody>,
    ) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
        let config = req
            .app_data::<web::Data<ApiValidationConfig>>()
            .expect("config missing")
            .clone();
        check_internal_endpoint_access(&req, &config)?;
        next.call(req).await
    }

    /// Sends a GET-request over a plain TCP-connection and returns the status-code.
    fn get_status(port: u16, path: &str) -> u16 {
        get_status_with_headers(port, path, "")
    }

    /// Sends a GET-request with additional header-lines and returns the status-code.
    fn get_status_with_headers(port: u16, path: &str, headers: &str) -> u16 {
        request_status(port, "GET", path, headers)
    }

    /// Sends a request without body with additional header-lines and returns the status-code.
    fn request_status(port: u16, method: &str, path: &str, headers: &str) -> u16 {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("failed to connect");
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\n{headers}Content-Length: 0\r\n\
             Connection: close\r\n\r\n"
        )
        .expect("failed to send request");
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .expect("failed to read response");
        response
            .split_whitespace()
            .nth(1)
            .and_then(|code| code.parse().ok())
            .expect("invalid response")
    }

    #[actix_web::test]
    async fn test_internal_endpoints_only_over_internal_port() {
        let public = TcpListener::bind("127.0.0.1:0").unwrap();
        let internal = TcpListener::bind("127.0.0.1:0").unwrap();
        let public_port = public.local_addr().unwrap().port();
        let internal_port = internal.local_addr().unwrap().port();

        let config = ApiValidationConfig {
            miko_address: String::new(),
            internal_ip: "127.0.0.1".to_string(),
            internal_api_key: Secret::from("key"),
            skip_tls_verification: false,
            internal_endpoints_port: None,
            internal_endpoints_without_token: false,
        }
        .restrict_internal_endpoints(internal_port);

        let server = HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(config.clone()))
                .wrap(from_fn(connection_check_middleware))
                .route("/v1alpha/host", web::get().to(HttpResponse::Ok))
                .route("/v1alpha/host/internal", web::get().to(HttpResponse::Ok))
        })
        .workers(1)
        .listen(public)
        .unwrap()
        .listen(internal)
        .unwrap()
        .run();
        let handle = server.handle();
        actix_rt::spawn(server);

        let results = actix_web::rt::task::spawn_blocking(move || {
            (
                get_status(public_port, "/v1alpha/host"),
                get_status(internal_port, "/v1alpha/host"),
                get_status(public_port, "/v1alpha/host/internal"),
                get_status(public_port, "/v1alpha/host/internal?x=1"),
                get_status(internal_port, "/v1alpha/host/internal"),
            )
        })
        .await
        .unwrap();
        handle.stop(true).await;

        // normal endpoints are reachable over both connections
        assert_eq!(results.0, 200);
        assert_eq!(results.1, 200);
        // internal endpoints only over the internal connection, also with a query-string
        assert_eq!(results.2, 403);
        assert_eq!(results.3, 403);
        assert_eq!(results.4, 200);
    }

    #[actix_web::test]
    async fn test_internal_endpoints_by_api_key_alone() {
        let internal = TcpListener::bind("127.0.0.1:0").unwrap();
        let internal_port = internal.local_addr().unwrap().port();

        let config = ApiValidationConfig {
            miko_address: String::new(),
            internal_ip: "127.0.0.1".to_string(),
            internal_api_key: Secret::from("key"),
            skip_tls_verification: false,
            internal_endpoints_port: None,
            internal_endpoints_without_token: false,
        }
        .restrict_internal_endpoints(internal_port)
        .authorize_internal_endpoints_by_api_key();

        let server = HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(config.clone()))
                .wrap(from_fn(authorization_middleware))
                .route("/v1alpha/host", web::get().to(HttpResponse::Ok))
                .route("/v1alpha/host/internal", web::get().to(HttpResponse::Ok))
        })
        .workers(1)
        .listen(internal)
        .unwrap()
        .run();
        let handle = server.handle();
        actix_rt::spawn(server);

        let results = actix_web::rt::task::spawn_blocking(move || {
            (
                get_status_with_headers(
                    internal_port,
                    "/v1alpha/host/internal",
                    "X-Internal-API-Key: key\r\n",
                ),
                get_status_with_headers(
                    internal_port,
                    "/v1alpha/host/internal",
                    "X-Internal-API-Key: wrong\r\n",
                ),
                get_status(internal_port, "/v1alpha/host/internal"),
                get_status_with_headers(
                    internal_port,
                    "/v1alpha/host",
                    "X-Internal-API-Key: key\r\n",
                ),
            )
        })
        .await
        .unwrap();
        handle.stop(true).await;

        // an internal endpoint needs the internal API-key, but no token
        assert_eq!(results.0, 200);
        assert_ne!(results.1, 200);
        assert_ne!(results.2, 200);
        // every other endpoint still needs the token of a user
        assert_ne!(results.3, 200);
    }

    #[test]
    fn test_is_read_request() {
        assert!(is_read_request(&Method::GET));
        assert!(is_read_request(&Method::HEAD));
        assert!(is_read_request(&Method::OPTIONS));
        assert!(!is_read_request(&Method::POST));
        assert!(!is_read_request(&Method::PUT));
        assert!(!is_read_request(&Method::DELETE));
        assert!(!is_read_request(&Method::PATCH));
    }

    /// Token-validation of a fake miko, which reports the project of the token "locked" as locked.
    async fn fake_validate_token(req: actix_web::HttpRequest) -> HttpResponse {
        let token = req
            .headers()
            .get("Authorization")
            .and_then(|value| value.to_str().ok())
            .and_then(split_bearer_token)
            .unwrap_or_default();
        if token != "locked" && token != "unlocked" {
            return HttpResponse::Unauthorized().finish();
        }
        HttpResponse::Ok().json(serde_json::json!({
            "context": {
                "user_id": "user",
                "project_id": "project",
                "is_admin": "false",
                "project_role": "member",
            },
            "project_locked": token == "locked",
        }))
    }

    #[actix_web::test]
    async fn test_writes_to_locked_project_are_rejected() {
        let miko = TcpListener::bind("127.0.0.1:0").unwrap();
        let miko_port = miko.local_addr().unwrap().port();
        let miko_server = HttpServer::new(|| {
            App::new().route("/v1alpha/token", web::get().to(fake_validate_token))
        })
        .workers(1)
        .listen(miko)
        .unwrap()
        .run();
        let miko_handle = miko_server.handle();
        actix_rt::spawn(miko_server);

        let service = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = service.local_addr().unwrap().port();
        let config = ApiValidationConfig {
            miko_address: format!("http://127.0.0.1:{miko_port}"),
            internal_ip: "127.0.0.1".to_string(),
            internal_api_key: Secret::from("key"),
            skip_tls_verification: false,
            internal_endpoints_port: None,
            internal_endpoints_without_token: false,
        };
        let server = HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(config.clone()))
                .wrap(from_fn(authorization_middleware))
                .route("/v1alpha/vm", web::get().to(HttpResponse::Ok))
                .route("/v1alpha/vm", web::post().to(HttpResponse::Ok))
                .route("/v1alpha/vm", web::delete().to(HttpResponse::Ok))
        })
        .workers(1)
        .listen(service)
        .unwrap()
        .run();
        let handle = server.handle();
        actix_rt::spawn(server);

        let results = actix_web::rt::task::spawn_blocking(move || {
            let locked = "Authorization: Bearer locked\r\n";
            let unlocked = "Authorization: Bearer unlocked\r\n";
            let with_key = "Authorization: Bearer locked\r\nX-Internal-API-Key: key\r\n";
            let wrong_key = "Authorization: Bearer locked\r\nX-Internal-API-Key: wrong\r\n";
            (
                request_status(port, "GET", "/v1alpha/vm", locked),
                request_status(port, "POST", "/v1alpha/vm", locked),
                request_status(port, "DELETE", "/v1alpha/vm", locked),
                request_status(port, "POST", "/v1alpha/vm", with_key),
                request_status(port, "POST", "/v1alpha/vm", wrong_key),
                request_status(port, "POST", "/v1alpha/vm", unlocked),
                request_status(port, "POST", "/v1alpha/vm", "Authorization: Bearer x\r\n"),
            )
        })
        .await
        .unwrap();
        handle.stop(true).await;
        miko_handle.stop(true).await;

        // a locked project can still be read
        assert_eq!(results.0, 200);
        // all other requests of the user are rejected
        assert_eq!(results.1, 423);
        assert_eq!(results.2, 423);
        // requests of other components with the internal API-key are not blocked
        assert_eq!(results.3, 200);
        assert_eq!(results.4, 423);
        // a project, which is not locked, can be changed
        assert_eq!(results.5, 200);
        // an invalid token is still rejected first
        assert_eq!(results.6, 401);
    }
}
