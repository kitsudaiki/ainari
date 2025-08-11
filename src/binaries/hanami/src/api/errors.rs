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

use actix_web::ResponseError;
use actix_web::http::StatusCode;
use apistos::ApiErrorComponent;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

#[derive(Serialize, Deserialize, Clone, ApiErrorComponent)]
#[openapi_error(
    status(code = 400, description = "Bad request"),
    status(code = 401, description = "Unauthorized"),
    status(code = 403, description = "Forbidden"),
    status(code = 404, description = "Requested object not found"),
    status(code = 405, description = "Method not allowed"),
    status(code = 406, description = "Not acceptable"),
    status(code = 407, description = "Proxy authentication required"),
    status(code = 408, description = "Request timeout"),
    status(code = 409, description = "Conflict with existing resources"),
    status(code = 410, description = "Gone"),
    status(code = 412, description = "Precondition failed"),
    status(code = 413, description = "Payload too large"),
    status(code = 500, description = "Internal error")
)]
pub enum ErrorResponse {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    MethodNotAllowed(String),
    NotAcceptable(String),
    ProxyAuthenticationRequired(String),
    RequestTimeout(String),
    Conflict(String),
    Gone(String),
    PreconditionFailed(String),
    PayloadTooLarge(String),
    InternalError(String),
}

impl Debug for ErrorResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorResponse::BadRequest(str)
            | ErrorResponse::Unauthorized(str)
            | ErrorResponse::Forbidden(str)
            | ErrorResponse::NotFound(str)
            | ErrorResponse::MethodNotAllowed(str)
            | ErrorResponse::NotAcceptable(str)
            | ErrorResponse::ProxyAuthenticationRequired(str)
            | ErrorResponse::RequestTimeout(str)
            | ErrorResponse::Conflict(str)
            | ErrorResponse::Gone(str)
            | ErrorResponse::PreconditionFailed(str)
            | ErrorResponse::PayloadTooLarge(str)
            | ErrorResponse::InternalError(str) => {
                write!(f, "{str}")
            }
        }
    }
}

impl Display for ErrorResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorResponse::BadRequest(str)
            | ErrorResponse::Unauthorized(str)
            | ErrorResponse::Forbidden(str)
            | ErrorResponse::NotFound(str)
            | ErrorResponse::MethodNotAllowed(str)
            | ErrorResponse::NotAcceptable(str)
            | ErrorResponse::ProxyAuthenticationRequired(str)
            | ErrorResponse::RequestTimeout(str)
            | ErrorResponse::Conflict(str)
            | ErrorResponse::Gone(str)
            | ErrorResponse::PreconditionFailed(str)
            | ErrorResponse::PayloadTooLarge(str)
            | ErrorResponse::InternalError(str) => {
                write!(f, "{str}")
            }
        }
    }
}

impl ResponseError for ErrorResponse {
    fn status_code(&self) -> StatusCode {
        match self {
            ErrorResponse::BadRequest(_) => StatusCode::BAD_REQUEST,
            ErrorResponse::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ErrorResponse::Forbidden(_) => StatusCode::FORBIDDEN,
            ErrorResponse::NotFound(_) => StatusCode::NOT_FOUND,
            ErrorResponse::MethodNotAllowed(_) => StatusCode::METHOD_NOT_ALLOWED,
            ErrorResponse::NotAcceptable(_) => StatusCode::NOT_ACCEPTABLE,
            ErrorResponse::ProxyAuthenticationRequired(_) => {
                StatusCode::PROXY_AUTHENTICATION_REQUIRED
            }
            ErrorResponse::RequestTimeout(_) => StatusCode::REQUEST_TIMEOUT,
            ErrorResponse::Conflict(_) => StatusCode::CONFLICT,
            ErrorResponse::Gone(_) => StatusCode::GONE,
            ErrorResponse::PreconditionFailed(_) => StatusCode::PRECONDITION_FAILED,
            ErrorResponse::PayloadTooLarge(_) => StatusCode::PAYLOAD_TOO_LARGE,
            ErrorResponse::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
