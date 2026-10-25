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

//! Plan of a restore: which resources keep their UUID, which get a new one, which are overwritten
//! or deleted, and how the quota of the project is used afterwards. The plan is calculated before
//! anything is changed, so it can be shown by a dry-run and the quota is checked before the
//! restore starts.

use std::collections::BTreeSet;
use uuid::Uuid;

use ainari_api_structs::backup_structs::{
    BackupKind, IdMapEntryResp, PlannedResource, QuotaUsageResp, RestorePlanResp,
};
use ainari_api_structs::image_structs::RyokanImageExport;
use ainari_api_structs::network_structs::HanamiProjectExport;
use ainari_api_structs::project_structs::{MikoQuotaExport, OmamoriProjectExport};
use ainari_api_structs::secret_structs::{OmamoriSecretExport, SECRET_OWNER_BACKUP};
use ainari_clients::image::export_project_images;
use ainari_clients::network::export_project_networks;
use ainari_clients::project::export_project;
use ainari_clients::secret::export_project_secrets;
use ainari_common::error::AinariError;

use super::BackupMetadata;
use crate::core::keys::BackupKeys;
use crate::core::services::Services;

pub const SECRET: &str = "secret";
pub const PUBLIC_KEY: &str = "public_key";
pub const IMAGE: &str = "image";
pub const NETWORK: &str = "network";
pub const VIRTUAL_MACHINE: &str = "virtual_machine";
pub const FLOATING_IP: &str = "floating_ip";
pub const NETWORK_FILTER: &str = "network_filter";

/// Kind of owner of the secrets, which encrypt the files of images
pub const SECRET_OWNER_IMAGE: &str = "image";
/// Kind of owner of the secrets, which were created by the users
const SECRET_OWNER_USER: &str = "user";

/// Order, in which the resources are deleted by `prune`, so a resource is deleted before the
/// resources, which it depends on.
pub const PRUNE_ORDER: [&str; 7] = [
    FLOATING_IP,
    NETWORK_FILTER,
    VIRTUAL_MACHINE,
    NETWORK,
    IMAGE,
    PUBLIC_KEY,
    SECRET,
];

/// Resources of the project, into which a backup is restored, before the restore.
#[derive(Debug, Clone, Default)]
pub struct TargetState {
    /// false, if the project doesn't exist yet and is created by the restore
    pub exists: bool,
    /// quota of the project, if it exists
    pub quota: Option<MikoQuotaExport>,
    pub hanami: HanamiProjectExport,
    pub images: Vec<RyokanImageExport>,
    pub omamori: OmamoriProjectExport,
}

/// Options of a restore, which influence its plan.
#[derive(Debug, Clone, Copy)]
pub struct PlanOptions {
    pub kind: BackupKind,
    pub force: bool,
    pub prune: bool,
    /// true, if the backup is restored into the project, which it was created from
    pub same_project: bool,
    /// true, if the quota of the backup is applied to the project, because the project is
    /// created or overwritten by an admin
    pub import_quota: bool,
    pub include_dependencies: bool,
}

/// Plan of a restore.
#[derive(Debug, Clone, Default)]
pub struct RestorePlan {
    pub target_project_id: String,
    pub create_project: bool,
    /// resources, which get a new UUID
    pub id_map: Vec<IdMapEntryResp>,
    /// existing resources, which are overwritten
    pub overwritten: Vec<PlannedResource>,
    /// resources of the project, which are deleted by `prune`
    pub deleted: Vec<PlannedResource>,
    pub quota: Vec<QuotaUsageResp>,
    pub warnings: Vec<String>,
}

impl RestorePlan {
    /// Resources of one kind, which are deleted by `prune`.
    pub fn deleted_of<'a>(
        &'a self,
        resource_type: &'a str,
    ) -> impl Iterator<Item = &'a PlannedResource> + 'a {
        self.deleted
            .iter()
            .filter(move |resource| resource.resource_type == resource_type)
    }

    /// Describes the kinds of resources, whose quota would be exceeded by the restore.
    ///
    /// # Returns
    ///
    /// The description, or `None`, if the restore fits into the quota.
    pub fn exceeded_quota(&self) -> Option<String> {
        let exceeded: Vec<String> = self
            .quota
            .iter()
            .filter(|usage| usage.is_exceeded())
            .map(|usage| {
                format!(
                    "{} ({} instead of at most {})",
                    usage.resource_type, usage.after_restore, usage.max
                )
            })
            .collect();
        (!exceeded.is_empty()).then(|| {
            format!(
                "The restore would exceed the quota of the project: {}",
                exceeded.join(", ")
            )
        })
    }

    /// Converts the plan into its response.
    pub fn to_resp(&self) -> RestorePlanResp {
        RestorePlanResp {
            target_project_id: self.target_project_id.clone(),
            create_project: self.create_project,
            id_map: self.id_map.clone(),
            overwritten: self.overwritten.clone(),
            deleted: self.deleted.clone(),
            quota: self.quota.clone(),
            warnings: self.warnings.clone(),
        }
    }

    /// Decides, under which UUID a resource of the backup is restored.
    ///
    /// With `force` every UUID is kept and an existing resource is overwritten. Without it, a
    /// resource, whose UUID is used in the project, gets a new UUID. Into another project all
    /// resources get new UUIDs, because their UUIDs are still used by the project of the backup.
    fn decide(
        &mut self,
        options: &PlanOptions,
        resource_type: &str,
        uuid: Uuid,
        name: &str,
        existing: bool,
    ) {
        if existing && options.force {
            self.overwritten.push(planned(resource_type, uuid, name));
        } else if existing || !options.same_project && !options.force {
            self.id_map.push(IdMapEntryResp {
                resource_type: resource_type.to_string(),
                old_uuid: uuid,
                new_uuid: Uuid::new_v4(),
            });
        }
    }

    fn is_overwritten(&self, uuid: &Uuid) -> bool {
        self.overwritten
            .iter()
            .any(|resource| resource.uuid == *uuid)
    }
}

fn planned(resource_type: &str, uuid: Uuid, name: &str) -> PlannedResource {
    PlannedResource {
        resource_type: resource_type.to_string(),
        uuid,
        name: name.to_string(),
    }
}

/// True, if the files of the images of the backup are part of it, so the images are restored.
pub fn images_restored(metadata: &BackupMetadata) -> bool {
    metadata
        .ryokan
        .as_ref()
        .is_some_and(|ryokan| ryokan.files_included)
}

/// Secrets of the backup, which are imported by a restore of a project.
///
/// The secrets of the disks belong to the backup itself and are never imported. The secrets of the
/// images are only imported, if the images are restored.
pub fn restored_secrets(metadata: &BackupMetadata) -> Vec<OmamoriSecretExport> {
    let with_images = images_restored(metadata);
    metadata
        .omamori
        .secrets
        .iter()
        .filter(|secret| secret.owned_by != SECRET_OWNER_BACKUP)
        .filter(|secret| with_images || secret.owned_by != SECRET_OWNER_IMAGE)
        .cloned()
        .collect()
}

/// Calculates the plan of a restore.
///
/// # Arguments
///
/// * `metadata` - The decrypted metadata of the backup
/// * `target` - The resources of the project before the restore
/// * `options` - The options of the restore
/// * `target_project_id` - The project, into which the backup is restored
///
/// # Returns
///
/// The plan of the restore.
pub fn calculate_plan(
    metadata: &BackupMetadata,
    target: &TargetState,
    options: &PlanOptions,
    target_project_id: &str,
) -> RestorePlan {
    let mut plan = RestorePlan {
        target_project_id: target_project_id.to_string(),
        create_project: !target.exists,
        ..Default::default()
    };

    match options.kind {
        BackupKind::Project => plan_project(metadata, target, options, &mut plan),
        BackupKind::VirtualMachine => plan_virtual_machine(metadata, target, options, &mut plan),
    }
    if options.prune && options.kind == BackupKind::Project {
        plan_prune(metadata, target, &mut plan);
    }
    plan.quota = calculate_quota(metadata, target, options, &plan);

    plan
}

/// Plans the restore of all resources of a project.
fn plan_project(
    metadata: &BackupMetadata,
    target: &TargetState,
    options: &PlanOptions,
    plan: &mut RestorePlan,
) {
    let target_secrets: BTreeSet<Uuid> = target.omamori.secrets.iter().map(|s| s.uuid).collect();
    for secret in restored_secrets(metadata) {
        let existing = target_secrets.contains(&secret.uuid);
        plan.decide(options, SECRET, secret.uuid, &secret.name, existing);
    }

    let target_keys: BTreeSet<Uuid> = target.omamori.public_keys.iter().map(|k| k.uuid).collect();
    for public_key in &metadata.omamori.public_keys {
        let existing = target_keys.contains(&public_key.uuid);
        plan.decide(
            options,
            PUBLIC_KEY,
            public_key.uuid,
            &public_key.name,
            existing,
        );
    }

    match &metadata.ryokan {
        Some(ryokan) if ryokan.files_included => {
            let target_images: BTreeSet<Uuid> = target.images.iter().map(|i| i.uuid).collect();
            for image in &ryokan.images {
                let existing = target_images.contains(&image.uuid);
                plan.decide(options, IMAGE, image.uuid, &image.name, existing);
            }
        }
        Some(ryokan) if !ryokan.images.is_empty() => plan.warnings.push(format!(
            "The files of the {} images of the project are not part of the backup, so the images \
             are not restored.",
            ryokan.images.len()
        )),
        _ => {}
    }

    let target_networks: BTreeSet<Uuid> = target.hanami.networks.iter().map(|n| n.uuid).collect();
    for network in &metadata.hanami.networks {
        let existing = target_networks.contains(&network.uuid);
        plan.decide(options, NETWORK, network.uuid, &network.name, existing);
    }

    for virtual_machine in &metadata.hanami.virtual_machines {
        let existing = target
            .hanami
            .virtual_machines
            .iter()
            .find(|entry| entry.uuid == virtual_machine.uuid);
        if let Some(existing) = existing
            && options.force
            && existing.disk_size != virtual_machine.disk_size
        {
            plan.warnings.push(format!(
                "Virtual_machine '{}' has another disk-size than in the backup, so it is deleted \
                 and created again.",
                virtual_machine.uuid
            ));
        }
        plan.decide(
            options,
            VIRTUAL_MACHINE,
            virtual_machine.uuid,
            &virtual_machine.name,
            existing.is_some(),
        );
    }

    // a public-key, which was deleted before the backup, has to exist in the project
    let backup_keys: BTreeSet<Uuid> = metadata
        .omamori
        .public_keys
        .iter()
        .map(|k| k.uuid)
        .collect();
    for virtual_machine in &metadata.sakura.virtual_machines {
        let key = virtual_machine.config.public_key_uuid;
        if !backup_keys.contains(&key) && !target_keys.contains(&key) {
            plan.warnings.push(format!(
                "Public-key '{key}' of virtual_machine '{}' is neither part of the backup nor of \
                 the project, so the virtual_machine can't be restored.",
                virtual_machine.config.uuid
            ));
        }
    }

    if options.force && !options.same_project {
        plan.warnings.push(
            "With 'force' all resources keep their UUIDs, so the restore fails, if the UUIDs are \
             still used by the project of the backup."
                .to_string(),
        );
    }
}

/// Plans the restore of a single virtual_machine.
///
/// Its public-key and network are only created from the backup, if they don't exist in the
/// project anymore, and are never overwritten.
fn plan_virtual_machine(
    metadata: &BackupMetadata,
    target: &TargetState,
    options: &PlanOptions,
    plan: &mut RestorePlan,
) {
    for virtual_machine in &metadata.hanami.virtual_machines {
        let existing = target
            .hanami
            .virtual_machines
            .iter()
            .any(|entry| entry.uuid == virtual_machine.uuid);
        // into another project the virtual_machine gets a new UUID, because its UUID is still used
        // by the project of the backup
        plan.decide(
            options,
            VIRTUAL_MACHINE,
            virtual_machine.uuid,
            &virtual_machine.name,
            existing,
        );

        let network = virtual_machine.network_uuid;
        let network_exists = target.hanami.networks.iter().any(|n| n.uuid == network);
        let network_in_backup = metadata.hanami.networks.iter().any(|n| n.uuid == network);
        if !network_exists && !(options.include_dependencies && network_in_backup) {
            plan.warnings.push(format!(
                "Network '{network}' of virtual_machine '{}' doesn't exist in the project and is \
                 not restored, so the virtual_machine can't be restored.",
                virtual_machine.uuid
            ));
        }
    }

    for virtual_machine in &metadata.sakura.virtual_machines {
        let key = virtual_machine.config.public_key_uuid;
        let key_exists = target.omamori.public_keys.iter().any(|k| k.uuid == key);
        let key_in_backup = metadata.omamori.public_keys.iter().any(|k| k.uuid == key);
        if !key_exists && !key_in_backup {
            plan.warnings.push(format!(
                "Public-key '{key}' of virtual_machine '{}' is neither part of the backup nor of \
                 the project, so the virtual_machine can't be restored.",
                virtual_machine.config.uuid
            ));
        }
    }
}

/// Plans the deletion of the resources of the project, which are not part of the backup.
///
/// `prune` is only allowed together with `force`, so all resources of the backup keep their UUIDs
/// and are compared by them.
fn plan_prune(metadata: &BackupMetadata, target: &TargetState, plan: &mut RestorePlan) {
    let hanami = &metadata.hanami;
    let backup_vms: BTreeSet<Uuid> = hanami.virtual_machines.iter().map(|vm| vm.uuid).collect();

    // the floating ip-addresses are allocated again, so only the ones are kept, which are
    // attached to a virtual_machine, which also had one in the backup
    let backup_fips: BTreeSet<Uuid> = hanami.floating_ips.iter().map(|f| f.uuid).collect();
    let vms_with_fip: BTreeSet<Uuid> = hanami
        .floating_ips
        .iter()
        .filter_map(|f| f.virtual_machine_uuid)
        .collect();
    for floating_ip in &target.hanami.floating_ips {
        let kept = backup_fips.contains(&floating_ip.uuid)
            || floating_ip
                .virtual_machine_uuid
                .is_some_and(|vm| vms_with_fip.contains(&vm));
        if !kept {
            plan.deleted
                .push(planned(FLOATING_IP, floating_ip.uuid, &floating_ip.name));
        }
    }

    // the filters of the deleted virtual_machines are deleted together with them
    let backup_filters: BTreeSet<(Uuid, String)> = hanami
        .network_filters
        .iter()
        .map(|f| (f.virtual_machine_uuid, f.direction.to_string()))
        .collect();
    for filter in &target.hanami.network_filters {
        let key = (filter.virtual_machine_uuid, filter.direction.to_string());
        if backup_vms.contains(&filter.virtual_machine_uuid) && !backup_filters.contains(&key) {
            plan.deleted.push(planned(NETWORK_FILTER, key.0, &key.1));
        }
    }

    for virtual_machine in &target.hanami.virtual_machines {
        if !backup_vms.contains(&virtual_machine.uuid) {
            plan.deleted.push(planned(
                VIRTUAL_MACHINE,
                virtual_machine.uuid,
                &virtual_machine.name,
            ));
        }
    }

    let backup_networks: BTreeSet<Uuid> = hanami.networks.iter().map(|n| n.uuid).collect();
    for network in &target.hanami.networks {
        if !backup_networks.contains(&network.uuid) {
            plan.deleted
                .push(planned(NETWORK, network.uuid, &network.name));
        }
    }

    // images, which are listed in the backup, are kept, also if their files are not part of it
    let backup_images: BTreeSet<Uuid> = metadata
        .ryokan
        .iter()
        .flat_map(|ryokan| ryokan.images.iter().map(|i| i.uuid))
        .collect();
    for image in &target.images {
        if !backup_images.contains(&image.uuid) {
            plan.deleted.push(planned(IMAGE, image.uuid, &image.name));
        }
    }

    let backup_keys: BTreeSet<Uuid> = metadata
        .omamori
        .public_keys
        .iter()
        .map(|k| k.uuid)
        .collect();
    for public_key in &target.omamori.public_keys {
        if !backup_keys.contains(&public_key.uuid) {
            plan.deleted
                .push(planned(PUBLIC_KEY, public_key.uuid, &public_key.name));
        }
    }

    // the secrets of the images are deleted together with their images
    let backup_secrets: BTreeSet<Uuid> = metadata.omamori.secrets.iter().map(|s| s.uuid).collect();
    for secret in &target.omamori.secrets {
        if secret.owned_by == SECRET_OWNER_USER && !backup_secrets.contains(&secret.uuid) {
            plan.deleted
                .push(planned(SECRET, secret.uuid, &secret.name));
        }
    }
}

/// Calculates the usage of the quota of the project after the restore.
fn calculate_quota(
    metadata: &BackupMetadata,
    target: &TargetState,
    options: &PlanOptions,
    plan: &RestorePlan,
) -> Vec<QuotaUsageResp> {
    let backup_quota = metadata.miko.as_ref().map(|miko| miko.quota.clone());
    let quota = if options.import_quota {
        backup_quota.or_else(|| target.quota.clone())
    } else {
        target.quota.clone()
    };
    let Some(quota) = quota else {
        return Vec::new();
    };

    let deleted = |resource_type: &str| plan.deleted_of(resource_type).count() as i64;
    let created = |uuids: &mut dyn Iterator<Item = Uuid>| {
        uuids.filter(|uuid| !plan.is_overwritten(uuid)).count() as i64
    };
    let usage = |resource_type: &str, current: usize, created: i64, max: i32| QuotaUsageResp {
        resource_type: resource_type.to_string(),
        current: current as i64,
        after_restore: current as i64 - deleted(resource_type) + created,
        max: i64::from(max),
    };

    let hanami = &metadata.hanami;
    let mut result = vec![usage(
        VIRTUAL_MACHINE,
        target.hanami.virtual_machines.len(),
        created(&mut hanami.virtual_machines.iter().map(|vm| vm.uuid)),
        quota.max_virtual_machine,
    )];

    // a virtual_machine, which already has a floating ip-address, keeps it
    let target_vms_with_fip: BTreeSet<Uuid> = target
        .hanami
        .floating_ips
        .iter()
        .filter(|f| !plan.deleted_of(FLOATING_IP).any(|d| d.uuid == f.uuid))
        .filter_map(|f| f.virtual_machine_uuid)
        .collect();
    let new_fips = hanami
        .floating_ips
        .iter()
        .filter(|f| {
            !f.virtual_machine_uuid
                .is_some_and(|vm| plan.is_overwritten(&vm) && target_vms_with_fip.contains(&vm))
        })
        .count() as i64;
    result.push(usage(
        FLOATING_IP,
        target.hanami.floating_ips.len(),
        new_fips,
        quota.max_floating_ip,
    ));

    match options.kind {
        BackupKind::Project => {
            result.push(usage(
                NETWORK,
                target.hanami.networks.len(),
                created(&mut hanami.networks.iter().map(|n| n.uuid)),
                quota.max_network,
            ));
            let images = match &metadata.ryokan {
                Some(ryokan) if ryokan.files_included => {
                    created(&mut ryokan.images.iter().map(|i| i.uuid))
                }
                _ => 0,
            };
            result.push(usage(IMAGE, target.images.len(), images, quota.max_image));
            result.push(usage(
                SECRET,
                target.omamori.secrets.len(),
                created(&mut restored_secrets(metadata).iter().map(|s| s.uuid)),
                quota.max_secret,
            ));
        }
        BackupKind::VirtualMachine => {
            // the network is only created, if it doesn't exist anymore
            let new_networks = hanami
                .networks
                .iter()
                .filter(|_| options.include_dependencies)
                .filter(|n| !target.hanami.networks.iter().any(|t| t.uuid == n.uuid))
                .count() as i64;
            result.push(usage(
                NETWORK,
                target.hanami.networks.len(),
                new_networks,
                quota.max_network,
            ));
        }
    }

    result
}

/// Reads the resources of the project, into which a backup is restored.
///
/// A project, which doesn't exist, has no resources and no quota.
///
/// # Arguments
///
/// * `services` - Connection to the other components
/// * `project_id` - The project
/// * `keys` - Keys of the backup, whose wrap-key is used for the export of the secrets
///
/// # Returns
///
/// The resources of the project.
pub async fn load_target_state(
    services: &Services,
    project_id: &str,
    keys: &BackupKeys,
) -> Result<TargetState, AinariError> {
    let project = match export_project(
        &services.miko,
        &services.token,
        services.key(),
        project_id,
        services.insecure(),
    )
    .await
    {
        Ok(project) => Some(project),
        Err(AinariError::NotFound(_)) => None,
        Err(e) => return Err(e),
    };
    let Some(project) = project else {
        return Ok(TargetState::default());
    };

    let hanami = export_project_networks(
        &services.endpoints.hanami,
        &services.token,
        services.key(),
        project_id,
        services.insecure(),
    )
    .await?;
    let ryokan = export_project_images(
        &services.endpoints.ryokan,
        &services.token,
        services.key(),
        project_id,
        services.insecure(),
    )
    .await?;
    // the payloads of the secrets are not required, but are only exported wrapped
    let omamori = export_project_secrets(
        &services.endpoints.omamori,
        &services.token,
        services.key(),
        project_id,
        &keys.wrap_key_base64(),
        None,
        services.insecure(),
    )
    .await?;

    Ok(TargetState {
        exists: true,
        quota: Some(project.quota),
        hanami,
        images: ryokan.images,
        omamori,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::bundle::{
        BackupDisk, BackupVirtualMachine, RyokanMetadata, SakuraMetadata, disk_path,
    };
    use ainari_api_structs::floating_ip_structs::HanamiFloatingIpExport;
    use ainari_api_structs::network_filter_structs::{FilterDirection, HanamiNetworkFilterExport};
    use ainari_api_structs::network_structs::HanamiNetworkExport;
    use ainari_api_structs::project_structs::MikoProjectExport;
    use ainari_api_structs::public_key_structs::OmamoriPublicKeyExport;
    use ainari_api_structs::virtual_machine_structs::{
        HanamiVirtualMachineExport, SakuraVirtualMachineExport,
    };
    use std::net::Ipv4Addr;

    /// UUIDs of the resources of the test-backup.
    struct Ids {
        user_secret: Uuid,
        image_secret: Uuid,
        disk_secret: Uuid,
        key: Uuid,
        image: Uuid,
        network: Uuid,
        vm: Uuid,
        fip: Uuid,
    }

    fn ids() -> Ids {
        Ids {
            user_secret: Uuid::new_v4(),
            image_secret: Uuid::new_v4(),
            disk_secret: Uuid::new_v4(),
            key: Uuid::new_v4(),
            image: Uuid::new_v4(),
            network: Uuid::new_v4(),
            vm: Uuid::new_v4(),
            fip: Uuid::new_v4(),
        }
    }

    fn quota(max: i32) -> MikoQuotaExport {
        MikoQuotaExport {
            max_virtual_machine: max,
            max_image: max,
            max_secret: max,
            max_network: max,
            max_floating_ip: max,
        }
    }

    fn secret(uuid: Uuid, owned_by: &str) -> OmamoriSecretExport {
        OmamoriSecretExport {
            uuid,
            name: "secret".to_string(),
            owned_by: owned_by.to_string(),
            resource_uuid: None,
            wrapped_payload: "wrapped".to_string(),
        }
    }

    fn public_key(uuid: Uuid) -> OmamoriPublicKeyExport {
        OmamoriPublicKeyExport {
            uuid,
            name: "key".to_string(),
            public_key: "ssh-ed25519 AAAA test".to_string(),
        }
    }

    fn image(uuid: Uuid, secret_uuid: Uuid) -> RyokanImageExport {
        RyokanImageExport {
            uuid,
            name: "image".to_string(),
            file_type: "qcow2".to_string(),
            file_size: 1,
            is_snapshot: false,
            secret_uuid,
        }
    }

    fn network(uuid: Uuid) -> HanamiNetworkExport {
        HanamiNetworkExport {
            uuid,
            name: "network".to_string(),
            subnet: "192.168.100.0/24".to_string(),
            disable_encryption: false,
        }
    }

    fn hanami_vm(uuid: Uuid, network_uuid: Uuid, disk_size: i64) -> HanamiVirtualMachineExport {
        HanamiVirtualMachineExport {
            uuid,
            name: "virtual-machine".to_string(),
            number_of_cores: 1,
            memory_size: 1024,
            disk_size,
            network_uuid,
            internal_ip: Ipv4Addr::new(192, 168, 100, 2),
            mac_address: "52:54:00:00:00:01".to_string(),
        }
    }

    fn fip(uuid: Uuid, vm: Option<Uuid>) -> HanamiFloatingIpExport {
        HanamiFloatingIpExport {
            uuid,
            name: "fip".to_string(),
            floating_ip: Ipv4Addr::new(10, 0, 0, 5),
            virtual_machine_uuid: vm,
        }
    }

    fn filter(vm: Uuid, direction: FilterDirection) -> HanamiNetworkFilterExport {
        HanamiNetworkFilterExport {
            virtual_machine_uuid: vm,
            direction,
            ip_ranges: Vec::new(),
            ports: Vec::new(),
        }
    }

    /// Backup of a project with one resource of every kind.
    fn backup(ids: &Ids, files_included: bool) -> BackupMetadata {
        BackupMetadata {
            miko: Some(MikoProjectExport {
                id: "project".to_string(),
                name: "project".to_string(),
                quota: quota(10),
            }),
            omamori: OmamoriProjectExport {
                secrets: vec![
                    secret(ids.user_secret, "user"),
                    secret(ids.image_secret, SECRET_OWNER_IMAGE),
                    secret(ids.disk_secret, SECRET_OWNER_BACKUP),
                ],
                public_keys: vec![public_key(ids.key)],
            },
            ryokan: Some(RyokanMetadata {
                images: vec![image(ids.image, ids.image_secret)],
                files_included,
            }),
            hanami: HanamiProjectExport {
                networks: vec![network(ids.network)],
                virtual_machines: vec![hanami_vm(ids.vm, ids.network, 10)],
                network_filters: vec![filter(ids.vm, FilterDirection::Ingress)],
                floating_ips: vec![fip(ids.fip, Some(ids.vm))],
            },
            sakura: SakuraMetadata {
                virtual_machines: vec![BackupVirtualMachine {
                    config: SakuraVirtualMachineExport {
                        uuid: ids.vm,
                        name: "virtual-machine".to_string(),
                        vm_state: "RUNNING".to_string(),
                        number_of_cores: 1,
                        memory_size: 1 << 30,
                        disk_size: 10,
                        image_uuid: ids.image,
                        public_key_uuid: ids.key,
                        network_uuid: ids.network,
                        internal_ip: Ipv4Addr::new(192, 168, 100, 2),
                        mac_address: "52:54:00:00:00:01".to_string(),
                    },
                    disk: BackupDisk {
                        path: disk_path(&ids.vm),
                        secret_uuid: ids.disk_secret,
                    },
                }],
            },
        }
    }

    /// Project, which still contains all resources of the backup.
    fn unchanged_target(ids: &Ids) -> TargetState {
        let metadata = backup(ids, true);
        TargetState {
            exists: true,
            quota: Some(quota(10)),
            hanami: metadata.hanami.clone(),
            images: metadata.ryokan.unwrap().images,
            omamori: OmamoriProjectExport {
                secrets: vec![
                    secret(ids.user_secret, "user"),
                    secret(ids.image_secret, SECRET_OWNER_IMAGE),
                ],
                public_keys: vec![public_key(ids.key)],
            },
        }
    }

    fn options(force: bool, prune: bool, same_project: bool) -> PlanOptions {
        PlanOptions {
            kind: BackupKind::Project,
            force,
            prune,
            same_project,
            import_quota: false,
            include_dependencies: true,
        }
    }

    fn mapped(plan: &RestorePlan) -> BTreeSet<Uuid> {
        plan.id_map.iter().map(|entry| entry.old_uuid).collect()
    }

    fn usage<'a>(plan: &'a RestorePlan, resource_type: &str) -> &'a QuotaUsageResp {
        plan.quota
            .iter()
            .find(|usage| usage.resource_type == resource_type)
            .unwrap()
    }

    #[test]
    fn test_restore_into_deleted_project_keeps_uuids() {
        let ids = ids();
        let metadata = backup(&ids, true);
        let plan = calculate_plan(
            &metadata,
            &TargetState::default(),
            &options(false, false, true),
            "project",
        );

        assert!(plan.create_project);
        assert!(plan.id_map.is_empty());
        assert!(plan.overwritten.is_empty());
        assert!(plan.deleted.is_empty());
        // a project, which doesn't exist, has no quota yet
        assert!(plan.quota.is_empty());
    }

    #[test]
    fn test_used_uuids_are_remapped_without_force() {
        let ids = ids();
        let metadata = backup(&ids, true);
        let plan = calculate_plan(
            &metadata,
            &unchanged_target(&ids),
            &options(false, false, true),
            "project",
        );

        // every resource, which still exists, gets a new UUID, but never the disk-secret
        let expected = BTreeSet::from([
            ids.user_secret,
            ids.image_secret,
            ids.key,
            ids.image,
            ids.network,
            ids.vm,
        ]);
        assert_eq!(mapped(&plan), expected);
        assert!(
            plan.id_map
                .iter()
                .all(|entry| entry.old_uuid != entry.new_uuid)
        );
        assert!(plan.overwritten.is_empty());

        // all resources are created again next to the existing ones
        assert_eq!(usage(&plan, VIRTUAL_MACHINE).after_restore, 2);
        assert_eq!(usage(&plan, SECRET).after_restore, 4);
        assert_eq!(usage(&plan, FLOATING_IP).after_restore, 2);
        assert!(plan.exceeded_quota().is_none());
    }

    #[test]
    fn test_force_overwrites_existing_resources() {
        let ids = ids();
        let metadata = backup(&ids, true);
        let plan = calculate_plan(
            &metadata,
            &unchanged_target(&ids),
            &options(true, false, true),
            "project",
        );

        assert!(plan.id_map.is_empty());
        let overwritten: BTreeSet<Uuid> = plan.overwritten.iter().map(|r| r.uuid).collect();
        assert!(overwritten.contains(&ids.vm) && overwritten.contains(&ids.network));
        assert!(!overwritten.contains(&ids.disk_secret));
        // nothing is added, the virtual_machine keeps its floating ip-address
        for resource_type in [VIRTUAL_MACHINE, NETWORK, IMAGE, FLOATING_IP] {
            assert_eq!(
                usage(&plan, resource_type).after_restore,
                1,
                "{resource_type}"
            );
        }
        assert_eq!(usage(&plan, SECRET).after_restore, 2);
    }

    #[test]
    fn test_other_project_gets_new_uuids() {
        let ids = ids();
        let metadata = backup(&ids, true);
        let mut target = TargetState {
            exists: true,
            quota: Some(quota(10)),
            ..Default::default()
        };
        let plan = calculate_plan(&metadata, &target, &options(false, false, false), "other");
        assert_eq!(plan.id_map.len(), 6);
        assert!(plan.warnings.is_empty());

        // with force the UUIDs are kept, which is shown as warning
        target.exists = false;
        let plan = calculate_plan(&metadata, &target, &options(true, false, false), "other");
        assert!(plan.id_map.is_empty());
        assert_eq!(plan.warnings.len(), 1);
    }

    #[test]
    fn test_images_without_files() {
        let ids = ids();
        let metadata = backup(&ids, false);
        let plan = calculate_plan(
            &metadata,
            &TargetState::default(),
            &options(false, false, false),
            "other",
        );

        // neither the image nor its secret is restored
        assert!(!mapped(&plan).contains(&ids.image));
        assert!(!mapped(&plan).contains(&ids.image_secret));
        assert!(mapped(&plan).contains(&ids.user_secret));
        assert_eq!(plan.warnings.len(), 1);
        assert_eq!(restored_secrets(&metadata).len(), 1);
    }

    #[test]
    fn test_prune() {
        let ids = ids();
        let metadata = backup(&ids, false);
        let mut target = unchanged_target(&ids);

        // additional resources, which were created after the backup
        let (other_vm, other_net, other_fip) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let (other_image, other_key, other_secret) =
            (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        target
            .hanami
            .virtual_machines
            .push(hanami_vm(other_vm, other_net, 10));
        target.hanami.networks.push(network(other_net));
        target
            .hanami
            .floating_ips
            .push(fip(other_fip, Some(other_vm)));
        target
            .hanami
            .network_filters
            .push(filter(ids.vm, FilterDirection::Egress));
        // the filter of the deleted virtual_machine is deleted together with it
        target
            .hanami
            .network_filters
            .push(filter(other_vm, FilterDirection::Egress));
        target.images.push(image(other_image, Uuid::new_v4()));
        target.omamori.public_keys.push(public_key(other_key));
        target.omamori.secrets.push(secret(other_secret, "user"));
        target
            .omamori
            .secrets
            .push(secret(Uuid::new_v4(), SECRET_OWNER_IMAGE));

        let plan = calculate_plan(&metadata, &target, &options(true, true, true), "project");

        let deleted: Vec<(String, Uuid)> = plan
            .deleted
            .iter()
            .map(|r| (r.resource_type.clone(), r.uuid))
            .collect();
        assert_eq!(
            deleted,
            vec![
                (FLOATING_IP.to_string(), other_fip),
                (NETWORK_FILTER.to_string(), ids.vm),
                (VIRTUAL_MACHINE.to_string(), other_vm),
                (NETWORK.to_string(), other_net),
                (IMAGE.to_string(), other_image),
                (PUBLIC_KEY.to_string(), other_key),
                (SECRET.to_string(), other_secret),
            ]
        );
        assert_eq!(
            plan.deleted_of(NETWORK_FILTER).next().unwrap().name,
            "egress"
        );
        // the image of the backup is kept, also if its file is not part of the backup
        assert!(!deleted.contains(&(IMAGE.to_string(), ids.image)));

        assert_eq!(usage(&plan, VIRTUAL_MACHINE).after_restore, 1);
        assert_eq!(usage(&plan, NETWORK).after_restore, 1);
        assert_eq!(usage(&plan, FLOATING_IP).after_restore, 1);
    }

    #[test]
    fn test_quota() {
        let ids = ids();
        let mut metadata = backup(&ids, true);
        let mut target = TargetState {
            exists: true,
            quota: Some(quota(0)),
            ..Default::default()
        };
        let plan = calculate_plan(&metadata, &target, &options(false, false, true), "project");
        let exceeded = plan.exceeded_quota().unwrap();
        assert!(exceeded.contains(VIRTUAL_MACHINE) && exceeded.contains(NETWORK));

        // the quota of the backup is used, if it is imported
        let mut import = options(false, false, true);
        import.import_quota = true;
        assert!(
            calculate_plan(&metadata, &target, &import, "project")
                .exceeded_quota()
                .is_none()
        );
        // without a quota in the backup, the quota of the project is used
        metadata.miko = None;
        target.quota = Some(quota(2));
        assert!(
            calculate_plan(&metadata, &target, &import, "project")
                .exceeded_quota()
                .is_none()
        );
    }

    #[test]
    fn test_virtual_machine_backup() {
        let ids = ids();
        let mut metadata = backup(&ids, false);
        metadata.miko = None;
        metadata.ryokan = None;
        let vm_options = |force| PlanOptions {
            kind: BackupKind::VirtualMachine,
            ..options(force, false, true)
        };
        let target = unchanged_target(&ids);

        // an existing virtual_machine gets a new UUID, its public-key and network are reused
        let plan = calculate_plan(&metadata, &target, &vm_options(false), "project");
        assert_eq!(mapped(&plan), BTreeSet::from([ids.vm]));
        assert!(plan.warnings.is_empty());
        assert_eq!(usage(&plan, VIRTUAL_MACHINE).after_restore, 2);
        assert_eq!(usage(&plan, NETWORK).after_restore, 1);

        // with force its disk is replaced
        let plan = calculate_plan(&metadata, &target, &vm_options(true), "project");
        assert!(plan.id_map.is_empty());
        assert_eq!(plan.overwritten.len(), 1);

        // a missing network is created, unless the dependencies are excluded
        let empty = TargetState {
            exists: true,
            quota: Some(quota(10)),
            ..Default::default()
        };
        let plan = calculate_plan(&metadata, &empty, &vm_options(false), "project");
        assert!(plan.warnings.is_empty());
        assert_eq!(usage(&plan, NETWORK).after_restore, 1);
        let mut without_dependencies = vm_options(false);
        without_dependencies.include_dependencies = false;
        let plan = calculate_plan(&metadata, &empty, &without_dependencies, "project");
        assert_eq!(plan.warnings.len(), 1);

        // into another project the virtual_machine gets a new UUID, also if it doesn't exist there
        let other_project = PlanOptions {
            kind: BackupKind::VirtualMachine,
            ..options(false, false, false)
        };
        let plan = calculate_plan(&metadata, &empty, &other_project, "other");
        assert_eq!(mapped(&plan), BTreeSet::from([ids.vm]));
        assert!(plan.overwritten.is_empty());
    }
}
