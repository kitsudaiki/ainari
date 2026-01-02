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
    pub static ref PROXY_HANDLER: RwLock<ProxyHandler> = RwLock::new(init_proxy_handler());
}

// ==================================================================================================

pub struct ProxyHandler {
    pub proxys: HashMap<Uuid, Proxy>,
}

// ==================================================================================================

pub fn init_proxy_handler() -> ProxyHandler {
    ProxyHandler {
        proxys: HashMap::new(),
    }
}

impl ProxyHandler {
    pub async fn add_proxy(
        &mut self,
        uuid: &Uuid,
        port: u16,
        target_addr: &str,
    ) -> Result<(), AinariError> {
        if self.proxys.contains_key(uuid) {
            let msg = format!("Proxy with uuid '{uuid}' already exist.");
            return Err(AinariError::InvalidInput(msg));
        }

        let addr_str = format!("{}:{}", config::CONFIG.api.public_ip, port);
        let public_addr = match SocketAddr::from_str(&addr_str) {
            Ok(public_addr) => public_addr,
            Err(e) => {
                let msg = format!(
                    "Failed to convert address '{addr_str}' into a SocketAddr with error: {e}"
                );
                return Err(AinariError::Error(msg));
            }
        };
        let new_proxy = Proxy::new(&public_addr, target_addr).await;
        self.proxys.insert(*uuid, new_proxy);

        Ok(())
    }

    pub async fn delete_proxy(&mut self, proxy_uuid: &Uuid) -> Result<(), AinariError> {
        if !self.proxys.contains_key(proxy_uuid) {
            let msg = format!("Proxy with uuid '{proxy_uuid}' not found.");
            return Err(AinariError::InvalidInput(msg));
        }

        if let Some(mut proxy) = self.proxys.remove(proxy_uuid) {
            proxy.stop();
        }

        Ok(())
    }

    pub async fn fill_proxy_handler(&mut self) -> Result<(), AinariError> {
        // need an internal dummy-conext, which is allowed to read all proxies to initialize them after restart
        let dummy_context = UserContext {
            token: "".to_string(),
            user_id: "dummy".to_string(),
            project_id: "dummy".to_string(),
            is_admin: true.to_string(),
            is_project_admin: true.to_string(),
        };
        let proxys = match proxy_table::list_proxys(&dummy_context) {
            Ok(proxys) => proxys,
            Err(e) => {
                let msg = format!("Failed to get list of proxys form database: '{e}'");
                return Err(AinariError::Error(msg));
            }
        };

        for proxy in proxys {
            let uuid = Uuid::parse_str(&proxy.uuid).map_err(|e| {
                AinariError::Error(format!("Failed to convert proxy-uuid with error: '{e}'"))
            })?;

            let port = proxy.port as u16;
            self.add_proxy(&uuid, port, &proxy.target_address).await?;
        }

        Ok(())
    }
}
