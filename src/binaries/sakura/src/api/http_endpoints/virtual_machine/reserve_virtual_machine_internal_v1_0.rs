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
use uuid::Uuid;
use validator::Validate;

use crate::database::virtual_machine_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_api_structs::virtual_machine_structs::*;

#[api_operation(
    tag = "virtual_machine",
    summary = "Reserve new virtual_machine",
    description = r###"Reserve a new virtual_machine on this host,

so its resources are blocked, before the create-task is queued."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn reserve_virtual_machine_internal(
    body: Json<VirtualMachineInternalCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<VirtualMachineResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    // a fixed UUID, for example of a restored virtual_machine, can only replace a deleted one
    let virtual_machine_uuid = match body.uuid {
        Some(uuid) => {
            match virtual_machine_table::find_virtual_machine_status(&uuid)
                .map_err(|e| map_db_list_error("virtual_machines", e, Some(&context)))?
            {
                Some(status) if status != "DELETED" => {
                    return Err(ErrorResponse::Conflict(format!(
                        "Virtual_machine with UUID '{uuid}' already exists."
                    )));
                }
                Some(_) => {
                    virtual_machine_table::remove_deleted_virtual_machine(&uuid).map_err(|e| {
                        map_db_write_error(
                            &format!("remove deleted virtual_machine '{uuid}'"),
                            e,
                            Some(&context),
                        )
                    })?;
                }
                None => {}
            }
            uuid
        }
        None => Uuid::new_v4(),
    };

    // these values coming from the task, so they are temporary filled with nil-values
    let empty_image_uuid = Uuid::nil();
    let empty_public_key_uuid = Uuid::nil();

    let new_virtual_machine = virtual_machine_table::NewVirtualMachine {
        uuid: virtual_machine_uuid,
        name: body.name.clone(),
        number_of_cores: body.number_of_cores,
        memory_size: body.memory_size,
        disk_size: body.disk_size,
        image_uuid: empty_image_uuid,
        public_key_uuid: empty_public_key_uuid,
        network_uuid: body.network_uuid,
        internal_ip: body.internal_ip,
        root_disk_path: None,
        seed_path: "".to_owned(),
        tap_name: body.tap_name.clone(),
        mac_address: body.mac_address.clone(),
    };

    virtual_machine_table::add_new_virtual_machine(new_virtual_machine, &context).map_err(|e| {
        map_db_write_error(
            &format!("add virtual-machine with UUID '{virtual_machine_uuid}' to database"),
            e,
            Some(&context),
        )
    })?;

    let virtual_machine_data = virtual_machine_table::get_virtual_machine(
        &virtual_machine_uuid,
        &context,
    )
    .map_err(|e| {
        map_db_uuid_get_delete_error("virtual_machine", &virtual_machine_uuid, e, Some(&context))
    })?;

    let resp = VirtualMachineResp {
        uuid: virtual_machine_uuid,
        name: virtual_machine_data.name,
        vm_state: virtual_machine_data.vm_state,
        number_of_cores: virtual_machine_data.number_of_cores,
        memory_size: virtual_machine_data.memory_size,
        disk_size: virtual_machine_data.disk_size,
        image_uuid: virtual_machine_data.image_uuid,
        network_uuid: virtual_machine_data.network_uuid,
        internal_ip: virtual_machine_data.internal_ip,
        torii_address: String::new(),
        created_by: virtual_machine_data.created_by,
        created_at: virtual_machine_data.created_at,
        updated_by: virtual_machine_data.updated_by,
        updated_at: virtual_machine_data.updated_at,
    };

    Ok(CreatedJson(resp))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::http_endpoints::virtual_machine::export_virtual_machine_internal_v1_0::export_virtual_machine_internal;
    use crate::database::db_handle;
    use actix_web::web::Path;
    use ainari_common::enums::ProjectRole;
    use diesel::prelude::*;
    use serial_test::serial;
    use std::net::Ipv4Addr;

    fn context() -> UserContext {
        UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: "test-project".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        }
    }

    fn request(uuid: Option<Uuid>) -> VirtualMachineInternalCreateReq {
        VirtualMachineInternalCreateReq {
            name: "restored".to_string(),
            network_uuid: Uuid::new_v4(),
            number_of_cores: 2,
            memory_size: 1 << 30,
            disk_size: 10,
            internal_ip: Ipv4Addr::new(10, 0, 0, 5),
            tap_name: "tap-test".to_string(),
            mac_address: "02:00:00:00:00:05".to_string(),
            uuid,
        }
    }

    fn hard_delete(vm: &Uuid) {
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        use crate::database::virtual_machine_table::virtual_machines::dsl::*;
        let _ =
            diesel::delete(virtual_machines.filter(uuid.eq(vm.to_string()))).execute(&mut *conn);
    }

    #[actix_rt::test]
    #[serial]
    async fn test_reserve_with_fixed_uuid_and_export() {
        let vm = Uuid::new_v4();
        let resp = reserve_virtual_machine_internal(Json(request(Some(vm))), context())
            .await
            .unwrap()
            .0;
        assert_eq!(resp.uuid, vm);

        // the config is exported with the original values
        let exported = export_virtual_machine_internal(Path::from(vm), context())
            .await
            .unwrap()
            .into_inner();
        assert_eq!(exported.uuid, vm);
        assert_eq!(exported.number_of_cores, 2);
        assert_eq!(exported.memory_size, 1 << 30);
        assert_eq!(exported.internal_ip, Ipv4Addr::new(10, 0, 0, 5));
        assert_eq!(exported.mac_address, "02:00:00:00:00:05");

        // an active virtual_machine blocks its UUID
        assert!(matches!(
            reserve_virtual_machine_internal(Json(request(Some(vm))), context()).await,
            Err(ErrorResponse::Conflict(_))
        ));

        // the UUID of a deleted virtual_machine can be used again
        assert!(virtual_machine_table::delete_virtual_machine(&vm, &context()).is_ok());
        let resp = reserve_virtual_machine_internal(Json(request(Some(vm))), context())
            .await
            .unwrap()
            .0;
        assert_eq!(resp.uuid, vm);

        hard_delete(&vm);
    }
}
