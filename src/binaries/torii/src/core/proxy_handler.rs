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

use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::config;
use crate::config::ProxyConf;
use crate::core::proxy::{Proxy, start_listener};
use crate::database::proxy_table;

use ainari_api_structs::user_context::UserContext;
use ainari_common::enums::ProjectRole;
use ainari_common::error::AinariError;

lazy_static::lazy_static! {
    /// Global singleton that holds a thread-safe `ProxyHandler` virtual_machine.
    /// This allows for centralized management of all proxy virtual_machines in the application.
    pub static ref PROXY_HANDLER: RwLock<ProxyHandler> = RwLock::new(init_proxy_handler());
}

// ==================================================================================================

/// A handler for managing multiple proxy virtual_machines.
/// This struct maintains a collection of proxies identified by their UUIDs.
pub struct ProxyHandler {
    /// A map of proxy UUIDs to their corresponding `Proxy` virtual_machines.
    pub proxys: HashMap<Uuid, Proxy>,
}

// ==================================================================================================

/// Initializes a new, empty `ProxyHandler` virtual_machine.
///
/// # Returns
/// A new `ProxyHandler` with an empty proxy map.
pub fn init_proxy_handler() -> ProxyHandler {
    ProxyHandler {
        proxys: HashMap::new(),
    }
}

impl ProxyHandler {
    /// Adds a new proxy to the handler.
    ///
    /// # Arguments
    ///
    /// * `uuid` - The unique identifier for the proxy.
    /// * `target_addr` - The address the proxy will forward traffic to.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the proxy was added successfully.
    /// * `Err(AinariError::InvalidInput)` if a proxy with the given UUID already exists.
    pub fn add_proxy(&mut self, uuid: &Uuid, target_addr: &str) -> Result<(), AinariError> {
        // Check if a proxy with this UUID already exists
        if self.proxys.contains_key(uuid) {
            let msg = format!("Proxy with uuid '{uuid}' already exist.");
            return Err(AinariError::InvalidInput(msg));
        }

        self.proxys.insert(*uuid, Proxy::new(target_addr));

        Ok(())
    }

    /// Removes a proxy from the handler and closes its connections.
    ///
    /// # Arguments
    ///
    /// * `proxy_uuid` - The UUID of the proxy to remove.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the proxy was removed successfully.
    /// * `Err(AinariError::InvalidInput)` if no proxy with the given UUID was found.
    pub fn delete_proxy(&mut self, proxy_uuid: &Uuid) -> Result<(), AinariError> {
        // dropping the proxy closes its connections
        if self.proxys.remove(proxy_uuid).is_none() {
            let msg = format!("Proxy with uuid '{proxy_uuid}' not found.");
            return Err(AinariError::InvalidInput(msg));
        }

        Ok(())
    }

    /// Points a proxy at another target, while it keeps its address.
    ///
    /// Open connections to the old target are closed.
    ///
    /// # Arguments
    ///
    /// * `uuid` - The unique identifier of the proxy.
    /// * `target_addr` - The new address the proxy forwards traffic to.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the proxy forwards to the new target.
    /// * `Err(AinariError::InvalidInput)` if no proxy with the given UUID was found.
    pub fn retarget_proxy(&mut self, uuid: &Uuid, target_addr: &str) -> Result<(), AinariError> {
        let Some(proxy) = self.proxys.get_mut(uuid) else {
            let msg = format!("Proxy with uuid '{uuid}' not found.");
            return Err(AinariError::InvalidInput(msg));
        };

        // the old proxy is dropped with that and closes its connections
        *proxy = Proxy::new(target_addr);

        Ok(())
    }

    /// Populates the proxy handler with proxies from the database.
    ///
    /// This method is typically used during application startup to initialize
    /// all proxies that were persisted to the database.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if all proxies were loaded successfully.
    /// * `Err(AinariError::InternalError)` if there was an issue accessing the database or parsing proxy data.
    pub fn fill_proxy_handler(&mut self) -> Result<(), AinariError> {
        // Create a dummy context with admin privileges to read all proxies
        let dummy_context = UserContext {
            token: "".to_string(),
            user_id: "dummy".to_string(),
            project_id: "dummy".to_string(),
            is_admin: true.to_string(),
            project_role: ProjectRole::Admin.to_string(),
        };

        // Retrieve the list of proxies from the database
        let proxys = match proxy_table::list_proxys(&dummy_context) {
            Ok(proxys) => proxys,
            Err(e) => {
                let msg = format!("Failed to get list of proxys form database: '{e}'");
                return Err(AinariError::internal(msg));
            }
        };

        // Add each proxy to the handler
        for proxy in proxys {
            self.add_proxy(&proxy.uuid, &proxy.target_address)?;
        }

        Ok(())
    }
}

/// Returns the configuration of the proxies.
///
/// # Returns
///
/// * `Ok(&ProxyConf)` if the gateway has proxies.
/// * `Err(AinariError::InternalError)` if the config has no `proxy`-section, like the one of a
///   gateway in front of a sakura-host.
pub fn proxy_conf() -> Result<&'static ProxyConf, AinariError> {
    config::CONFIG.proxy.as_ref().ok_or_else(|| {
        AinariError::internal("This gateway has no proxies, its config has no 'proxy'-section.")
    })
}

/// Starts the listener of all proxies, if the gateway has proxies.
///
/// # Returns
///
/// * `Ok(())` if the listener runs or the gateway has no proxies.
/// * `Err(AinariError::InternalError)` if the listener could not be started.
pub async fn start_proxy_listener() -> Result<(), AinariError> {
    let Some(conf) = config::CONFIG.proxy.as_ref() else {
        log::info!("No 'proxy'-section in the config, so this gateway has no proxies");
        return Ok(());
    };

    let listen_ip = conf.listen_ip(&config::CONFIG.api);
    let addr_str = format!("{listen_ip}:{}", conf.port);
    let listen_addr = SocketAddr::from_str(&addr_str).map_err(|e| {
        AinariError::internal(format!(
            "Failed to convert address '{addr_str}' into a SocketAddr with error: {e}"
        ))
    })?;

    start_listener(conf, listen_addr).await.map_err(|e| {
        AinariError::internal(format!(
            "Failed to start the listener of the proxies on '{listen_addr}': {e}"
        ))
    })
}
