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

//! Authorization of the requests against the izakaya.
//!
//! Only services talk to the izakaya: the gateways from their background-loop, which has no user,
//! and hanami. So the requests are authorized by the internal API-key alone, instead of the token
//! of a user, and the internal endpoints are only reachable over the internal port. The decision,
//! who may be member of which group, doesn't rely on this: it is signed by hanami and checked by
//! the gateways themselves.

use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    http::Method,
    middleware::Next,
    web,
};

use ainari_api::auth_middleware::{
    ApiValidationConfig, check_internal_endpoint_access, check_internal_request,
};

/// Checks the internal API-key of every request against an internal endpoint.
///
/// # Arguments
/// * `req` - The incoming request
/// * `next` - The rest of the middleware-chain
///
/// # Returns
/// The response of the endpoint, or an error, if the request is not authorized
pub async fn izakaya_auth_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let api_validation_config = req
        .app_data::<web::Data<ApiValidationConfig>>()
        .expect("Api-validation-config missing!");

    check_internal_endpoint_access(&req, api_validation_config)?;
    if *req.method() != Method::OPTIONS {
        check_internal_request(&req, api_validation_config)?;
    }

    next.call(req).await
}
