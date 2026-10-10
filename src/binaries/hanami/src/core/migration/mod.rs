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

//! Cold migration of a virtual_machine from one sakura-host to another, which is started by an
//! admin.
//!
//! The migration runs in the background and moves the virtual_machine in these steps:
//!
//! 1. The source host prepares the virtual_machine: it is shut down and frozen.
//! 2. The network of the virtual_machine is prepared on the target host and all gateways route
//!    the virtual_machine to the target host (see `network`).
//! 3. The target host imports the virtual_machine: it pulls its files directly from the source
//!    host and boots it, if it was running before.
//! 4. hanami takes the virtual_machine over to the target host: its host, its address and its
//!    proxy are changed.
//! 5. The source host and its gateway forget the virtual_machine and its resources are released.
//!
//! A failure before step 4 rolls the migration back, so the virtual_machine runs on the source
//! host again. After step 4 the virtual_machine runs on the target host, so a failure only leaves
//! something behind on the source host, which is logged for a manual cleanup.
//!
//! The token of the admin is renewed during the migration (see `session`), because the transfer
//! of a large disk can take longer than its lifetime.

pub mod network;
pub mod session;

use std::collections::HashSet;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use uuid::Uuid;

use crate::config;
use crate::database::address_table::{self, AddressEntry};
use crate::database::host_table::{self, HostEntry, HostResources};
use crate::database::meta_virtual_machine_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::migration_structs::MigrationImportReq;
use ainari_api_structs::task_structs::{TaskResp, TaskState};
use ainari_clients::endpoints::get_endpoints;
use ainari_clients::proxy as proxy_clients;
use ainari_clients::task::get_task;
use ainari_clients::virtual_machine as virtual_machine_clients;
use ainari_clients::virtual_machine_migration as migration_clients;

use network::VmNetwork;
use session::Session;

/// Interval, in which the tasks of the sakura-hosts are checked
const TASK_POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Maximum time of a single task of a sakura-host, before the migration is given up
const MAX_TASK_DURATION: Duration = Duration::from_secs(24 * 60 * 60);

/// Number of failed checks of a task in a row, before the migration is given up. Single checks
/// can fail, for example while the sakura-host restarts.
const MAX_FAILED_TASK_CHECKS: u32 = 30;

/// Virtual_machines, which are migrated right now
static MIGRATING: Mutex<Option<HashSet<Uuid>>> = Mutex::new(None);

/// Marks a virtual_machine as migrated, as long as it lives.
///
/// Only one migration of a virtual_machine can run at the same time, and the virtual_machine
/// must not be deleted or get new packet-filters during its migration.
pub struct MigrationGuard {
    virtual_machine_uuid: Uuid,
}

impl MigrationGuard {
    /// Marks a virtual_machine as migrated.
    ///
    /// # Returns
    /// The guard, or `None` if the virtual_machine is already migrated
    pub fn acquire(virtual_machine_uuid: Uuid) -> Option<Self> {
        let mut migrating = MIGRATING.lock().expect("mutex poisoned");
        // `then` and not `then_some`, because a guard, which is created and dropped right away,
        // would remove the mark of the running migration
        migrating
            .get_or_insert_with(HashSet::new)
            .insert(virtual_machine_uuid)
            .then(|| Self {
                virtual_machine_uuid,
            })
    }
}

impl Drop for MigrationGuard {
    fn drop(&mut self) {
        if let Some(migrating) = MIGRATING.lock().expect("mutex poisoned").as_mut() {
            migrating.remove(&self.virtual_machine_uuid);
        }
    }
}

/// Checks, if a virtual_machine is migrated right now.
pub fn is_migrating(virtual_machine_uuid: &Uuid) -> bool {
    MIGRATING
        .lock()
        .expect("mutex poisoned")
        .as_ref()
        .is_some_and(|migrating| migrating.contains(virtual_machine_uuid))
}

/// A migration, which was accepted and whose resources are allocated on the target host
pub struct Migration {
    pub virtual_machine_uuid: Uuid,
    /// Proxy of the virtual_machine on the torii at the edge
    pub proxy_uuid: Uuid,
    pub source: HostEntry,
    pub target: HostEntry,
    /// Address of the virtual_machine, which still names the source host
    pub address: AddressEntry,
    /// Resources of the virtual_machine, which are allocated on the target host already
    pub resources: HostResources,
}

/// Runs a migration in the background.
///
/// # Arguments
/// * `migration` - The migration
/// * `session` - Session of the admin, who started the migration
/// * `guard` - Marks the virtual_machine as migrated until the migration ended
pub fn spawn_migration(migration: Migration, session: Session, guard: MigrationGuard) {
    thread::spawn(move || {
        let _guard = guard;
        let mut session = session;
        let uuid = migration.virtual_machine_uuid;
        log::info!(
            "Start migration of virtual_machine '{uuid}' from host '{}' to host '{}'",
            migration.source.name,
            migration.target.name
        );

        // the http-client is bound to an actix-runtime, so the thread gets its own one
        let system = actix_rt::System::new();
        match system.block_on(run(&migration, &mut session)) {
            Ok(()) => log::info!(
                "Migrated virtual_machine '{uuid}' to host '{}'",
                migration.target.name
            ),
            Err(e) => log::error!("Migration of virtual_machine '{uuid}' failed: {e:?}"),
        }
    });
}

/// Progress of a migration, which decides what a rollback has to undo
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Progress {
    /// Nothing was changed yet, besides the allocated resources on the target host
    Started,
    /// The preparation on the source host was requested
    PrepareRequested,
    /// The network was touched on the target host and the other gateways
    NetworkMoved,
    /// The import on the target host was requested
    ImportRequested,
}

/// Runs all steps of a migration and rolls it back, if one of the steps before the takeover
/// failed.
async fn run(migration: &Migration, session: &mut Session) -> Result<(), ErrorResponse> {
    let endpoints = get_endpoints(&config::CONFIG.miko, config::CONFIG.skip_tls_verification)
        .await
        .map_err(map_ainari_error_to_api_response);
    let network = match endpoints {
        Ok(endpoints) => {
            VmNetwork::new(
                migration.virtual_machine_uuid,
                migration.address.clone(),
                endpoints,
            )
            .await
        }
        Err(e) => Err(e),
    };
    let network = match network {
        Ok(network) => network,
        Err(e) => {
            release_resources(&migration.target, &migration.resources);
            return Err(e);
        }
    };

    let mut progress = Progress::Started;
    let mut boot = false;
    if let Err(e) =
        move_virtual_machine(migration, &network, session, &mut progress, &mut boot).await
    {
        log::warn!(
            "Migration of virtual_machine '{}' failed, roll it back: {e:?}",
            migration.virtual_machine_uuid
        );
        rollback(migration, &network, session, progress, boot).await;
        return Err(e);
    }

    take_over(migration, session).await;
    clean_up_source(migration, &network, session).await;
    Ok(())
}

/// Moves the virtual_machine and its network to the target host (steps 1 to 3).
///
/// # Arguments
/// * `progress` - Is updated with every step, which changes something
/// * `boot` - Is set, if the virtual_machine was running before the migration
async fn move_virtual_machine(
    migration: &Migration,
    network: &VmNetwork,
    session: &mut Session,
    progress: &mut Progress,
    boot: &mut bool,
) -> Result<(), ErrorResponse> {
    let uuid = &migration.virtual_machine_uuid;
    let skip_tls = config::CONFIG.skip_tls_verification;

    // a virtual_machine, which was running before, is running afterwards again
    let virtual_machine = virtual_machine_clients::get_virtual_machine(
        &migration.source.address,
        &session.context().await?.token,
        &config::INTERNAL_API_KEY,
        uuid,
        skip_tls,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;
    *boot = virtual_machine.vm_state == "RUNNING";

    *progress = Progress::PrepareRequested;
    let task = migration_clients::prepare_migration(
        &migration.source.address,
        &session.context().await?.token,
        &config::INTERNAL_API_KEY,
        uuid,
        skip_tls,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;
    wait_for_task(session, &migration.source, &task).await?;
    log::info!("Virtual_machine '{uuid}' is prepared on the source host");

    // the virtual_machine is shut down now, so its network can be moved, before it is started
    // on the target host
    *progress = Progress::NetworkMoved;
    network
        .attach_to_host(&migration.target, session.context().await?)
        .await?;
    network
        .point_to_host(&migration.target, session.context().await?)
        .await?;
    log::info!("Network of virtual_machine '{uuid}' is moved to the target host");

    *progress = Progress::ImportRequested;
    let req = MigrationImportReq {
        virtual_machine_uuid: *uuid,
        source_address: migration.source.address.clone(),
        boot: *boot,
    };
    let task = migration_clients::import_virtual_machine(
        &migration.target.address,
        &session.context().await?.token,
        &config::INTERNAL_API_KEY,
        &req,
        skip_tls,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;
    wait_for_task(session, &migration.target, &task).await?;
    log::info!("Virtual_machine '{uuid}' is imported on the target host");

    Ok(())
}

/// Moves the virtual_machine back to the source host after a failed migration.
///
/// Every step is tried, also if another one failed, so as much as possible is restored. A failed
/// step is logged for a manual cleanup. The only exception is the removal from the target host:
/// if it is not confirmed, nothing else is rolled back, so the virtual_machine never runs twice.
async fn rollback(
    migration: &Migration,
    network: &VmNetwork,
    session: &mut Session,
    progress: Progress,
    boot: bool,
) {
    let uuid = &migration.virtual_machine_uuid;
    let skip_tls = config::CONFIG.skip_tls_verification;

    if progress >= Progress::ImportRequested {
        let result = match session.context().await {
            Ok(context) => migration_clients::remove_migrated_virtual_machine(
                &migration.target.address,
                &context.token,
                &config::INTERNAL_API_KEY,
                uuid,
                skip_tls,
            )
            .await
            .map_err(map_ainari_error_to_api_response),
            Err(e) => Err(e),
        };
        let result = match result {
            Ok(task) => wait_for_task(session, &migration.target, &task).await,
            // the import was rejected, before the virtual_machine was created on the target
            Err(ErrorResponse::NotFound(_)) => Ok(()),
            Err(e) => Err(e),
        };
        // The removal runs after the import on the target host, so it also covers an import,
        // whose end was not seen, and refuses to remove a virtual_machine, which was imported
        // successfully. Without the confirmation, that it is gone, the virtual_machine could run
        // on both hosts with the same address, so the rollback stops here.
        if let Err(e) = result {
            log::error!(
                "Rollback of migration of virtual_machine '{uuid}' stopped, because it is not \
                 sure, that the virtual_machine is gone from host '{}'. It is left frozen on \
                 host '{}' and its resources stay allocated on both hosts. This has to be \
                 resolved by hand: {e:?}",
                migration.target.name,
                migration.source.name
            );
            return;
        }
        log_rollback_step(
            uuid,
            "remove the virtual_machine from the target host",
            Ok(()),
        );
    }

    if progress >= Progress::NetworkMoved {
        let result = async {
            network
                .attach_to_host(&migration.source, session.context().await?)
                .await?;
            network
                .point_to_host(&migration.source, session.context().await?)
                .await?;
            network
                .release_host(&migration.target.address, session.context().await?)
                .await
        }
        .await;
        log_rollback_step(uuid, "move the network back to the source host", result);
    }

    if progress >= Progress::PrepareRequested {
        let result = match session.context().await {
            Ok(context) => migration_clients::cancel_migration(
                &migration.source.address,
                &context.token,
                &config::INTERNAL_API_KEY,
                uuid,
                boot,
                skip_tls,
            )
            .await
            .map_err(map_ainari_error_to_api_response),
            Err(e) => Err(e),
        };
        let result = match result {
            Ok(task) => wait_for_task(session, &migration.source, &task).await,
            Err(e) => Err(e),
        };
        log_rollback_step(
            uuid,
            "start the virtual_machine on the source host again",
            result,
        );
    }

    release_resources(&migration.target, &migration.resources);
}

/// Logs the result of a step of a rollback.
fn log_rollback_step(uuid: &Uuid, step: &str, result: Result<(), ErrorResponse>) {
    match result {
        Ok(()) => log::info!("Rollback of migration of virtual_machine '{uuid}': {step}"),
        Err(e) => log::error!(
            "Rollback of migration of virtual_machine '{uuid}' failed to {step}, this has to be \
             done by hand: {e:?}"
        ),
    }
}

/// Takes the virtual_machine over to the target host in hanami (step 4).
///
/// The virtual_machine runs on the target host already, so a failure is only logged and doesn't
/// roll the migration back.
async fn take_over(migration: &Migration, session: &mut Session) {
    let uuid = &migration.virtual_machine_uuid;
    let target = &migration.target;

    let context = match session.context().await {
        Ok(context) => context,
        Err(e) => {
            log::error!(
                "Failed to take virtual_machine '{uuid}' over to host '{}', this has to be done \
                 by hand: {e:?}",
                target.name
            );
            return;
        }
    };

    if meta_virtual_machine_table::set_host_of_meta_virtual_machine(uuid, &target.uuid, context)
        .is_err()
    {
        log::error!(
            "Failed to set host '{}' of virtual_machine '{uuid}' in database, this has to be \
             done by hand.",
            target.uuid
        );
    }
    if address_table::set_host_of_address(&migration.address.uuid, &target.address).is_err() {
        log::error!(
            "Failed to set host '{}' of address '{}' in database, this has to be done by hand.",
            target.address,
            migration.address.uuid
        );
    }

    // the proxy keeps its port, so the virtual_machine stays reachable under the same port
    let result =
        match get_endpoints(&config::CONFIG.miko, config::CONFIG.skip_tls_verification).await {
            Ok(endpoints) => {
                proxy_clients::update_proxy(
                    &endpoints.torii,
                    &context.token,
                    &config::INTERNAL_API_KEY,
                    &migration.proxy_uuid,
                    target.proxy_target_address(),
                    config::CONFIG.skip_tls_verification,
                )
                .await
            }
            Err(e) => Err(e),
        };
    if let Err(e) = result {
        log::error!(
            "Failed to point proxy '{}' of virtual_machine '{uuid}' to host '{}', this has to be \
             done by hand: {e}",
            migration.proxy_uuid,
            target.name
        );
    }
}

/// Removes the virtual_machine from the source host and its gateway and releases its resources
/// there (step 5).
///
/// The virtual_machine runs on the target host already, so a failure is only logged. The
/// resources are only released, if the virtual_machine is really gone from the source host.
async fn clean_up_source(migration: &Migration, network: &VmNetwork, session: &mut Session) {
    let uuid = &migration.virtual_machine_uuid;
    let source = &migration.source;

    let result = match session.context().await {
        Ok(context) => network.release_host(&source.address, context).await,
        Err(e) => Err(e),
    };
    if let Err(e) = result {
        log::error!(
            "Failed to remove virtual_machine '{uuid}' from the gateway of host '{}', this has \
             to be done by hand: {e:?}",
            source.name
        );
    }

    let result = match session.context().await {
        Ok(context) => migration_clients::remove_migrated_virtual_machine(
            &source.address,
            &context.token,
            &config::INTERNAL_API_KEY,
            uuid,
            config::CONFIG.skip_tls_verification,
        )
        .await
        .map_err(map_ainari_error_to_api_response),
        Err(e) => Err(e),
    };
    let result = match result {
        Ok(task) => wait_for_task(session, source, &task).await,
        Err(e) => Err(e),
    };
    match result {
        Ok(()) => release_resources(source, &migration.resources),
        Err(e) => log::error!(
            "Failed to remove virtual_machine '{uuid}' from host '{}', so its resources there are \
             not released. This has to be done by hand: {e:?}",
            source.name
        ),
    }
}

/// Releases the resources of the virtual_machine on a host.
fn release_resources(host: &HostEntry, resources: &HostResources) {
    if host_table::release_host_resources(&host.uuid, resources).is_err() {
        log::error!(
            "Failed to release resources of a migrated virtual_machine on host '{}'.",
            host.uuid
        );
    }
}

/// Waits until a task of a sakura-host is finished.
///
/// # Arguments
/// * `session` - Session of the migration
/// * `host` - The sakura-host, which processes the task
/// * `task` - The task
///
/// # Returns
/// * `Ok(())` if the task finished successfully
/// * `Err(ErrorResponse)` if the task failed, didn't finish in time or can't be checked anymore
async fn wait_for_task(
    session: &mut Session,
    host: &HostEntry,
    task: &TaskResp,
) -> Result<(), ErrorResponse> {
    let deadline = Instant::now() + MAX_TASK_DURATION;
    let mut failed_checks = 0;

    loop {
        actix_rt::time::sleep(TASK_POLL_INTERVAL).await;

        let token = session.context().await?.token.clone();
        let current = match get_task(
            &host.address,
            &token,
            &task.uuid,
            config::CONFIG.skip_tls_verification,
        )
        .await
        {
            Ok(current) => {
                failed_checks = 0;
                current
            }
            Err(e) => {
                failed_checks += 1;
                if failed_checks >= MAX_FAILED_TASK_CHECKS {
                    return Err(map_ainari_error_to_api_response(e));
                }
                log::warn!(
                    "Failed to check task '{}' on host '{}': {e}",
                    task.uuid,
                    host.name
                );
                continue;
            }
        };

        match current.state {
            TaskState::Finished => return Ok(()),
            TaskState::Error | TaskState::Aborted => {
                return Err(ErrorResponse::InternalError(format!(
                    "{} on host '{}' ended with state {}: {}",
                    task.description,
                    host.name,
                    current.state,
                    current.messages.join("; ")
                )));
            }
            TaskState::Created | TaskState::Queued | TaskState::Active => {}
        }

        if Instant::now() >= deadline {
            return Err(ErrorResponse::InternalError(format!(
                "{} on host '{}' didn't finish within {}h",
                task.description,
                host.name,
                MAX_TASK_DURATION.as_secs() / 3600
            )));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_guard() {
        let uuid = Uuid::new_v4();
        assert!(!is_migrating(&uuid));

        let guard = MigrationGuard::acquire(uuid).expect("first migration is accepted");
        assert!(is_migrating(&uuid));
        // a second migration of the same virtual_machine is rejected ...
        assert!(MigrationGuard::acquire(uuid).is_none());
        // ... but not of another one
        let other = MigrationGuard::acquire(Uuid::new_v4()).expect("other migration is accepted");

        drop(guard);
        assert!(!is_migrating(&uuid));
        assert!(MigrationGuard::acquire(uuid).is_some());
        drop(other);
    }

    #[test]
    fn test_rollback_order_of_progress() {
        // a later step implies the earlier ones, which the rollback relies on
        assert!(Progress::Started < Progress::PrepareRequested);
        assert!(Progress::PrepareRequested < Progress::NetworkMoved);
        assert!(Progress::NetworkMoved < Progress::ImportRequested);
    }
}
