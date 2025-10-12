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

use actix_web::HttpResponse;
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::{
    Error,
    body::{EitherBody, MessageBody},
    dev::{ServiceRequest, ServiceResponse},
    http::Method,
    middleware::Next,
};

//Cross-Origin Resource Sharing (CORS) middleware, which is necessary to make web-browser happy to make the dasboard working
pub async fn cors_middleware<B>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<EitherBody<B>>, Error>
where
    B: MessageBody + 'static,
{
    if req.method() == Method::OPTIONS {
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

        return Ok(req.into_response(res.map_into_right_body()));
    }

    let mut res = next.call(req).await?;

    // Correct way to insert a header
    res.headers_mut().insert(
        HeaderName::from_static("access-control-allow-origin"),
        HeaderValue::from_static("*"),
    );

    Ok(res.map_into_left_body())
}
