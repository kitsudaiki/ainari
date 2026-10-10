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
use chrono::Utc;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use validator::Validate;

use crate::config;
use crate::core::processing::tasks::{
    CloudHypervisorVirtualMachineMigrationImportInfo, TaskVariant,
};
use crate::database::virtual_machine_table;
use crate::database::virtual_machine_table::{VirtualMachineEntry, VirtualMachineState};

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::migration_structs::*;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::virtual_machine_migration::get_migration_description;

#[api_operation(
    tag = "migration",
    summary = "Import virtual_machine",
    description = r###"Take over a virtual_machine, which was prepared by another host.

The description of the virtual_machine is read from the source host and the virtual_machine is
created on this host with the same identity, owner and project, in the state `MIGRATING`. A task
pulls its files from the source host afterwards and boots it with `boot`, because it was running
before the migration. The TAP-device of the virtual_machine has to exist on this host already."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn import_virtual_machine_internal(
    body: Json<MigrationImportReq>,
    context: UserContext,
) -> Result<CreatedJson<TaskResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;
    if context.is_read_only() {
        return Err(permission_denied_response());
    }

    let virtual_machine_uuid = body.virtual_machine_uuid;

    // fails, if the virtual_machine is not prepared on the source host
    let description = get_migration_description(
        &body.source_address,
        &context.token,
        &config::INTERNAL_API_KEY,
        &virtual_machine_uuid,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    virtual_machine_table::add_virtual_machine(imported_entry(description, &context)).map_err(
        |e| match e {
            DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
                ErrorResponse::Conflict(format!(
                    "Virtual_machine '{virtual_machine_uuid}' already exists on this host."
                ))
            }
            e => map_db_write_error(
                &format!("add imported virtual_machine '{virtual_machine_uuid}' to database"),
                e,
            ),
        },
    )?;

    let task_description = format!(
        "Import virtual machine with UUID {virtual_machine_uuid} from {}",
        body.source_address
    );
    let info = CloudHypervisorVirtualMachineMigrationImportInfo {
        vm_uuid: virtual_machine_uuid,
        source_address: body.source_address.clone(),
        boot: body.boot,
        description: task_description.clone(),
        context: context.clone(),
    };

    let resp = super::add_migration_task(
        &virtual_machine_uuid,
        TaskType::MigrationImport,
        task_description,
        TaskVariant::CloudHypervisorVirtualMachineMigrationImport(info),
        &context,
    )?;

    Ok(CreatedJson(resp))
}

/// Builds the database-entry of an imported virtual_machine from its description
///
/// The virtual_machine keeps its identity, owner, project and creation. It has no files yet,
/// which are pulled by the import-task, so it starts in the state `MIGRATING`.
///
/// # Arguments
/// * `description` - Description of the virtual_machine from the source host
/// * `context` - User context of the migration
///
/// # Returns
/// The new database-entry
fn imported_entry(
    description: MigrationDescriptionResp,
    context: &UserContext,
) -> VirtualMachineEntry {
    VirtualMachineEntry {
        uuid: description.uuid,
        name: description.name,
        vm_state: VirtualMachineState::Migrating.to_string(),
        number_of_cores: description.number_of_cores,
        memory_size: description.memory_size,
        disk_size: description.disk_size,
        image_uuid: description.image_uuid,
        public_key_uuid: description.public_key_uuid,
        network_uuid: description.network_uuid,
        internal_ip: description.internal_ip,
        root_disk_path: None,
        seed_path: String::new(),
        tap_name: description.tap_name,
        mac_address: description.mac_address,
        owner_id: description.owner_id,
        project_id: description.project_id,
        status: "ACTIVE".to_string(),
        created_at: description.created_at,
        created_by: description.created_by,
        updated_at: Utc::now(),
        updated_by: context.user_id.clone(),
        deleted_at: None,
        deleted_by: None,
    }
}
