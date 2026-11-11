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

use crate::api::http_endpoints::*;
use crate::core::backend::{Backend, VolumeBackend};

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::storage_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "volume",
    summary = "Get volume-connection (internal)",
    description = r###"Get the information, how a sakura-host connects to a volume. Its `type`
depends on the storage-backend, like `nfs` for the local backend. A volume, into which a copy
still runs, is a conflict.

This is an internal call, which is protected by the internal api-key and the token of a user."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn get_volume_connection_internal(
    backend_id: Path<Uuid>,
    backend: Data<Backend>,
    context: UserContext,
) -> Result<Json<VolumeConnection>, ErrorResponse> {
    let connection = backend
        .volume_connection(&backend_id)
        .await
        .map_err(|e| map_storage_error(e, &context))?;

    Ok(Json(connection))
}
