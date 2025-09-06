use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::{
    Error,
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    middleware::Next,
};

pub async fn cors_middleware<B>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<impl MessageBody>, Error>
where
    B: MessageBody + 'static,
{
    let mut res = next.call(req).await?;

    // Correct way to insert a header
    res.headers_mut().insert(
        HeaderName::from_static("access-control-allow-origin"),
        HeaderValue::from_static("*"),
    );

    Ok(res)
}
