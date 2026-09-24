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

use actix_web::web::Json;
use apistos::actix::CreatedJson;
use apistos::api_operation;
use validator::Validate;

use crate::config;
use crate::database::snapshot_table;
use crate::onsen_functions::select_onsen;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::snapshot_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::quota::get_quota;

#[api_operation(
    tag = "snapshot",
    summary = "Initialize new snapshot",
    description = r###"Initialize a new snapshot.

Prepares the database-entry and the onsen, before the payload is uploaded.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn init_snapshot(
    body: Json<SnapshotCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<SnapshotInternalResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let name = &body.name;
    let snapshot_uuid = &body.uuid.clone();
    let file_path_str: String = format!("snapshots/{}", snapshot_uuid);

    check_snapshot_quota(&context).await?;

    let selected_onsen = select_onsen(&context)?;

    let (secret_uuid, _) = super::super::generate_new_key(snapshot_uuid, &context).await?;

    snapshot_table::add_new_snapshot(
        snapshot_uuid,
        name,
        &selected_onsen.address,
        &file_path_str,
        &secret_uuid,
        &context,
    )
    .map_err(|e| {
        log::error!("Failed to add snapshot to database: {e}");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    let snapshot = snapshot_table::get_snapshot(snapshot_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("snapshot", snapshot_uuid, e))?;

    let secret_uuid = convert_uuid(&snapshot.secret_uuid)?;
    let resp = SnapshotInternalResp {
        uuid: *snapshot_uuid,
        name: snapshot.name,
        onsen_address: snapshot.onsen_address,
        file_path: snapshot.file_path,
        secret_uuid,
        created_by: snapshot.created_by,
        created_at: snapshot.created_at,
        updated_by: snapshot.updated_by,
        updated_at: snapshot.updated_at,
    };

    Ok(CreatedJson(resp))
}

/// Checks if the user is still allowed to create another snapshot.
///
/// The current number of snapshots of the user is counted in the database and compared against
/// the maximum, which is defined by the quota of the user in the miko.
///
/// # Arguments
///
/// * `context` - User-context of the request
///
/// # Returns
///
/// * `Ok(())` - The user is still below the limit.
/// * `Err(ErrorResponse::Conflict)` - The quota of the user is already exhausted.
/// * `Err(ErrorResponse)` - The snapshots could not be counted or the quota not be read.
async fn check_snapshot_quota(context: &UserContext) -> Result<(), ErrorResponse> {
    // get number of snapshots of the user
    let current_number_of_snapshots = snapshot_table::count_snapshots(context).map_err(|e| {
        log::error!("Failed to count snapshots in database.: {e}");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    // check the maximum number of snapshots defined in miko
    let miko_endpoint = &config::CONFIG.miko;
    let quota = get_quota(
        miko_endpoint,
        &context.token,
        &context.user_id,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    // check if quota is already exceeded
    if current_number_of_snapshots as i64 >= quota.max_snapshot as i64 {
        return Err(ErrorResponse::Conflict(
            "Maximum number of snapshots exceeded.".to_string(),
        ));
    }

    Ok(())
}
