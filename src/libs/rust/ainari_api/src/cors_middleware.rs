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

use actix_web::HttpResponse;
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::{
    Error,
    body::{EitherBody, MessageBody},
    dev::{ServiceRequest, ServiceResponse},
    http::Method,
    middleware::Next,
};

/**
 * Cross-Origin Resource Sharing (CORS) middleware implementation.
 *
 * This middleware handles CORS preflight requests (OPTIONS method) and adds the necessary
 * CORS headers to all responses. It's essential for enabling cross-origin requests
 * from web browsers to the server, which is required for the dashboard functionality.
 */
pub async fn cors_middleware<B>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<EitherBody<B>>, Error>
where
    B: MessageBody + 'static,
{
    // Check if the request is a CORS preflight request (OPTIONS method)
    if req.method() == Method::OPTIONS {
        // Create a response for preflight requests with all necessary CORS headers
        let res = HttpResponse::Ok()
            .insert_header(("access-control-allow-origin", "*"))
            .insert_header((
                "access-control-allow-methods",
                "GET, POST, PUT, DELETE, OPTIONS",
            ))
            .insert_header((
                "access-control-allow-headers",
                "Content-Type, Authorization, X-Internal-API-Key",
            ))
            .finish();

        // Return the preflight response
        return Ok(req.into_response(res.map_into_right_body()));
    }

    // Process the request with the next middleware in the chain
    let mut res = next.call(req).await?;

    // Add CORS headers to the response
    // This allows any origin to access the resource (adjust in production for security)
    res.headers_mut().insert(
        HeaderName::from_static("access-control-allow-origin"),
        HeaderValue::from_static("*"),
    );

    // Return the modified response
    Ok(res.map_into_left_body())
}
