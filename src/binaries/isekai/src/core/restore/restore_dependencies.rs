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

use std::collections::BTreeMap;
use uuid::Uuid;

use ainari_api_structs::exports::{HanamiProjectExport, PublicKeyExport, SecretExport};
use ainari_api_structs::imports::{
    HanamiProjectImportReq, OmamoriProjectImportReq, OmamoriProjectImportResp,
};
use ainari_clients::imports::import_project_networks;
use ainari_clients::imports::import_project_secrets;
use ainari_clients::public_key::get_public_key;
use ainari_common::error::AinariError;

use super::Restorer;

impl Restorer<'_> {
    /// Makes sure, that the public-key of a virtual_machine exists in the project.
    ///
    /// A public-key, which still exists, is used as it is. Otherwise it is imported from the
    /// backup, under a new UUID, if its UUID is used by another project.
    ///
    /// # Returns
    ///
    /// The UUID of the public-key within the project.
    pub(super) async fn restore_public_key(
        &mut self,
        old_uuid: &Uuid,
    ) -> Result<Uuid, AinariError> {
        match get_public_key(
            &self.services.endpoints.omamori,
            &self.services.token,
            self.services.key(),
            old_uuid,
            self.services.insecure(),
        )
        .await
        {
            Ok(_) => return Ok(*old_uuid),
            Err(AinariError::NotFound(_)) => {}
            Err(e) => return Err(e),
        }

        let public_key = self
            .restore
            .metadata
            .omamori
            .public_keys
            .iter()
            .find(|public_key| public_key.uuid == *old_uuid)
            .ok_or_else(|| {
                AinariError::InvalidInput(format!(
                    "Public-key '{old_uuid}' of the virtual_machine doesn't exist anymore and is \
                     not part of the backup. Upload it again, before the backup is restored."
                ))
            })?;

        match self
            .import_secrets(Vec::new(), vec![public_key.clone()], false, BTreeMap::new())
            .await
        {
            Ok(_) => Ok(*old_uuid),
            Err(AinariError::Conflict(_)) => {
                let new_uuid = Uuid::new_v4();
                self.import_secrets(
                    Vec::new(),
                    vec![public_key.clone()],
                    false,
                    BTreeMap::from([(*old_uuid, new_uuid)]),
                )
                .await?;
                self.map_uuid("public_key", *old_uuid, new_uuid)?;
                Ok(new_uuid)
            }
            Err(e) => Err(e),
        }
    }

    /// Imports secrets and public-keys into omamori.
    pub(super) async fn import_secrets(
        &self,
        secrets: Vec<SecretExport>,
        public_keys: Vec<PublicKeyExport>,
        force: bool,
        id_map: BTreeMap<Uuid, Uuid>,
    ) -> Result<OmamoriProjectImportResp, AinariError> {
        let request = OmamoriProjectImportReq {
            wrap_key: self.restore.keys.wrap_key_base64(),
            secrets,
            public_keys,
            force,
            id_map,
        };
        import_project_secrets(
            &self.services.endpoints.omamori,
            &self.services.token,
            self.services.key(),
            &self.job.project_id,
            &request,
            self.services.insecure(),
        )
        .await
    }

    /// Makes sure, that the network of a virtual_machine exists in the project.
    ///
    /// A network, which still exists, is used as it is. Otherwise it is created from the backup,
    /// if the dependencies are included, under a new UUID, if its UUID is already used.
    ///
    /// # Returns
    ///
    /// The UUID of the network within the project.
    pub(super) async fn restore_network(
        &mut self,
        target: &HanamiProjectExport,
        old_uuid: &Uuid,
    ) -> Result<Uuid, AinariError> {
        if target
            .networks
            .iter()
            .any(|network| network.uuid == *old_uuid)
        {
            return Ok(*old_uuid);
        }
        if !self.restore.include_dependencies {
            return Err(AinariError::InvalidInput(format!(
                "Network '{old_uuid}' of the virtual_machine doesn't exist in the project. Restore \
                 with 'include_dependencies' to create it from the backup."
            )));
        }
        let network = self
            .restore
            .metadata
            .hanami
            .networks
            .iter()
            .find(|network| network.uuid == *old_uuid)
            .ok_or_else(|| {
                AinariError::InvalidInput(format!(
                    "Network '{old_uuid}' of the virtual_machine is not part of the backup."
                ))
            })?;

        let mut request = HanamiProjectImportReq {
            networks: vec![network.clone()],
            network_filters: Vec::new(),
            floating_ips: Vec::new(),
            force: false,
            id_map: BTreeMap::new(),
        };
        match self.import_networks(&request).await {
            Ok(()) => Ok(*old_uuid),
            Err(AinariError::Conflict(_)) => {
                let new_uuid = Uuid::new_v4();
                request.id_map.insert(*old_uuid, new_uuid);
                self.import_networks(&request).await?;
                self.map_uuid("network", *old_uuid, new_uuid)?;
                Ok(new_uuid)
            }
            Err(e) => Err(e),
        }
    }

    /// Sends an import-request to hanami.
    pub(super) async fn import_networks(
        &self,
        request: &HanamiProjectImportReq,
    ) -> Result<(), AinariError> {
        import_project_networks(
            &self.services.endpoints.hanami,
            &self.services.token,
            self.services.key(),
            &self.job.project_id,
            request,
            self.services.insecure(),
        )
        .await
        .map(|_| ())
    }

    /// Restores the packet-filters and floating ip-addresses of a virtual_machine.
    ///
    /// The floating ip-addresses are allocated again from the pool of the deployment, so they
    /// only keep their address, if it is still free.
    pub(super) async fn restore_network_filters(&self, old_uuid: &Uuid) -> Result<(), AinariError> {
        let hanami = &self.restore.metadata.hanami;
        let request = HanamiProjectImportReq {
            networks: Vec::new(),
            network_filters: hanami
                .network_filters
                .iter()
                .filter(|filter| filter.virtual_machine_uuid == *old_uuid)
                .cloned()
                .collect(),
            floating_ips: hanami
                .floating_ips
                .iter()
                .filter(|floating_ip| floating_ip.virtual_machine_uuid == Some(*old_uuid))
                .cloned()
                .collect(),
            force: self.restore.force,
            id_map: self.id_map.clone(),
        };
        if request.network_filters.is_empty() && request.floating_ips.is_empty() {
            return Ok(());
        }
        self.import_networks(&request).await
    }
}
