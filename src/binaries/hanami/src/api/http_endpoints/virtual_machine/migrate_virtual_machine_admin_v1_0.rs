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
use apistos::actix::AcceptedJson;
use apistos::api_operation;
use uuid::Uuid;
use validator::Validate;

use crate::core::migration::session::Session;
use crate::core::migration::{Migration, MigrationGuard, spawn_migration};
use crate::database::address_table;
use crate::database::host_table::{self, HostResources};
use crate::database::meta_virtual_machine_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::migration_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums::DbError;

#[api_operation(
    tag = "virtual_machine",
    summary = "Migrate virtual_machine",
    description = r###"Move a virtual_machine to another sakura-host. This can only be done by an admin.

The migration is a cold migration, which runs in the background after the request was accepted:
the virtual_machine is shut down gracefully on its current host, its disk is transferred directly
to the target host and it is booted there again, if it was running before. It keeps its UUID, its
address, its proxy-port and its packet-filters. While it is migrated, it can not be started,
stopped, deleted or get new packet-filters.

The target host must have enough free resources for the virtual_machine. A virtual_machine on an
isolated host can only be moved to another isolated host, which is free or isolated for its
project. A virtual_machine on a shared host can only be moved to another shared host. If the
migration fails, the virtual_machine is started on its current host again."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn migrate_virtual_machine_admin(
    virtual_machine_uuid: Path<Uuid>,
    body: Json<VirtualMachineMigrateReq>,
    context: UserContext,
) -> Result<AcceptedJson<VirtualMachineMigrateResp>, ErrorResponse> {
    check_admin_context(&context)?;
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let virtual_machine_uuid = virtual_machine_uuid.into_inner();
    let virtual_machine_data =
        meta_virtual_machine_table::get_meta_virtual_machine(&virtual_machine_uuid, &context)
            .map_err(|e| {
                map_db_uuid_get_delete_error("virtual_machine", &virtual_machine_uuid, e)
            })?;

    let source_uuid = virtual_machine_data.sakura_host_uuid;
    let target_uuid = body.target_host_uuid;
    if source_uuid == target_uuid {
        return Err(ErrorResponse::BadRequest(format!(
            "Virtual_machine '{virtual_machine_uuid}' already runs on host '{target_uuid}'."
        )));
    }
    let source = host_table::get_host(&source_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("sakura-host", &source_uuid, e))?;
    let target = host_table::get_host(&target_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("sakura-host", &target_uuid, e))?;

    let address =
        address_table::get_address_of_virtual_machine(&virtual_machine_uuid).map_err(|e| {
            map_db_uuid_get_delete_error("address of virtual_machine", &virtual_machine_uuid, e)
        })?;
    // entries of a database, which was created before the host-address was stored with the
    // addresses, have no torii, whose routes could be moved
    if address.host_address.is_empty() {
        return Err(ErrorResponse::Conflict(format!(
            "Virtual_machine '{virtual_machine_uuid}' has no host-address and can not be migrated."
        )));
    }

    let guard = MigrationGuard::acquire(virtual_machine_uuid).ok_or_else(|| {
        ErrorResponse::Conflict(format!(
            "Virtual_machine '{virtual_machine_uuid}' is already migrated."
        ))
    })?;

    // fails before anything was changed, if the token of the admin can't be renewed during the
    // migration
    let session = Session::start(context).await?;

    // the virtual_machine stays in the kind of host, which it was placed on
    let resources = HostResources {
        number_of_cores: virtual_machine_data.number_of_cores,
        memory_size: virtual_machine_data.memory_size,
        disk_space: virtual_machine_data.disk_size,
        project_id: source
            .is_host_isolated
            .then(|| virtual_machine_data.project_id.clone()),
    };
    host_table::allocate_resources_on_host(&target_uuid, &resources).map_err(|e| match e {
        DbError::NotFound => ErrorResponse::Conflict(format!(
            "Host '{target_uuid}' has not enough free resources for virtual_machine \
             '{virtual_machine_uuid}' or doesn't match the isolation of its current host."
        )),
        e => map_db_uuid_get_delete_error("sakura-host", &target_uuid, e),
    })?;

    spawn_migration(
        Migration {
            virtual_machine_uuid,
            proxy_uuid: virtual_machine_data.proxy_uuid,
            source,
            target,
            address,
            resources,
        },
        session,
        guard,
    );

    Ok(AcceptedJson(VirtualMachineMigrateResp {
        virtual_machine_uuid,
        source_host_uuid: source_uuid,
        target_host_uuid: target_uuid,
    }))
}
