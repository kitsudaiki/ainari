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

use ainari_api_structs::backup_structs::PlannedResource;
use ainari_api_structs::network_filter_structs::FilterDirection;
use ainari_clients::floating_ip::delete_floating_ip_in_hanami;
use ainari_clients::image::delete_image;
use ainari_clients::network::delete_network;
use ainari_clients::network_filter::delete_network_filter;
use ainari_clients::public_key::delete_public_key;
use ainari_clients::secret::delete_user_secret;
use ainari_common::error::AinariError;

use super::Restorer;
use super::plan::*;

impl Restorer<'_> {
    /// Deletes the resources of the project, which are not part of the backup, like they are listed
    /// in the plan.
    ///
    /// They are deleted in the reverse order of their dependencies, so a resource is never deleted,
    /// while another resource still uses it. Resources, which are already deleted, are skipped, so
    /// the prune can be repeated after a failure.
    pub(super) async fn prune(&self) -> Result<(), AinariError> {
        for resource_type in PRUNE_ORDER {
            for resource in self.restore.plan.deleted_of(resource_type) {
                self.job.step(&format!(
                    "delete {resource_type} {} ({})",
                    resource.uuid, resource.name
                ));
                match self.delete_resource(resource).await {
                    Ok(()) | Err(AinariError::NotFound(_)) => {}
                    Err(e) => return Err(e),
                }
            }
        }
        Ok(())
    }

    /// Deletes a single resource of the project.
    async fn delete_resource(&self, resource: &PlannedResource) -> Result<(), AinariError> {
        let services = self.services;
        let (token, key, insecure) = (&services.token, services.key(), services.insecure());
        let uuid = &resource.uuid;

        match resource.resource_type.as_str() {
            FLOATING_IP => {
                delete_floating_ip_in_hanami(&services.endpoints.hanami, token, key, uuid, insecure)
                    .await
            }
            NETWORK_FILTER => {
                let direction: FilterDirection = resource.name.parse().map_err(|_| {
                    AinariError::internal(format!(
                        "Invalid direction '{}' of a packet-filter",
                        resource.name
                    ))
                })?;
                delete_network_filter(
                    &services.endpoints.hanami,
                    token,
                    key,
                    uuid,
                    direction,
                    insecure,
                )
                .await
            }
            VIRTUAL_MACHINE => self.delete_virtual_machine(uuid).await,
            NETWORK => delete_network(&services.endpoints.hanami, token, key, uuid, insecure).await,
            // the secret of the image is deleted together with the image
            IMAGE => delete_image(&services.endpoints.ryokan, token, key, uuid, insecure).await,
            PUBLIC_KEY => {
                delete_public_key(&services.endpoints.omamori, token, key, uuid, insecure).await
            }
            SECRET => {
                delete_user_secret(&services.endpoints.omamori, token, key, uuid, insecure).await
            }
            other => Err(AinariError::internal(format!(
                "Unknown kind of resource '{other}' to delete"
            ))),
        }
    }
}
