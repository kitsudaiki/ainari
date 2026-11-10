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

use actix_web::dev::Payload;
use actix_web::web::{Data, Query};
use actix_web::{FromRequest, HttpRequest};
use apistos::actix::NoContent;
use apistos::reference_or::ReferenceOr;
use apistos::{ApiComponent, api_operation};
use schemars::JsonSchema;
use schemars::r#gen::SchemaGenerator;
use schemars::schema::{InstanceType, Schema, SchemaObject};
use std::future::{Ready, ready};

use crate::api::http_endpoints::*;
use crate::core::backend::{Backend, FileBackend};

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::storage_structs::StoragePathQuery;
use ainari_api_structs::user_context::UserContext;

// It is only used for the openapi-documentation, because the file is streamed directly to disk,
// instead of collecting it in memory.
/// Content of the uploaded file
#[derive(JsonSchema, ApiComponent)]
#[allow(dead_code)]
struct FileContent(#[schemars(schema_with = "binary_file_schema")] String);

/// Returns the openapi-schema of an uploaded file
fn binary_file_schema(_: &mut SchemaGenerator) -> Schema {
    SchemaObject {
        instance_type: Some(InstanceType::String.into()),
        format: Some("binary".to_string()),
        ..Default::default()
    }
    .into()
}

/// Raw body of the upload, which documents the content as binary file in the openapi-
/// specification.
pub struct FilePayload(Payload);

impl FromRequest for FilePayload {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(_: &HttpRequest, payload: &mut Payload) -> Self::Future {
        ready(Ok(Self(payload.take())))
    }
}

impl ApiComponent for FilePayload {
    fn content_type() -> String {
        "application/octet-stream".to_string()
    }

    fn child_schemas() -> Vec<(String, ReferenceOr<apistos::Schema>)> {
        FileContent::child_schemas()
    }

    fn schema() -> Option<(String, ReferenceOr<apistos::Schema>)> {
        FileContent::schema()
    }
}

#[api_operation(
    tag = "file",
    summary = "Upload file (internal)",
    description = r###"Store the body of the request as file under the query-parameter `path`,
which is relative to the storage-location. An existing file is overwritten and missing directories
are created.

The body is streamed directly into the file, so it can have any size. If the upload breaks off
before the announced `Content-Length` was received, the incomplete file is deleted again.

This is an internal call, which is protected by the internal api-key and the token of a user."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn upload_file_internal(
    query: Query<StoragePathQuery>,
    payload: FilePayload,
    backend: Data<Backend>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    validate_query(&query)?;

    let size = backend
        .write_file(&query.path, payload.0)
        .await
        .map_err(|e| map_storage_error(e, &context))?;
    log::info!("Stored file '{}' with {size} bytes", query.path);

    Ok(NoContent)
}
