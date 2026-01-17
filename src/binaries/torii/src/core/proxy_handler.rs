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
use crate::core::proxy::Proxy;
use crate::database::proxy_table;

use ainari_api_structs::user_context::UserContext;
use ainari_common::error::AinariError;

lazy_static::lazy_static! {
    /// Global singleton that holds a thread-safe `ProxyHandler` instance.
    /// This allows for centralized management of all proxy instances in the application.
    pub static ref PROXY_HANDLER: RwLock<ProxyHandler> = RwLock::new(init_proxy_handler());
}

// ==================================================================================================

/// A handler for managing multiple proxy instances.
/// This struct maintains a collection of proxies identified by their UUIDs.
pub struct ProxyHandler {
    /// A map of proxy UUIDs to their corresponding `Proxy` instances.
    pub proxys: HashMap<Uuid, Proxy>,
}

// ==================================================================================================

/// Initializes a new, empty `ProxyHandler` instance.
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
    /// * `port` - The port number the proxy will listen on.
    /// * `target_addr` - The address the proxy will forward traffic to.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the proxy was added successfully.
    /// * `Err(AinariError::InvalidInput)` if a proxy with the given UUID already exists.
    /// * `Err(AinariError::InternalError)` if there was an issue converting the address.
    pub async fn add_proxy(
        &mut self,
        uuid: &Uuid,
        port: u16,
        target_addr: &str,
    ) -> Result<(), AinariError> {
        // Check if a proxy with this UUID already exists
        if self.proxys.contains_key(uuid) {
            let msg = format!("Proxy with uuid '{uuid}' already exist.");
            return Err(AinariError::InvalidInput(msg));
        }

        // Construct the public address string and convert it to a SocketAddr
        let addr_str = format!("{}:{}", config::CONFIG.api.public_ip, port);
        let public_addr = match SocketAddr::from_str(&addr_str) {
            Ok(public_addr) => public_addr,
            Err(e) => {
                let msg = format!(
                    "Failed to convert address '{addr_str}' into a SocketAddr with error: {e}"
                );
                return Err(AinariError::InternalError(msg));
            }
        };

        // Create a new proxy and add it to the map
        let new_proxy = Proxy::new(&public_addr, target_addr).await;
        self.proxys.insert(*uuid, new_proxy);

        Ok(())
    }

    /// Removes a proxy from the handler and stops it.
    ///
    /// # Arguments
    ///
    /// * `proxy_uuid` - The UUID of the proxy to remove.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the proxy was removed successfully.
    /// * `Err(AinariError::InvalidInput)` if no proxy with the given UUID was found.
    pub async fn delete_proxy(&mut self, proxy_uuid: &Uuid) -> Result<(), AinariError> {
        // Check if a proxy with this UUID exists
        if !self.proxys.contains_key(proxy_uuid) {
            let msg = format!("Proxy with uuid '{proxy_uuid}' not found.");
            return Err(AinariError::InvalidInput(msg));
        }

        // Remove the proxy and stop it
        if let Some(mut proxy) = self.proxys.remove(proxy_uuid) {
            proxy.stop();
        }

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
    pub async fn fill_proxy_handler(&mut self) -> Result<(), AinariError> {
        // Create a dummy context with admin privileges to read all proxies
        let dummy_context = UserContext {
            token: "".to_string(),
            user_id: "dummy".to_string(),
            project_id: "dummy".to_string(),
            is_admin: true.to_string(),
            is_project_admin: true.to_string(),
        };

        // Retrieve the list of proxies from the database
        let proxys = match proxy_table::list_proxys(&dummy_context) {
            Ok(proxys) => proxys,
            Err(e) => {
                let msg = format!("Failed to get list of proxys form database: '{e}'");
                return Err(AinariError::InternalError(msg));
            }
        };

        // Add each proxy to the handler
        for proxy in proxys {
            let uuid = Uuid::parse_str(&proxy.uuid).map_err(|e| {
                AinariError::InternalError(format!(
                    "Failed to convert proxy-uuid with error: '{e}'"
                ))
            })?;

            let port = proxy.port as u16;
            self.add_proxy(&uuid, port, &proxy.target_address).await?;
        }

        Ok(())
    }
}
