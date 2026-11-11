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

use actix_web::web::{Data, Json, Path};
use apistos::api_operation;
use uuid::Uuid;
use validator::Validate;

use crate::api::http_endpoints::*;
use crate::core::backend::{Backend, VolumeBackend};

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::storage_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "volume",
    summary = "Resize volume (internal)",
    description = r###"Grow a volume within the storage-backend to the given size in bytes, which
must be a multiple of 512. A volume can only grow, a smaller size is rejected.

This is an internal call, which is protected by the internal api-key and the token of a user."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn resize_volume_internal(
    backend_id: Path<Uuid>,
    body: Json<VolumeResizeInternalReq>,
    backend: Data<Backend>,
    context: UserContext,
) -> Result<Json<VolumeInfoResp>, ErrorResponse> {
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let info = backend
        .resize_volume(&backend_id, body.size)
        .await
        .map_err(|e| map_storage_error(e, &context))?;

    Ok(Json(info))
}
