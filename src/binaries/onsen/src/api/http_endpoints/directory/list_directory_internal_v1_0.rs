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

use actix_web::web::{Data, Json, Query};
use apistos::api_operation;

use crate::api::http_endpoints::*;
use crate::core::file_operations;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::storage_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "directory",
    summary = "List directory (internal)",
    description = r###"List the direct entries of the directory under the query-parameter `path`,
which is relative to the storage-location, sorted by name. An empty path lists the
storage-location itself. Symbolic links are listed as files with the size of the link.

This is an internal call, which is protected by the internal api-key and the token of a user."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn list_directory_internal(
    query: Query<StoragePathQuery>,
    storage: Data<StorageLocation>,
    context: UserContext,
) -> Result<Json<StorageDirectoryResp>, ErrorResponse> {
    validate_query(&query)?;

    let entries = file_operations::list_directory(&storage.0, &query.path)
        .await
        .map_err(|e| map_storage_error(e, &context))?;

    Ok(Json(StorageDirectoryResp { entries }))
}
