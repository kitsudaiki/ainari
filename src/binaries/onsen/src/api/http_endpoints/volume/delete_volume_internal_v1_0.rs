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

use actix_web::web::{Data, Path};
use apistos::actix::NoContent;
use apistos::api_operation;
use uuid::Uuid;

use crate::api::http_endpoints::*;
use crate::core::backend::{Backend, VolumeBackend};

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "volume",
    summary = "Delete volume (internal)",
    description = r###"Delete a volume from the storage-backend. Deleting a not existing volume is
no error. A copy, which runs into the volume, fails.

This is an internal call, which is protected by the internal api-key and the token of a user."###,
    error_code = 400,
    error_code = 401,
    error_code = 409,
    error_code = 500
)]
pub async fn delete_volume_internal(
    backend_id: Path<Uuid>,
    backend: Data<Backend>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    backend
        .delete_volume(&backend_id)
        .await
        .map_err(|e| map_storage_error(e, &context))?;

    Ok(NoContent)
}
