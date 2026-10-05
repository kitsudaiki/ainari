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

use aya::programs::tc::{TcAttachType, qdisc_add_clsact};
use aya::programs::{SchedClassifier, Xdp, XdpMode};
use std::net::Ipv4Addr;

use crate::config::CONFIG;
use crate::core::models::{ArpProxyPod, IfaceConfigPod, TapInfo};
use crate::core::routing_interface::GATEWAY_STATE_HANDLE;
use crate::core::state::GatewayState;
use crate::core::utils::{
    bind_iface_to_table, enable_forwarding, exempt_from_rp_filter, get_ifindex, get_mac_address,
    is_iface_up, parse_mac, run_ip, unbind_iface_from_table, with_table,
};

use ainari_api::common_functions::map_internal_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_interface_structs::*;
use torii_common::{ArpProxy, IFACE_FLAG_FIP, IfaceConfig};

/// Configures an existing interface, places it into its tenant and persists the configuration.
///
/// The interface is brought up and gets its address, if requested, and is registered in the
/// interface map of the datapath with its tenant and its floating IP flag.
///
/// `persist` is called while the gateway state is still locked, right after the interface map
/// was updated. If it or the map fails, every change is reverted: the previous entry of the
/// interface map comes back, an added address is removed again and an interface, which was
/// brought up here, goes down again.
///
/// # Arguments
/// * `req` - The already validated configuration of the interface
/// * `persist` - Writes the configuration to the database
///
/// # Returns
/// `Ok(())` once the interface is configured, `NotFound` if it does not exist, or
/// `InternalError` if it could not be configured or `persist` failed
pub async fn configure_interface(
    req: &IfaceConfigReq,
    persist: impl FnOnce() -> Result<(), ErrorResponse>,
) -> Result<(), ErrorResponse> {
    let name = &req.iface_name;

    let ifindex = get_ifindex(name);
    if ifindex == 0 {
        return Err(ErrorResponse::NotFound(format!(
            "Interface {name} not found"
        )));
    }

    let brought_up = req.up && !is_iface_up(name);
    if req.up {
        std::process::Command::new("ip")
            .args(["link", "set", name, "up"])
            .status()
            .map_err(|e| map_internal_error(&format!("bring interface '{name}' up"), e))?;
    }

    // Adding an address, which the interface already has, fails. Only an address, which was
    // really added here, is removed again on a rollback.
    let mut added_ip_cidr = None;
    if let Some(ip) = &req.ip_cidr {
        let added = std::process::Command::new("ip")
            .args(["addr", "add", ip, "dev", name])
            .status()
            .is_ok_and(|status| status.success());
        if added {
            added_ip_cidr = Some(ip.as_str());
        }
    }

    // Place the port into its tenant. An interface the control plane never
    // registered keeps behaving like a port of the shared tenant that translates
    // floating IPs, which is how the datapath worked before tenants existed.
    let flags = if req.fip_port { IFACE_FLAG_FIP } else { 0 };
    let cfg = IfaceConfig {
        vni: req.vni,
        flags,
    };
    let result = {
        let mut st = GATEWAY_STATE_HANDLE.lock().await;
        let previous_cfg = st.iface_map.get(&ifindex, 0).ok();
        let result = st
            .iface_map
            .insert(ifindex, IfaceConfigPod(cfg), 0)
            .map_err(|e| {
                log::error!("eBPF Map error (interface): {e}");
                ErrorResponse::InternalError("Internal Error".to_string())
            })
            .and_then(|_| persist());

        if result.is_err() {
            let reverted = match previous_cfg {
                Some(previous_cfg) => st.iface_map.insert(ifindex, previous_cfg, 0),
                None => st.iface_map.remove(&ifindex),
            };
            if let Err(e) = reverted {
                log::error!("Failed to restore the interface map entry of '{name}': {e}");
            }
        }
        result
    };

    if result.is_err() {
        if let Some(ip) = added_ip_cidr {
            let _ = run_ip(&["addr", "del", ip, "dev", name]);
        }
        if brought_up {
            let _ = run_ip(&["link", "set", name, "down"]);
        }
    }

    result
}

/// Checks the parts of a TAP registration, which the json-validation can not check.
///
/// # Arguments
/// * `req` - The registration of the TAP device
///
/// # Returns
/// `Ok(())` for a usable registration, otherwise `BadRequest`
pub fn validate_tap_req(req: &TapReq) -> Result<(), ErrorResponse> {
    if let Some(vm_mac) = &req.vm_mac
        && parse_mac(vm_mac).is_none()
    {
        return Err(ErrorResponse::BadRequest("Invalid vm_mac".to_string()));
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

    validate_tap_req(req)?;
    let vm_mac = req.vm_mac.as_deref().and_then(parse_mac);

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

        bind_iface_to_table(name, &table, Ipv4Addr::from(vm_ip)).map_err(|e| {
            map_internal_error(&format!("bind '{name}' to the table of its tenant"), e)
        })?;

        // The kernel can't check the source of the unnumbered TAP and would drop everything the
        // VM sends towards an encrypted destination. In a tenant with a table of its own the rule
        // above checks the source instead. The shared tenant keeps the filter: the route of the
        // VM in `main` leads back over the TAP, which satisfies it.
        if table.is_some() {
            exempt_from_rp_filter(name).map_err(|e| {
                map_internal_error(&format!("exempt '{name}' from the reverse-path filter"), e)
            })?;
        }

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
        let mut link_id = None;
        if let Some(program) = st.bpf.program_mut("overlay_ingress") {
            // Provide explicit type inference to TryInto
            let overlay_prog: Result<&mut Xdp, _> = program.try_into();

            if let Ok(overlay) = overlay_prog {
                match overlay.attach(name, XdpMode::Skb) {
                    Err(e) => println!(
                        "Note: eBPF attach on {} failed (maybe already attached?): {}",
                        name, e
                    ),
                    Ok(id) => {
                        println!(
                            "Dynamically attached overlay_ingress to {} (tenant {})",
                            name, req.vni
                        );
                        link_id = Some(id);
                    }
                }
            }
        }
        // the link is kept, so the program can be detached again on a rollback
        if let Some(link_id) = link_id {
            st.tap_xdp_links.insert(name.to_string(), link_id);
        }

        // Decrypted IPsec traffic is routed into the TAP device by the kernel and never passes
        // the XDP programs, so the ingress filter of the VM is applied to it on the egress of
        // the device. A registered device already has the program, and a second one would only
        // check every packet twice.
        if !st.tap_tc_links.contains_key(name.as_str()) {
            let link_id = attach_tap_egress(&mut st, name).map_err(|e| {
                log::error!("Failed to attach tap_egress to '{name}': {e}");
                ErrorResponse::InternalError("Internal Error".to_string())
            })?;
            st.tap_tc_links.insert(name.to_string(), link_id);
        }
    }

    Ok(())
}

/// Removes everything `register_tap` set up for a TAP device.
///
/// # Arguments
/// * `req` - The registration of the TAP device, which is removed
/// * `delete_device` - True, if the device itself is deleted as well
pub async fn unregister_tap(req: &TapReq, delete_device: bool) {
    let name = &req.tap_name;
    let ifindex = get_ifindex(name);

    if let Some(vm_ip) = req.vm_ip.filter(|vm_ip| !vm_ip.is_unspecified()) {
        let table = CONFIG.network.tenant_table(req.vni);
        let route = format!("{}/32", vm_ip);
        let mut args = vec!["route", "del", route.as_str(), "dev", name.as_str()];
        with_table(&mut args, &table);
        let _ = run_ip(&args);
        let _ = run_ip(&["neigh", "del", &vm_ip.to_string(), "dev", name]);
        unbind_iface_from_table(name, &table);
    }

    {
        let mut st = GATEWAY_STATE_HANDLE.lock().await;
        if ifindex != 0 {
            let _ = st.arp_proxy_map.remove(&ifindex);
            let _ = st.iface_map.remove(&ifindex);
        }
        st.taps.remove(name.as_str());

        // Without its registration the device would act as a port of the shared tenant, so the
        // overlay program is detached again.
        if let Some(link_id) = st.tap_xdp_links.remove(name.as_str())
            && let Some(program) = st.bpf.program_mut("overlay_ingress")
        {
            let overlay_prog: Result<&mut Xdp, _> = program.try_into();
            if let Ok(overlay) = overlay_prog
                && let Err(e) = overlay.detach(link_id)
            {
                log::error!("Failed to detach overlay_ingress from '{name}': {e}");
            }
        }
    }

    {
        let mut st = GATEWAY_STATE_HANDLE.lock().await;
        if let Some(link_id) = st.tap_tc_links.remove(name.as_str())
            && let Some(program) = st.bpf.program_mut("tap_egress")
        {
            let tap_egress: Result<&mut SchedClassifier, _> = program.try_into();
            if let Ok(tap_egress) = tap_egress
                && let Err(e) = tap_egress.detach(link_id)
            {
                log::error!("Failed to detach tap_egress from '{name}': {e}");
            }
        }
    }

    if delete_device {
        let _ = run_ip(&["link", "del", name]);
    }
}

/// Attaches the filter program `tap_egress` to the egress of a TAP device.
///
/// Kernels before 6.6 attach TC programs over a `clsact` qdisc, which is added first. Newer
/// kernels don't need it, and a device, which already has one, keeps it, so a failure to add it
/// is ignored. The attach itself reports, if the program can't be attached at all.
///
/// # Arguments
/// * `st` - The locked gateway state, which holds the loaded program
/// * `name` - Name of the TAP device
///
/// # Returns
/// The id of the new link, or a message describing why the program couldn't be attached
fn attach_tap_egress(
    st: &mut GatewayState,
    name: &str,
) -> Result<aya::programs::tc::SchedClassifierLinkId, String> {
    let _ = qdisc_add_clsact(name);

    let program = st
        .bpf
        .program_mut("tap_egress")
        .ok_or_else(|| "program tap_egress not found".to_string())?;
    let tap_egress: &mut SchedClassifier = program
        .try_into()
        .map_err(|e| format!("tap_egress is no TC program: {e}"))?;
    tap_egress
        .attach(name, TcAttachType::Egress)
        .map_err(|e| e.to_string())
}

/// Reverts a TAP registration, which could not be completed or persisted.
///
/// The new registration is removed completely and the previous one, if there was any, is
/// registered again. A device, which was created by the failed registration, is deleted.
///
/// # Arguments
/// * `req` - The failed registration
/// * `previous` - The registration the device had before, if any
/// * `existed` - True, if the device already existed before the failed registration
pub async fn rollback_tap(req: &TapReq, previous: Option<&TapReq>, existed: bool) {
    unregister_tap(req, !existed).await;

    if let Some(previous) = previous
        && let Err(e) = register_tap(previous).await
    {
        log::error!(
            "Failed to restore the previous registration of TAP '{}': {e}",
            previous.tap_name
        );
    }
}
