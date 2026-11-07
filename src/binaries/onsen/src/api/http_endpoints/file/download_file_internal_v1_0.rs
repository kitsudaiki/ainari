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
use actix_web::web::{Data, Query};
use apistos::api_operation;

use crate::api::http_endpoints::*;
use crate::core::file_operations;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::storage_structs::StoragePathQuery;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "file",
    summary = "Download file (internal)",
    description = r###"Stream the content of the file under the query-parameter `path`, which is
relative to the storage-location.

The size of the file is sent in the header `Content-Length`, so the caller detects a download,
which broke off. Symbolic links are not followed.

This is an internal call, which is protected by the internal api-key and the token of a user."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn download_file_internal(
    query: Query<StoragePathQuery>,
    storage: Data<StorageLocation>,
    context: UserContext,
) -> Result<HttpResponse, ErrorResponse> {
    validate_query(&query)?;

    let (size, content) = file_operations::read_file(&storage.0, &query.path)
        .await
        .map_err(|e| map_storage_error(e, &context))?;
    log::info!("Send file '{}' with {size} bytes", query.path);

    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .no_chunking(size)
        .streaming(content))
}
