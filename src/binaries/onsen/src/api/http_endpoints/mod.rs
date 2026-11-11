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

pub mod backend;
pub mod directory;
pub mod file;
pub mod volume;

use validator::Validate;

use ainari_api::error_mapping::map_ainari_error_to_api_response;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::storage_structs::StoragePathQuery;
use ainari_api_structs::user_context::UserContext;
use ainari_common::error::AinariError;

/// Validates the query with the path of a file or directory.
pub(crate) fn validate_query(query: &StoragePathQuery) -> Result<(), ErrorResponse> {
    query
        .validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))
}

/// Maps the error of a file-operation to the response. The internal errors are only reported for
/// the user of the request, but not returned to the caller.
pub(crate) fn map_storage_error(e: AinariError, context: &UserContext) -> ErrorResponse {
    map_ainari_error_to_api_response(e, Some(context))
}
