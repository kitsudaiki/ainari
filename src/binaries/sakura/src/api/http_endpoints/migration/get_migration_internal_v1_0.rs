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

use actix_web::web::{Json, Path};
use apistos::api_operation;
use uuid::Uuid;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::migration_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "migration",
    summary = "Get migration",
    description = r###"Get the description of a virtual_machine, which is exported on this host for its migration.

It contains everything the target host needs to create the virtual_machine with the same
identity, including its owner and its project."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn get_migration_internal(
    virtual_machine_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<MigrationDescriptionResp>, ErrorResponse> {
    let virtual_machine_data =
        super::get_exported_virtual_machine(&virtual_machine_uuid, &context)?;

    Ok(Json(MigrationDescriptionResp {
        uuid: virtual_machine_data.uuid,
        name: virtual_machine_data.name,
        number_of_cores: virtual_machine_data.number_of_cores,
        memory_size: virtual_machine_data.memory_size,
        disk_size: virtual_machine_data.disk_size,
        image_uuid: virtual_machine_data.image_uuid,
        public_key_uuid: virtual_machine_data.public_key_uuid,
        network_uuid: virtual_machine_data.network_uuid,
        internal_ip: virtual_machine_data.internal_ip,
        tap_name: virtual_machine_data.tap_name,
        mac_address: virtual_machine_data.mac_address,
        owner_id: virtual_machine_data.owner_id,
        project_id: virtual_machine_data.project_id,
        created_at: virtual_machine_data.created_at,
        created_by: virtual_machine_data.created_by,
    }))
}
