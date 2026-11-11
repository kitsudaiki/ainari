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
use actix_web::web::{Data, Json, Path};
use apistos::api_operation;
use uuid::Uuid;

use crate::api::http_endpoints::*;
use crate::core::backend::{Backend, VolumeBackend};

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::storage_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "volume",
    summary = "Copy volume (internal)",
    description = r###"Start the copy of a whole volume into the new volume `target_backend_id`
within the same storage-backend. The copy runs in the background, so the request returns
`202 Accepted` directly. The state of the copy is reported by the get-request of the target.

The content of a volume is encrypted, so the copy needs no key. An already existing target is a
conflict.

This is an internal call, which is protected by the internal api-key and the token of a user."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn copy_volume_internal(
    backend_id: Path<Uuid>,
    body: Json<VolumeCopyInternalReq>,
    backend: Data<Backend>,
    context: UserContext,
) -> Result<HttpResponse, ErrorResponse> {
    backend
        .copy_volume(&backend_id, &body.target_backend_id)
        .await
        .map_err(|e| map_storage_error(e, &context))?;

    Ok(HttpResponse::Accepted().finish())
}
