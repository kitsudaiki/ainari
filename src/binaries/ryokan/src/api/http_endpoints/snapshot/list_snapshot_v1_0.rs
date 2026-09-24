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
use apistos::api_operation;

use crate::database::snapshot_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::snapshot_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "snapshot",
    summary = "List snapshots",
    description = r###"List basic information of all snapshots from the database."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_snapshot(context: UserContext) -> Result<Json<SnapshotListResp>, ErrorResponse> {
    let snapshots =
        snapshot_table::list_snapshots(&context).map_err(|e| map_db_list_error("snapshots", e))?;

    let mut resp = SnapshotListResp {
        snapshots: Vec::new(),
    };

    for snapshot in snapshots {
        let uuid = convert_uuid(&snapshot.uuid)?;
        let obj = SnapshotBasicResp {
            uuid,
            name: snapshot.name.clone(),
        };

        resp.snapshots.push(obj);
    }

    Ok(Json(resp))
}
