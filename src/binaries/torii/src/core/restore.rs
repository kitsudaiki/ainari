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

//! Restore of the persisted gateway state after a restart.
//!
//! The eBPF maps start out empty with every start of the torii, so everything the control plane
//! configured over the endpoints is read back from the database and programmed again. The order
//! matters: interfaces and TAP devices come first, because routes resolve the MAC of the VM and
//! the tenant of the port from the TAP registry, and the packet filters need their routes and
//! the TAP devices behind them.
//!
//! A single entry, which can not be restored anymore (for example because its interface is
//! gone), is logged and skipped, so it doesn't keep the rest of the network down.
//!
//! The proxies are restored as well. They don't depend on the datapath, but belong to the state
//! the control plane configured over the endpoints.

use crate::core::filter::{apply_filter, filter_slot};
use crate::core::floating_ip::add_floating_ip;
use crate::core::interface::{configure_interface, register_tap};
use crate::core::mls::restore_network_keys;
use crate::core::models::Route;
use crate::core::proxy_handler::PROXY_HANDLER;
use crate::core::routing::{add_route, update_route};
use crate::core::routing_interface::GATEWAY_STATE_HANDLE;
use crate::database::{
    floating_ip_table, network_filter_table, network_interface_table, route_table, tap_table,
};

use ainari_api_structs::network_interface_structs::*;
use ainari_api_structs::route_structs::RouteReq;
use ainari_common::error::AinariError;

/// Restarts all proxies and re-programs all interfaces, TAP devices, routes, packet filters and
/// floating IPs, which are persisted in the database, into the datapath.
///
/// # Returns
///
/// * `Ok(())` once every entry was processed. Network entries, which could not be restored, are
///   logged.
/// * `Err(AinariError)` if the persisted state could not be read from the database or a proxy
///   could not be restarted.
pub async fn restore_gateway_state() -> Result<(), AinariError> {
    {
        let mut proxy_handler = PROXY_HANDLER.write().await;
        proxy_handler.fill_proxy_handler().await?;
        log::info!("Restored {} proxy(s)", proxy_handler.proxys.len());
    }

    let db_error = |obj_type: &str, e: diesel::result::Error| {
        AinariError::InternalError(format!(
            "Failed to get list of {obj_type} from database: '{e}'"
        ))
    };

    let interfaces = network_interface_table::list_network_interfaces()
        .map_err(|e| db_error("interfaces", e))?;
    let taps = tap_table::list_taps().map_err(|e| db_error("taps", e))?;
    let routes = route_table::list_routes().map_err(|e| db_error("routes", e))?;
    let filters = network_filter_table::list_filter_rules().map_err(|e| db_error("filters", e))?;
    let floating_ips =
        floating_ip_table::list_floating_ips().map_err(|e| db_error("floating ips", e))?;

    for entry in interfaces {
        let req: IfaceConfigReq = entry.into();
        if let Err(e) = configure_interface(&req, || Ok(())).await {
            log::error!("Failed to restore interface '{}': {e}", req.iface_name);
        }
    }

    for entry in taps {
        let req: TapReq = entry.into();
        if let Err(e) = register_tap(&req).await {
            log::error!("Failed to restore TAP '{}': {e}", req.tap_name);
        }
    }

    for entry in routes {
        let route: Route = entry.into();
        let route_uuid = route.uuid;
        let req = RouteReq::from(&route);
        // A route, which the gateway built from its own config at startup, already exists with
        // the same UUID. Its persisted version is an update of it, which may have moved it to
        // another destination, so it is applied as an update.
        let exists = GATEWAY_STATE_HANDLE
            .lock()
            .await
            .routes
            .contains_key(&route_uuid);
        let result = if exists {
            update_route(route_uuid, &req, |_, _| Ok(())).await
        } else {
            add_route(route_uuid, &req, |_| Ok(())).await
        };
        if let Err(e) = result {
            log::error!(
                "Failed to restore route '{route_uuid}' to {} in tenant {}: {e}",
                req.dest_ip,
                req.vni
            );
        }
    }

    let mut st = GATEWAY_STATE_HANDLE.lock().await;

    for (key, rules) in filters {
        let result =
            filter_slot(&st, &key).and_then(|slot| apply_filter(&mut st, key, slot, rules));
        if let Err(e) = result {
            log::error!(
                "Failed to restore the {} packet-filter of {} in tenant {}: {e}",
                key.direction,
                key.ip,
                key.vni
            );
        }
    }

    for entry in floating_ips {
        if let Err(e) = add_floating_ip(
            &mut st,
            entry.floating_ip,
            entry.vni,
            entry.internal_ip,
            || Ok(()),
        ) {
            log::error!("Failed to restore floating ip '{}': {e}", entry.floating_ip);
        }
    }

    log::info!(
        "Gateway state restored: {} route(s), {} TAP device(s), {} packet-filter(s) and {} floating ip(s)",
        st.routes.len(),
        st.taps.len(),
        st.filters.len(),
        st.floating_ips.len()
    );
    drop(st);

    // the keys of the encrypted routes are derived from the MLS-groups, which are restored with
    // the first access, so they can only be installed after the routes are back
    restore_network_keys().await;

    Ok(())
}
