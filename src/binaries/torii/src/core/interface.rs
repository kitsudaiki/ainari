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

//! Configuration of the interfaces and TAP devices of the gateway.
//!
//! Used by the network-interface endpoints and by the restore of the persisted state at startup.

use aya::programs::{Xdp, XdpMode};
use std::net::Ipv4Addr;

use crate::config::CONFIG;
use crate::core::models::{ArpProxyPod, IfaceConfigPod, TapInfo};
use crate::core::routing_interface::GATEWAY_STATE_HANDLE;
use crate::core::utils::{
    bind_iface_to_table, enable_forwarding, get_ifindex, get_mac_address, parse_mac, run_ip,
    unbind_iface_from_table, with_table,
};

use ainari_api::common_functions::map_internal_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_interface_structs::*;
use torii_common::{ArpProxy, IFACE_FLAG_FIP, IfaceConfig};

/// Configures an existing interface and places it into its tenant.
///
/// The interface is brought up and gets its address, if requested, and is registered in the
/// interface map of the datapath with its tenant and its floating IP flag.
///
/// # Arguments
/// * `req` - The already validated configuration of the interface
///
/// # Returns
/// `Ok(())` once the interface is configured, `NotFound` if it does not exist, or
/// `InternalError` if it could not be configured
pub async fn configure_interface(req: &IfaceConfigReq) -> Result<(), ErrorResponse> {
    let name = &req.iface_name;

    if req.up {
        std::process::Command::new("ip")
            .args(["link", "set", name, "up"])
            .status()
            .map_err(|e| map_internal_error(&format!("bring interface '{name}' up"), e))?;
    }
    if let Some(ip) = &req.ip_cidr {
        let _ = std::process::Command::new("ip")
            .args(["addr", "add", ip, "dev", name])
            .status();
    }

    let ifindex = get_ifindex(name);
    if ifindex == 0 {
        return Err(ErrorResponse::NotFound(format!(
            "Interface {name} not found"
        )));
    }

    // Place the port into its tenant. An interface the control plane never
    // registered keeps behaving like a port of the shared tenant that translates
    // floating IPs, which is how the datapath worked before tenants existed.
    let flags = if req.fip_port { IFACE_FLAG_FIP } else { 0 };
    let cfg = IfaceConfig {
        vni: req.vni,
        flags,
    };
    let mut st = GATEWAY_STATE_HANDLE.lock().await;
    if st
        .iface_map
        .insert(ifindex, IfaceConfigPod(cfg), 0)
        .is_err()
    {
        log::error!("eBPF Map error (interface)");
        return Err(ErrorResponse::InternalError("Internal Error".to_string()));
    }

    Ok(())
}

/// Creates a TAP device, if necessary, and registers it as a port of its tenant.
///
/// The device stays unnumbered. The XDP ARP responder, the interface map and the overlay program
/// are set up for it, and for a VM with a known address a host route and a permanent neighbour
/// entry are programmed into the kernel, so decrypted IPsec traffic finds its way to the VM.
///
/// # Arguments
/// * `req` - The already validated registration of the TAP device
///
/// # Returns
/// `Ok(())` once the device is registered, `BadRequest` for an invalid `vm_mac`, or
/// `InternalError` if the device could not be set up
pub async fn register_tap(req: &TapReq) -> Result<(), ErrorResponse> {
    let name = &req.tap_name;
    let exists = get_ifindex(name) > 0;

    let vm_mac = req.vm_mac.as_deref().and_then(parse_mac);
    if req.vm_mac.is_some() && vm_mac.is_none() {
        return Err(ErrorResponse::BadRequest("Invalid vm_mac".to_string()));
    }

    // Re-registering a TAP into another tenant has to take its old policy routing
    // rule with it, or the kernel would keep sending the traffic of this link into
    // the table of the tenant it just left.
    let previous_vni = {
        GATEWAY_STATE_HANDLE
            .lock()
            .await
            .taps
            .get(name.as_str())
            .map(|info| info.vni)
    };
    if let Some(previous_vni) = previous_vni
        && previous_vni != req.vni
    {
        unbind_iface_from_table(name, &CONFIG.network.tenant_table(previous_vni));
    }

    if !exists {
        std::process::Command::new("ip")
            .args(["tuntap", "add", "mode", "tap", name])
            .status()
            .map_err(|e| map_internal_error(&format!("create TAP '{name}'"), e))?;
    }

    std::process::Command::new("ip")
        .args(["link", "set", name, "up"])
        .status()
        .map_err(|e| map_internal_error(&format!("bring TAP '{name}' up"), e))?;

    std::process::Command::new("ethtool")
        .args(["-K", name, "tx", "off", "rx", "off"])
        .status()
        .map_err(|e| map_internal_error(&format!("disable offloading of TAP '{name}'"), e))?;

    // The TAP must not own any address: the eBPF datapath is the gateway of the
    // VM, and an address here would collide with every other TAP serving the
    // same subnet on this host.
    let _ = std::process::Command::new("ip")
        .args(["addr", "flush", "dev", name])
        .status();

    let ifindex = get_ifindex(name);
    if ifindex == 0 {
        log::error!("TAP '{name}' has no interface index");
        return Err(ErrorResponse::InternalError("Internal Error".to_string()));
    }

    let tap_mac = get_mac_address(name);
    let vm_ip = req.vm_ip.map(u32::from).unwrap_or(0);

    // Give the kernel a way to reach the VM as well. Decrypted IPsec traffic is
    // routed by the kernel, and an unnumbered link can neither be resolved via
    // ARP nor picked by a subnet route - so both are stated explicitly. In a
    // tenant of its own the entries live in that tenant's table, which the rule
    // below selects for everything arriving on this TAP.
    let table = CONFIG.network.tenant_table(req.vni);
    if vm_ip != 0 {
        let vm_ip_str = Ipv4Addr::from(vm_ip).to_string();
        let route = format!("{}/32", vm_ip_str);
        let mut args = vec!["route", "replace", route.as_str(), "dev", name.as_str()];
        with_table(&mut args, &table);
        run_ip(&args).map_err(|e| map_internal_error(&format!("add host-route '{route}'"), e))?;

        bind_iface_to_table(name, &table).map_err(|e| {
            map_internal_error(&format!("bind '{name}' to the table of its tenant"), e)
        })?;

        if let Some(mac) = vm_mac {
            let mac_str = format!(
                "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
            );
            run_ip(&[
                "neigh",
                "replace",
                &vm_ip_str,
                "lladdr",
                &mac_str,
                "dev",
                name,
                "nud",
                "permanent",
            ])
            .map_err(|e| map_internal_error(&format!("add neighbour-entry '{vm_ip_str}'"), e))?;
        }
        enable_forwarding(&format!("/proc/sys/net/ipv4/conf/{}/forwarding", name));
    }

    {
        let mut st = GATEWAY_STATE_HANDLE.lock().await;

        // Teach the XDP ARP responder to serve this link. Answers carry the MAC
        // of the TAP itself, which becomes the gateway MAC of the attached VM.
        let proxy = ArpProxy {
            mac: tap_mac,
            _pad: [0; 2],
            vm_ip,
        };
        if st
            .arp_proxy_map
            .insert(ifindex, ArpProxyPod(proxy), 0)
            .is_err()
        {
            log::error!("eBPF Map error (ARP proxy)");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }

        // The port of a tenant, and never a floating IP port: a VM must not be
        // able to address a floating IP and end up in the tenant behind it.
        let cfg = IfaceConfig {
            vni: req.vni,
            flags: 0,
        };
        if st
            .iface_map
            .insert(ifindex, IfaceConfigPod(cfg), 0)
            .is_err()
        {
            log::error!("eBPF Map error (interface)");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }

        st.taps.insert(
            name.to_string(),
            TapInfo {
                vni: req.vni,
                tap_mac,
                vm_mac,
            },
        );
    }

    // DYNAMIC eBPF ATTACHMENT
    //
    // This runs for a device that already existed as well. A TAP survives a
    // restart of the gateway, and so does a link the operator created by hand,
    // and neither of them carries the program until it is attached here. A second
    // attach on a link that already has one is refused by the kernel and only
    // logged - the interesting case is the one that would otherwise leave a port
    // of a tenant silently unprogrammed.
    {
        let mut st = GATEWAY_STATE_HANDLE.lock().await;
        if let Some(program) = st.bpf.program_mut("overlay_ingress") {
            // Provide explicit type inference to TryInto
            let overlay_prog: Result<&mut Xdp, _> = program.try_into();

            if let Ok(overlay) = overlay_prog {
                if let Err(e) = overlay.attach(name, XdpMode::Skb) {
                    println!(
                        "Note: eBPF attach on {} failed (maybe already attached?): {}",
                        name, e
                    );
                } else {
                    println!(
                        "Dynamically attached overlay_ingress to {} (tenant {})",
                        name, req.vni
                    );
                }
            }
        }
    }

    Ok(())
}
