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
    tag = "file",
    summary = "Get file-info (internal)",
    description = r###"Get the size and the hex-encoded sha256-checksum of the file under the
query-parameter `path`, which is relative to the storage-location. The whole file is read for the
checksum, which takes its time for large files. Symbolic links are not followed.

This is an internal call, which is protected by the internal api-key and the token of a user."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_file_info_internal(
    query: Query<StoragePathQuery>,
    storage: Data<StorageLocation>,
    context: UserContext,
) -> Result<Json<StorageFileInfoResp>, ErrorResponse> {
    validate_query(&query)?;

    let info = file_operations::get_file_info(&storage.0, &query.path)
        .await
        .map_err(|e| map_storage_error(e, &context))?;

    Ok(Json(info))
}
