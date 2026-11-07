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

use actix_web::web::{Data, Query};
use apistos::actix::NoContent;
use apistos::api_operation;

use crate::api::http_endpoints::*;
use crate::core::file_operations;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::storage_structs::StoragePathQuery;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "directory",
    summary = "Delete directory (internal)",
    description = r###"Delete the directory under the query-parameter `path`, which is relative to
the storage-location, with all of its content. Deleting a not existing directory is no error.
Symbolic links within the directory are removed, but not followed. The storage-location itself
can not be deleted.

This is an internal call, which is protected by the internal api-key and the token of a user."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn delete_directory_internal(
    query: Query<StoragePathQuery>,
    storage: Data<StorageLocation>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    validate_query(&query)?;

    file_operations::delete_directory(&storage.0, &query.path)
        .await
        .map_err(|e| map_storage_error(e, &context))?;

    Ok(NoContent)
}
