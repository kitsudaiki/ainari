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

use std::collections::BTreeSet;
use uuid::Uuid;

use ainari_api_structs::backup_structs::BackupCopyDirection;
use ainari_api_structs::exports::{HanamiProjectExport, OmamoriProjectExport};
use ainari_clients::backup as backup_clients;
use ainari_clients::exports::export_project_images;
use ainari_common::error::AinariError;

use super::BackupJob;
use crate::core::bundle::{RyokanMetadata, SakuraMetadata, image_path};
use crate::core::processing::jobs::Job;
use crate::core::services::Services;

/// Filters the resources of a project down to a single virtual_machine and the resources, which
/// belong to it.
///
/// # Arguments
///
/// * `export` - The resources of the project in hanami
/// * `virtual_machine_uuid` - The virtual_machine
///
/// # Returns
///
/// The resources of the virtual_machine, or `NotFound`, if it is not part of the project.
pub(super) fn filter_resources_of_virtual_machine(
    export: HanamiProjectExport,
    virtual_machine_uuid: &Uuid,
) -> Result<HanamiProjectExport, AinariError> {
    let virtual_machine = export
        .virtual_machines
        .into_iter()
        .find(|virtual_machine| virtual_machine.uuid == *virtual_machine_uuid)
        .ok_or_else(|| {
            AinariError::NotFound(format!(
                "Virtual_machine '{virtual_machine_uuid}' was not found in the project."
            ))
        })?;

    Ok(HanamiProjectExport {
        networks: export
            .networks
            .into_iter()
            .filter(|network| network.uuid == virtual_machine.network_uuid)
            .collect(),
        network_filters: export
            .network_filters
            .into_iter()
            .filter(|filter| filter.virtual_machine_uuid == *virtual_machine_uuid)
            .collect(),
        floating_ips: export
            .floating_ips
            .into_iter()
            .filter(|floating_ip| floating_ip.virtual_machine_uuid == Some(*virtual_machine_uuid))
            .collect(),
        virtual_machines: vec![virtual_machine],
    })
}

/// Filters the secrets and public-keys of a project down to the ones of the saved
/// virtual_machines.
///
/// # Arguments
///
/// * `export` - The secrets and public-keys of the project
/// * `sakura` - The saved virtual_machines
///
/// # Returns
///
/// The secrets of the disks and the public-keys of the virtual_machines.
pub(super) fn filter_secrets_of_virtual_machines(
    export: OmamoriProjectExport,
    sakura: &SakuraMetadata,
) -> OmamoriProjectExport {
    let secrets: BTreeSet<Uuid> = sakura
        .virtual_machines
        .iter()
        .map(|virtual_machine| virtual_machine.disk.secret_uuid)
        .collect();
    let public_keys: BTreeSet<Uuid> = sakura
        .virtual_machines
        .iter()
        .map(|virtual_machine| virtual_machine.config.public_key_uuid)
        .collect();

    OmamoriProjectExport {
        secrets: export
            .secrets
            .into_iter()
            .filter(|secret| secrets.contains(&secret.uuid))
            .collect(),
        public_keys: export
            .public_keys
            .into_iter()
            .filter(|public_key| public_keys.contains(&public_key.uuid))
            .collect(),
    }
}

/// Checks, that the secrets, which are required to decrypt the disks and images of the backup,
/// are part of it. The lock of the project prevents, that they are deleted in the meantime, so
/// this is only a safety-net.
///
/// # Arguments
///
/// * `omamori` - The exported secrets and public-keys
/// * `sakura` - The saved virtual_machines
/// * `ryokan` - The exported images
///
/// # Returns
///
/// Hints about public-keys of virtual_machines, which were deleted, or `InternalError`, if a
/// secret is missing, which makes the backup useless.
pub(super) fn check_references(
    omamori: &OmamoriProjectExport,
    sakura: &SakuraMetadata,
    ryokan: Option<&RyokanMetadata>,
) -> Result<Vec<String>, AinariError> {
    let secrets: BTreeSet<Uuid> = omamori.secrets.iter().map(|secret| secret.uuid).collect();
    let public_keys: BTreeSet<Uuid> = omamori.public_keys.iter().map(|key| key.uuid).collect();
    let mut hints = Vec::new();

    for virtual_machine in &sakura.virtual_machines {
        if !secrets.contains(&virtual_machine.disk.secret_uuid) {
            return Err(AinariError::internal(format!(
                "Secret '{}' of the disk of virtual_machine '{}' is missing in the export of omamori",
                virtual_machine.disk.secret_uuid, virtual_machine.config.uuid
            )));
        }
        // a public-key can be deleted, while the virtual_machine still uses it. The
        // virtual_machine can only be restored into a project, which still has the public-key.
        if !public_keys.contains(&virtual_machine.config.public_key_uuid) {
            hints.push(format!(
                "Public-key '{}' of virtual_machine '{}' doesn't exist anymore and is not part of \
                 the backup.",
                virtual_machine.config.public_key_uuid, virtual_machine.config.uuid
            ));
        }
    }

    for image in ryokan
        .map(|ryokan| ryokan.images.as_slice())
        .unwrap_or_default()
    {
        if !secrets.contains(&image.secret_uuid) {
            return Err(AinariError::internal(format!(
                "Secret '{}' of image '{}' is missing in the export of omamori",
                image.secret_uuid, image.uuid
            )));
        }
    }

    Ok(hints)
}

/// Collects the images of the project and copies their files into the backup, if requested.
pub(super) async fn collect_images(
    job: &Job,
    backup: &BackupJob,
    services: &Services,
) -> Result<RyokanMetadata, AinariError> {
    let export = export_project_images(
        &services.endpoints.ryokan,
        &services.token,
        services.key(),
        &job.project_id,
        services.insecure(),
    )
    .await?;

    if backup.include_images {
        job.step("copy images");
        for image in &export.images {
            backup_clients::copy_backup_file(
                &services.endpoints.ryokan,
                &services.token,
                services.key(),
                &job.backup_uuid,
                BackupCopyDirection::IntoBackup,
                &image_path(&image.uuid),
                &image.uuid,
                services.insecure(),
            )
            .await?;
        }
    }

    Ok(RyokanMetadata {
        images: export.images,
        files_included: backup.include_images,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::bundle::{BackupDisk, BackupVirtualMachine, disk_path};
    use ainari_api_structs::exports::{
        FloatingIpExport, HanamiVirtualMachineExport, ImageExport, NetworkExport,
        NetworkFilterExport, PublicKeyExport, SakuraVirtualMachineExport, SecretExport,
    };
    use ainari_api_structs::network_filter_structs::FilterDirection;
    use std::net::Ipv4Addr;

    fn network(uuid: Uuid) -> NetworkExport {
        NetworkExport {
            uuid,
            name: "network".to_string(),
            subnet: "192.168.100.0/24".to_string(),
            disable_encryption: false,
        }
    }

    fn hanami_vm(uuid: Uuid, network_uuid: Uuid) -> HanamiVirtualMachineExport {
        HanamiVirtualMachineExport {
            uuid,
            name: "virtual-machine".to_string(),
            number_of_cores: 1,
            memory_size: 1024,
            disk_size: 10,
            network_uuid,
            internal_ip: Ipv4Addr::new(192, 168, 100, 2),
            mac_address: "52:54:00:00:00:01".to_string(),
        }
    }

    fn filter(virtual_machine_uuid: Uuid) -> NetworkFilterExport {
        NetworkFilterExport {
            virtual_machine_uuid,
            direction: FilterDirection::Ingress,
            ip_ranges: Vec::new(),
            ports: Vec::new(),
        }
    }

    fn floating_ip(virtual_machine_uuid: Option<Uuid>) -> FloatingIpExport {
        FloatingIpExport {
            uuid: Uuid::new_v4(),
            name: "floating-ip".to_string(),
            floating_ip: Ipv4Addr::new(10, 0, 0, 5),
            virtual_machine_uuid,
        }
    }

    fn backup_vm(uuid: Uuid, public_key_uuid: Uuid, secret_uuid: Uuid) -> BackupVirtualMachine {
        BackupVirtualMachine {
            config: SakuraVirtualMachineExport {
                uuid,
                name: "virtual-machine".to_string(),
                vm_state: "RUNNING".to_string(),
                number_of_cores: 1,
                memory_size: 1 << 30,
                disk_size: 10,
                image_uuid: Uuid::new_v4(),
                public_key_uuid,
                network_uuid: Uuid::new_v4(),
                internal_ip: Ipv4Addr::new(192, 168, 100, 2),
                mac_address: "52:54:00:00:00:01".to_string(),
            },
            disk: BackupDisk {
                path: disk_path(&uuid),
                secret_uuid,
            },
        }
    }

    fn secret(uuid: Uuid) -> SecretExport {
        SecretExport {
            uuid,
            name: "secret".to_string(),
            owned_by: "user".to_string(),
            resource_uuid: None,
            wrapped_payload: "wrapped".to_string(),
        }
    }

    fn public_key(uuid: Uuid) -> PublicKeyExport {
        PublicKeyExport {
            uuid,
            name: "key".to_string(),
            public_key: "ssh-ed25519 AAAA test".to_string(),
        }
    }

    #[test]
    fn test_filter_resources_of_virtual_machine() {
        let (vm, other_vm) = (Uuid::new_v4(), Uuid::new_v4());
        let (net, other_net) = (Uuid::new_v4(), Uuid::new_v4());
        let export = HanamiProjectExport {
            networks: vec![network(net), network(other_net)],
            virtual_machines: vec![hanami_vm(vm, net), hanami_vm(other_vm, other_net)],
            network_filters: vec![filter(vm), filter(other_vm)],
            floating_ips: vec![
                floating_ip(Some(vm)),
                floating_ip(Some(other_vm)),
                floating_ip(None),
            ],
        };

        let filtered = filter_resources_of_virtual_machine(export.clone(), &vm).unwrap();
        assert_eq!(filtered.virtual_machines, vec![hanami_vm(vm, net)]);
        assert_eq!(filtered.networks, vec![network(net)]);
        assert_eq!(filtered.network_filters.len(), 1);
        assert_eq!(filtered.network_filters[0].virtual_machine_uuid, vm);
        assert_eq!(filtered.floating_ips.len(), 1);
        assert_eq!(filtered.floating_ips[0].virtual_machine_uuid, Some(vm));

        assert!(matches!(
            filter_resources_of_virtual_machine(export, &Uuid::new_v4()),
            Err(AinariError::NotFound(_))
        ));
    }

    #[test]
    fn test_filter_secrets_of_virtual_machines() {
        let (vm, key, other_key) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let (disk_secret, other_secret) = (Uuid::new_v4(), Uuid::new_v4());
        let sakura = SakuraMetadata {
            virtual_machines: vec![backup_vm(vm, key, disk_secret)],
        };
        let export = OmamoriProjectExport {
            secrets: vec![secret(disk_secret), secret(other_secret)],
            public_keys: vec![public_key(key), public_key(other_key)],
        };

        let filtered = filter_secrets_of_virtual_machines(export, &sakura);
        assert_eq!(filtered.secrets, vec![secret(disk_secret)]);
        assert_eq!(filtered.public_keys, vec![public_key(key)]);
    }

    #[test]
    fn test_check_references() {
        let (vm, key, disk_secret) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let image_secret = Uuid::new_v4();
        let sakura = SakuraMetadata {
            virtual_machines: vec![backup_vm(vm, key, disk_secret)],
        };
        let ryokan = RyokanMetadata {
            images: vec![ImageExport {
                uuid: Uuid::new_v4(),
                name: "image".to_string(),
                file_type: "qcow2".to_string(),
                file_size: 1,
                is_snapshot: false,
                secret_uuid: image_secret,
            }],
            files_included: true,
        };
        let complete = OmamoriProjectExport {
            secrets: vec![secret(disk_secret), secret(image_secret)],
            public_keys: vec![public_key(key)],
        };
        assert!(
            check_references(&complete, &sakura, Some(&ryokan))
                .unwrap()
                .is_empty()
        );

        // a deleted public-key is only a hint
        let without_key = OmamoriProjectExport {
            public_keys: Vec::new(),
            ..complete.clone()
        };
        assert_eq!(
            check_references(&without_key, &sakura, Some(&ryokan))
                .unwrap()
                .len(),
            1
        );

        // missing secrets make the backup useless
        let without_disk_secret = OmamoriProjectExport {
            secrets: vec![secret(image_secret)],
            ..complete.clone()
        };
        assert!(check_references(&without_disk_secret, &sakura, None).is_err());
        let without_image_secret = OmamoriProjectExport {
            secrets: vec![secret(disk_secret)],
            ..complete
        };
        assert!(check_references(&without_image_secret, &sakura, None).is_ok());
        assert!(check_references(&without_image_secret, &sakura, Some(&ryokan)).is_err());
    }
}
