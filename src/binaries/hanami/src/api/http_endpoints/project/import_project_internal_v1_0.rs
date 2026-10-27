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
use actix_web::web::Path;
use apistos::api_operation;
use chrono::Utc;
use uuid::Uuid;
use validator::Validate;

use crate::config;
use crate::core::floating_ip::attach_floating_ip;
use crate::core::network_filter::{
    NETWORK_FILTER_LOCK, resolve_filter_target, store_network_filter,
};
use crate::database::address_table;
use crate::database::floating_ip_table::{self, FloatingIpReserveError};
use crate::database::meta_virtual_machine_table;
use crate::database::network_filter_table;
use crate::database::network_table::{self, NetworkEntry};

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::floating_ip_structs::HanamiFloatingIpExport;
use ainari_api_structs::network_filter_structs::HanamiNetworkFilterExport;
use ainari_api_structs::network_structs::*;
use ainari_api_structs::project_structs::ImportedResource;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::network_filter as network_filter_clients;

#[api_operation(
    tag = "project",
    summary = "Import project",
    description = r###"Import the networks, packet filters and floating ip-addresses of a project from
a backup.

The virtual_machines are not imported here, but reserved like new ones over
`POST /v1alpha/virtual_machine/internal`. So the networks are imported first, the packet filters
and floating ip-addresses with a second request, after the virtual_machines were reserved. All
sections of the request are optional.

- Networks keep their UUIDs, unless the `id_map` gives them a new one. A UUID, which is used by
  another project, is rejected with `409`. An existing network of the project is rejected with
  `409`, unless `force` is set, which overwrites it. Its subnet and encryption can only be
  changed, while no virtual_machine uses the network. A deleted network is restored.
- Packet filters are applied by the torii of the host of their virtual_machine. An existing
  filter of the same direction is rejected with `409`, unless `force` is set, which replaces it.
- Floating ip-addresses are allocated again from the pool and get new UUIDs. The address of the
  backup is only used, if it is still free. A virtual_machine, which already has a floating
  ip-address, keeps it.

The virtual_machines of the packet filters and floating ip-addresses, after applying the
`id_map`, must exist in the project. All conflicts are checked, before anything is changed. The
quota of the project is not checked here. Users, which are not admin, can only import into their
own project.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 403,
    error_code = 409,
    error_code = 500
)]
pub async fn import_project_internal(
    project_id: Path<String>,
    body: Json<HanamiProjectImportReq>,
    context: UserContext,
) -> Result<Json<HanamiProjectImportResp>, ErrorResponse> {
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;
    check_project_access(&project_id, &context)?;
    if context.is_read_only() {
        return Err(permission_denied_response());
    }
    // the new entries belong to the project of the import, also if an admin of another project
    // imports them
    let context = UserContext {
        project_id: project_id.clone(),
        ..context
    };
    let new_uuid = |uuid: &Uuid| *body.id_map.get(uuid).unwrap_or(uuid);

    // check all conflicts, before anything is changed
    let network_uuids: Vec<Uuid> = body.networks.iter().map(|n| new_uuid(&n.uuid)).collect();
    check_unique_uuids(&network_uuids)?;
    let mut overwritten_networks = Vec::new();
    for (network, uuid) in body.networks.iter().zip(&network_uuids) {
        overwritten_networks.push(check_network(network, uuid, body.force, &context)?);
    }
    let mut replaced_filters = Vec::new();
    for filter in &body.network_filters {
        let virtual_machine_uuid = new_uuid(&filter.virtual_machine_uuid);
        check_virtual_machine(&virtual_machine_uuid, &context)?;
        let exists = network_filter_table::get_network_filter(
            &virtual_machine_uuid,
            filter.direction,
            &context,
        )
        .is_ok();
        if exists && !body.force {
            return Err(ErrorResponse::Conflict(format!(
                "Virtual_machine '{virtual_machine_uuid}' already has a {} packet-filter. Use \
                 'force' to replace it.",
                filter.direction
            )));
        }
        replaced_filters.push(exists);
    }
    for floating_ip in &body.floating_ips {
        if let Some(virtual_machine_uuid) = &floating_ip.virtual_machine_uuid {
            check_virtual_machine(&new_uuid(virtual_machine_uuid), &context)?;
        }
    }

    // networks
    let mut imported_networks = Vec::new();
    for ((network, uuid), overwritten) in body
        .networks
        .iter()
        .zip(network_uuids)
        .zip(overwritten_networks)
    {
        network_table::put_network(NetworkEntry {
            uuid,
            name: network.name.clone(),
            subnet: network.subnet.clone(),
            owner_id: context.user_id.clone(),
            project_id: project_id.clone(),
            created_at: Utc::now(),
            created_by: context.user_id.clone(),
            updated_at: Utc::now(),
            updated_by: context.user_id.clone(),
            disable_encryption: network.disable_encryption,
            // an overwritten network keeps its tenant, a new one gets it with its first address
            vni: None,
        })
        .map_err(|e| {
            map_db_write_error(
                &format!("import network with UUID '{uuid}'"),
                e,
                Some(&context),
            )
        })?;
        imported_networks.push(ImportedResource {
            source_uuid: network.uuid,
            uuid,
            overwritten,
        });
    }

    // packet filters
    for (filter, replace) in body.network_filters.iter().zip(replaced_filters) {
        import_network_filter(
            filter,
            &new_uuid(&filter.virtual_machine_uuid),
            replace,
            &context,
        )
        .await?;
    }

    // floating ip-addresses
    let mut imported_floating_ips = Vec::new();
    for floating_ip in &body.floating_ips {
        let virtual_machine_uuid = floating_ip.virtual_machine_uuid.as_ref().map(new_uuid);
        let uuid = import_floating_ip(floating_ip, virtual_machine_uuid.as_ref(), &context).await?;
        imported_floating_ips.push(ImportedResource {
            source_uuid: floating_ip.uuid,
            uuid,
            overwritten: false,
        });
    }

    Ok(Json(HanamiProjectImportResp {
        project_id: project_id.clone(),
        networks: imported_networks,
        floating_ips: imported_floating_ips,
    }))
}

/// Checks, if a network can be imported under a UUID.
///
/// # Arguments
///
/// * `network` - The network from the backup
/// * `uuid` - The UUID, under which the network is imported
/// * `force` - Overwrite an existing network of the project
/// * `context` - User-context with the project of the import
///
/// # Returns
///
/// * `Ok(bool)` - The network can be imported, true if an active network is overwritten
/// * `Err(ErrorResponse::Conflict)` - The UUID can't be used, or the subnet or the encryption of
///   a network, which is used by virtual_machines, would change
fn check_network(
    network: &HanamiNetworkExport,
    uuid: &Uuid,
    force: bool,
    context: &UserContext,
) -> Result<bool, ErrorResponse> {
    let existing = network_table::find_network(uuid)
        .map_err(|e| map_db_list_error("networks", e, Some(context)))?;
    let overwritten = check_import_target(
        existing.as_ref().map(|n| n.project_id.as_str()),
        &context.project_id,
        force,
        "Network",
        uuid,
    )?;

    // the addresses of the virtual_machines are bound to the subnet and the encryption
    if let Some(existing) = existing.filter(|_| overwritten) {
        let changed = existing.subnet != network.subnet
            || existing.disable_encryption != network.disable_encryption;
        let in_use = !address_table::list_addresses_of_network(uuid)
            .map_err(|e| map_db_list_error("addresses", e, Some(context)))?
            .is_empty();
        if changed && in_use {
            return Err(ErrorResponse::Conflict(format!(
                "Network with UUID '{uuid}' is used by virtual_machines, so its subnet and \
                 encryption can't be changed."
            )));
        }
    }
    Ok(overwritten)
}

/// Checks, that a virtual_machine exists within the project of the import.
///
/// # Arguments
///
/// * `virtual_machine_uuid` - The UUID of the virtual_machine
/// * `context` - User-context with the project of the import
///
/// # Returns
///
/// * `Ok(())` - The virtual_machine exists in the project
/// * `Err(ErrorResponse::BadRequest)` - The virtual_machine doesn't exist in the project
fn check_virtual_machine(
    virtual_machine_uuid: &Uuid,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    match meta_virtual_machine_table::get_meta_virtual_machine(virtual_machine_uuid, context) {
        Ok(virtual_machine) if virtual_machine.project_id == context.project_id => Ok(()),
        Ok(_) | Err(ainari_common::enums::DbError::NotFound) => {
            Err(ErrorResponse::BadRequest(format!(
                "Virtual_machine '{virtual_machine_uuid}' doesn't exist in project '{}'.",
                context.project_id
            )))
        }
        Err(e) => Err(map_db_uuid_get_delete_error(
            "virtual_machine",
            virtual_machine_uuid,
            e,
            Some(context),
        )),
    }
}

/// Applies a packet filter from a backup on the torii of the host of the virtual_machine.
///
/// # Arguments
///
/// * `filter` - The packet filter from the backup
/// * `virtual_machine_uuid` - The virtual_machine, which gets the filter
/// * `replace` - The virtual_machine already has a filter of this direction, which is replaced
/// * `context` - User-context with the project of the import
///
/// # Returns
///
/// * `Ok(())` - The filter is applied and stored
/// * `Err(ErrorResponse)` - The torii or the database failed
async fn import_network_filter(
    filter: &HanamiNetworkFilterExport,
    virtual_machine_uuid: &Uuid,
    replace: bool,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    let _lock = NETWORK_FILTER_LOCK.lock().await;
    let target = resolve_filter_target(virtual_machine_uuid, context).await?;

    let mut filter_resp = None;
    if replace {
        filter_resp = Some(
            network_filter_clients::clear_filter(
                &target.torii,
                &context.token,
                &config::INTERNAL_API_KEY,
                target.vni,
                &target.ip,
                filter.direction,
                config::CONFIG.skip_tls_verification,
            )
            .await
            .map_err(|e| map_ainari_error_to_api_response(e, Some(context)))?,
        );
    }
    if !filter.ip_ranges.is_empty() {
        filter_resp = Some(
            network_filter_clients::add_filter_ip_range(
                &target.torii,
                &context.token,
                &config::INTERNAL_API_KEY,
                target.vni,
                &target.ip,
                filter.direction,
                filter.ip_ranges.clone(),
                config::CONFIG.skip_tls_verification,
            )
            .await
            .map_err(|e| map_ainari_error_to_api_response(e, Some(context)))?,
        );
    }
    if !filter.ports.is_empty() {
        filter_resp = Some(
            network_filter_clients::add_filter_port(
                &target.torii,
                &context.token,
                &config::INTERNAL_API_KEY,
                target.vni,
                &target.ip,
                filter.direction,
                filter.ports.clone(),
                config::CONFIG.skip_tls_verification,
            )
            .await
            .map_err(|e| map_ainari_error_to_api_response(e, Some(context)))?,
        );
    }

    // the torii reports back the complete include-lists after each change
    if let Some(filter_resp) = filter_resp {
        store_network_filter(
            virtual_machine_uuid,
            filter.direction,
            &filter_resp,
            context,
        )?;
    }
    Ok(())
}

/// Allocates a floating ip-address from a backup again and attaches it to its virtual_machine.
///
/// The address of the backup is used, if it is still free, otherwise a free one of the pool. A
/// virtual_machine, which already has a floating ip-address, keeps it, so the import can be
/// repeated.
///
/// # Arguments
///
/// * `floating_ip` - The floating ip-address from the backup
/// * `virtual_machine_uuid` - The virtual_machine, which gets the floating ip-address
/// * `context` - User-context with the project of the import
///
/// # Returns
///
/// * `Ok(Uuid)` - The UUID of the new or already attached floating ip-address
/// * `Err(ErrorResponse)` - No floating ip-address is free or the attach failed
async fn import_floating_ip(
    floating_ip: &HanamiFloatingIpExport,
    virtual_machine_uuid: Option<&Uuid>,
    context: &UserContext,
) -> Result<Uuid, ErrorResponse> {
    if let Some(virtual_machine_uuid) = virtual_machine_uuid {
        let address =
            address_table::get_address_of_virtual_machine(virtual_machine_uuid).map_err(|e| {
                map_db_uuid_get_delete_error(
                    "address of virtual_machine",
                    virtual_machine_uuid,
                    e,
                    Some(context),
                )
            })?;
        let attached = floating_ip_table::get_floating_ip_by_internal_ip(
            &address.network_uuid,
            &address.internal_ip,
        )
        .map_err(|e| {
            map_db_uuid_get_delete_error(
                "floating_ip of virtual_machine",
                virtual_machine_uuid,
                e,
                Some(context),
            )
        })?;
        if let Some(attached) = attached {
            return Ok(attached.uuid);
        }
    }

    let cidr = &config::CONFIG.network.floating_ip_cidr;
    let reserved = match floating_ip_table::add_new_floating_ip(
        &floating_ip.name,
        Some(&floating_ip.floating_ip),
        cidr,
        context,
    ) {
        Err(FloatingIpReserveError::AlreadyUsed | FloatingIpReserveError::NotInRange) => {
            floating_ip_table::add_new_floating_ip(&floating_ip.name, None, cidr, context)
        }
        result => result,
    };
    let (floating_ip_uuid, _) = reserved.map_err(|e| match e {
        FloatingIpReserveError::NoFreeAddress => {
            ErrorResponse::Conflict("No free floating ip left.".to_string())
        }
        FloatingIpReserveError::PermissionDenied => permission_denied_response(),
        _ => {
            ainari_api::report_internal_error!(
                Some(context),
                "Failed to add floating_ip '{}' of backup to database.",
                floating_ip.uuid
            );
            ErrorResponse::InternalError("Internal Error".to_string())
        }
    })?;

    if let Some(virtual_machine_uuid) = virtual_machine_uuid
        && let Err(e) = attach_floating_ip(&floating_ip_uuid, virtual_machine_uuid, context).await
    {
        // an unattached floating ip-address is not what the backup contains
        let _ = floating_ip_table::force_delete_floating_ip(&floating_ip_uuid);
        return Err(e);
    }
    Ok(floating_ip_uuid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::http_endpoints::project::export_project_internal_v1_0::export_project_internal;
    use crate::database::address_table::AddressEntry;
    use crate::database::db_handle;
    use crate::database::floating_ip_table::FloatingIpEntry;
    use crate::database::meta_virtual_machine_table::MetaVirtualMachineEntry;
    use crate::database::network_filter_table::NetworkFilterEntry;
    use ainari_api_structs::network_filter_structs::FilterDirection;
    use ainari_common::enums::ProjectRole;
    use diesel::prelude::*;
    use serial_test::serial;
    use std::collections::BTreeMap;
    use std::net::Ipv4Addr;

    fn context(project: &str, is_admin: bool, role: ProjectRole) -> UserContext {
        UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: project.to_string(),
            is_admin: is_admin.to_string(),
            project_role: role.to_string(),
        }
    }

    fn new_project() -> String {
        format!("test-project-{}", Uuid::new_v4())
    }

    fn network_entry(project: &str, uuid: Uuid, subnet: &str) -> NetworkEntry {
        NetworkEntry {
            uuid,
            name: "network".to_string(),
            subnet: subnet.to_string(),
            owner_id: "test-user".to_string(),
            project_id: project.to_string(),
            created_at: Utc::now(),
            created_by: "test-user".to_string(),
            updated_at: Utc::now(),
            updated_by: "test-user".to_string(),
            disable_encryption: false,
            vni: None,
        }
    }

    /// Adds a virtual_machine together with its address to a network of a project.
    fn add_virtual_machine(project: &str, network: &Uuid, ip: Ipv4Addr) -> Uuid {
        let vm_uuid = Uuid::new_v4();
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        {
            use crate::database::meta_virtual_machine_table::meta_virtual_machines::dsl::*;
            diesel::insert_into(meta_virtual_machines)
                .values(MetaVirtualMachineEntry {
                    uuid: vm_uuid,
                    name: "vm".to_string(),
                    sakura_host_uuid: Uuid::new_v4(),
                    proxy_uuid: Uuid::new_v4(),
                    owner_id: "test-user".to_string(),
                    project_id: project.to_string(),
                    created_at: Utc::now(),
                    created_by: "test-user".to_string(),
                    updated_at: Utc::now(),
                    updated_by: "test-user".to_string(),
                    number_of_cores: 2,
                    memory_size: 2048,
                    disk_size: 20,
                })
                .execute(&mut *conn)
                .unwrap();
        }
        use crate::database::address_table::addresses::dsl::*;
        diesel::insert_into(addresses)
            .values(AddressEntry {
                uuid: Uuid::new_v4(),
                mac_address: format!("52:54:00:00:{:02x}:{:02x}", ip.octets()[2], ip.octets()[3]),
                tap_name: format!("tst{}", ip.octets()[3]),
                internal_ip: ip,
                network_uuid: *network,
                vni: 4242,
                host_address: "http://host".to_string(),
                virtual_machine_uuid: Some(vm_uuid),
                owner_id: "test-user".to_string(),
                project_id: project.to_string(),
                created_at: Utc::now(),
                created_by: "test-user".to_string(),
                updated_at: Utc::now(),
                updated_by: "test-user".to_string(),
            })
            .execute(&mut *conn)
            .unwrap();
        vm_uuid
    }

    fn add_floating_ip(project: &str, attached: Option<(&Uuid, Ipv4Addr)>, ip: Ipv4Addr) -> Uuid {
        let uuid = Uuid::new_v4();
        floating_ip_table::add_floating_ip(FloatingIpEntry {
            uuid,
            name: "floating".to_string(),
            network_uuid: attached.map(|(network, _)| *network),
            internal_ip_addr: attached.map(|(_, internal_ip)| internal_ip),
            floating_ip_addr: ip,
            owner_id: "test-user".to_string(),
            project_id: project.to_string(),
            created_at: Utc::now(),
            created_by: "test-user".to_string(),
            updated_at: Utc::now(),
            updated_by: "test-user".to_string(),
        })
        .unwrap();
        uuid
    }

    fn cleanup(projects: &[&str]) {
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        for project in projects {
            let project = project.to_string();
            {
                use crate::database::network_table::networks::dsl::*;
                let _ =
                    diesel::delete(networks.filter(project_id.eq(&project))).execute(&mut *conn);
            }
            {
                use crate::database::meta_virtual_machine_table::meta_virtual_machines::dsl::*;
                let _ = diesel::delete(meta_virtual_machines.filter(project_id.eq(&project)))
                    .execute(&mut *conn);
            }
            {
                use crate::database::address_table::addresses::dsl::*;
                let _ =
                    diesel::delete(addresses.filter(project_id.eq(&project))).execute(&mut *conn);
            }
            {
                use crate::database::network_filter_table::network_filters::dsl::*;
                let _ = diesel::delete(network_filters.filter(project_id.eq(&project)))
                    .execute(&mut *conn);
            }
            use crate::database::floating_ip_table::floating_ips::dsl::*;
            let _ =
                diesel::delete(floating_ips.filter(project_id.eq(&project))).execute(&mut *conn);
        }
    }

    async fn export(project: &str) -> HanamiProjectExport {
        export_project_internal(
            Path::from(project.to_string()),
            context(project, true, ProjectRole::Member),
        )
        .await
        .unwrap()
        .into_inner()
    }

    async fn import(
        project: &str,
        body: HanamiProjectImportReq,
        context: &UserContext,
    ) -> Result<HanamiProjectImportResp, ErrorResponse> {
        import_project_internal(Path::from(project.to_string()), Json(body), context.clone())
            .await
            .map(|resp| resp.into_inner())
    }

    fn request(export: &HanamiProjectExport, force: bool) -> HanamiProjectImportReq {
        HanamiProjectImportReq {
            networks: export.networks.clone(),
            network_filters: vec![],
            floating_ips: vec![],
            force,
            id_map: BTreeMap::new(),
        }
    }

    #[actix_rt::test]
    #[serial]
    async fn test_export_project() {
        let project = new_project();
        let network = Uuid::new_v4();
        network_table::put_network(network_entry(&project, network, "192.168.10.0/24")).unwrap();
        let ip = Ipv4Addr::new(192, 168, 10, 5);
        let virtual_machine = add_virtual_machine(&project, &network, ip);
        network_filter_table::add_network_filter(NetworkFilterEntry {
            uuid: Uuid::new_v4(),
            virtual_machine_uuid: virtual_machine,
            direction: FilterDirection::Ingress.to_string(),
            ip_ranges: "10.1.0.0/16".to_string(),
            ports: "22".to_string(),
            owner_id: "test-user".to_string(),
            project_id: project.clone(),
            created_at: Utc::now(),
            created_by: "test-user".to_string(),
            updated_at: Utc::now(),
            updated_by: "test-user".to_string(),
        })
        .unwrap();
        let attached =
            add_floating_ip(&project, Some((&network, ip)), Ipv4Addr::new(10, 0, 0, 201));
        let free = add_floating_ip(&project, None, Ipv4Addr::new(10, 0, 0, 202));

        let exported = export(&project).await;
        assert_eq!(exported.networks.len(), 1);
        assert_eq!(exported.networks[0].subnet, "192.168.10.0/24");
        assert_eq!(exported.virtual_machines.len(), 1);
        let exported_vm = &exported.virtual_machines[0];
        assert_eq!(exported_vm.uuid, virtual_machine);
        assert_eq!(exported_vm.network_uuid, network);
        assert_eq!(exported_vm.internal_ip, ip);
        assert_eq!(exported_vm.memory_size, 2048);
        assert_eq!(exported.network_filters.len(), 1);
        let filter = &exported.network_filters[0];
        assert_eq!(filter.virtual_machine_uuid, virtual_machine);
        assert_eq!(filter.ip_ranges[0].spec, "10.1.0.0/16");
        assert_eq!(filter.ports[0].spec, "22");
        // the floating ip-addresses know their virtual_machine
        let floating = |uuid| {
            exported
                .floating_ips
                .iter()
                .find(|f| f.uuid == uuid)
                .unwrap()
        };
        assert_eq!(
            floating(attached).virtual_machine_uuid,
            Some(virtual_machine)
        );
        assert_eq!(floating(free).virtual_machine_uuid, None);

        // users can only export their own project
        let other = context(&new_project(), false, ProjectRole::Admin);
        let result = export_project_internal(Path::from(project.clone()), other).await;
        assert!(matches!(result, Err(ErrorResponse::Forbidden(_))));

        cleanup(&[&project]);
    }

    #[actix_rt::test]
    #[serial]
    async fn test_import_networks() {
        let source = new_project();
        let network = Uuid::new_v4();
        network_table::put_network(network_entry(&source, network, "192.168.20.0/24")).unwrap();
        let exported = export(&source).await;

        // into another project with a new UUID
        let target = new_project();
        let new_network = Uuid::new_v4();
        let mut body = request(&exported, false);
        body.id_map = BTreeMap::from([(network, new_network)]);
        let resp = import(&target, body, &context(&target, true, ProjectRole::Member))
            .await
            .unwrap();
        assert_eq!(resp.networks[0].uuid, new_network);
        let imported = network_table::find_network(&new_network).unwrap().unwrap();
        assert_eq!(imported.project_id, target);
        assert_eq!(imported.subnet, "192.168.20.0/24");

        // the UUID of another project is never overwritten
        let admin = context(&target, true, ProjectRole::Member);
        assert!(matches!(
            import(&target, request(&exported, true), &admin).await,
            Err(ErrorResponse::Conflict(_))
        ));

        // within the same project only with force
        let user = context(&source, false, ProjectRole::Admin);
        assert!(matches!(
            import(&source, request(&exported, false), &user).await,
            Err(ErrorResponse::Conflict(_))
        ));
        let resp = import(&source, request(&exported, true), &user)
            .await
            .unwrap();
        assert!(resp.networks[0].overwritten);

        // the subnet of a network with virtual_machines can't be changed
        add_virtual_machine(&source, &network, Ipv4Addr::new(192, 168, 20, 5));
        let mut changed = exported.clone();
        changed.networks[0].subnet = "192.168.30.0/24".to_string();
        assert!(matches!(
            import(&source, request(&changed, true), &user).await,
            Err(ErrorResponse::Conflict(_))
        ));
        assert_eq!(
            network_table::find_network(&network)
                .unwrap()
                .unwrap()
                .subnet,
            "192.168.20.0/24"
        );

        // observers can't import
        let observer = context(&source, false, ProjectRole::Observer);
        assert!(matches!(
            import(&source, request(&exported, true), &observer).await,
            Err(ErrorResponse::Forbidden(_))
        ));

        cleanup(&[&source, &target]);
    }

    #[actix_rt::test]
    #[serial]
    async fn test_import_requires_virtual_machines() {
        let project = new_project();
        let network = Uuid::new_v4();
        let exported = HanamiProjectExport {
            networks: vec![HanamiNetworkExport {
                uuid: network,
                name: "network".to_string(),
                subnet: "192.168.40.0/24".to_string(),
                disable_encryption: false,
            }],
            virtual_machines: vec![],
            network_filters: vec![HanamiNetworkFilterExport {
                virtual_machine_uuid: Uuid::new_v4(),
                direction: FilterDirection::Egress,
                ip_ranges: vec!["10.0.0.0/8".parse().unwrap()],
                ports: vec![],
            }],
            floating_ips: vec![],
        };
        let mut body = request(&exported, false);
        body.network_filters = exported.network_filters.clone();

        // the filter of an unknown virtual_machine is rejected, before the network is created
        let user = context(&project, false, ProjectRole::Admin);
        assert!(matches!(
            import(&project, body, &user).await,
            Err(ErrorResponse::BadRequest(_))
        ));
        assert!(network_table::find_network(&network).unwrap().is_none());

        cleanup(&[&project]);
    }

    #[actix_rt::test]
    #[serial]
    async fn test_import_floating_ips() {
        let project = new_project();
        let network = Uuid::new_v4();
        network_table::put_network(network_entry(&project, network, "192.168.50.0/24")).unwrap();
        let ip = Ipv4Addr::new(192, 168, 50, 5);
        let virtual_machine = add_virtual_machine(&project, &network, ip);
        let attached =
            add_floating_ip(&project, Some((&network, ip)), Ipv4Addr::new(10, 0, 0, 211));
        let taken = Ipv4Addr::new(10, 0, 0, 212);
        add_floating_ip(&project, None, taken);
        let user = context(&project, false, ProjectRole::Admin);
        let floating = |uuid, ip, virtual_machine_uuid| HanamiFloatingIpExport {
            uuid,
            name: "restored".to_string(),
            floating_ip: ip,
            virtual_machine_uuid,
        };
        let (free_source, taken_source, attached_source) =
            (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let free = Ipv4Addr::new(10, 0, 0, 213);
        let body = HanamiProjectImportReq {
            networks: vec![],
            network_filters: vec![],
            floating_ips: vec![
                floating(free_source, free, None),
                floating(taken_source, taken, None),
                floating(
                    attached_source,
                    Ipv4Addr::new(10, 0, 0, 214),
                    Some(virtual_machine),
                ),
            ],
            force: false,
            id_map: BTreeMap::new(),
        };

        let resp = import(&project, body, &user).await.unwrap();
        let new_uuid = |source| {
            resp.floating_ips
                .iter()
                .find(|r| r.source_uuid == source)
                .unwrap()
                .uuid
        };
        let address_of = |uuid| {
            floating_ip_table::get_floating_ip(&uuid, &user)
                .ok()
                .unwrap()
                .floating_ip_addr
        };
        // the address of the backup is used, if it is still free
        assert_eq!(address_of(new_uuid(free_source)), free);
        assert_ne!(address_of(new_uuid(taken_source)), taken);
        // a virtual_machine, which already has a floating ip-address, keeps it
        assert_eq!(new_uuid(attached_source), attached);

        cleanup(&[&project]);
    }
}
