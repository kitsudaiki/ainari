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

use actix_web::web::{Data, Json};
use apistos::actix::CreatedJson;
use apistos::api_operation;
use validator::Validate;

use crate::api::http_endpoints::*;
use crate::core::backend::{Backend, VolumeBackend};

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::storage_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "volume",
    summary = "Create volume (internal)",
    description = r###"Create an empty volume with the given size in bytes within the
storage-backend. The size must be a multiple of 512. The volume is addressed by its `backend_id`.

Creating an already existing volume with the same size is no error, so a failed request can be
repeated. A volume with another size, or a volume, into which a copy runs, is a conflict. A
backend without support of volumes rejects the request with a conflict as well.

This is an internal call, which is protected by the internal api-key and the token of a user."###,
    error_code = 400,
    error_code = 401,
    error_code = 409,
    error_code = 500
)]
pub async fn create_volume_internal(
    body: Json<VolumeCreateInternalReq>,
    backend: Data<Backend>,
    context: UserContext,
) -> Result<CreatedJson<VolumeInfoResp>, ErrorResponse> {
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let info = backend
        .create_volume(&body.backend_id, body.size)
        .await
        .map_err(|e| map_storage_error(e, &context))?;

    Ok(CreatedJson(info))
}
