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

use actix_web::web::Json;
use apistos::actix::CreatedJson;
use apistos::api_operation;
use aya::programs::{Xdp, XdpFlags};
use std::net::Ipv4Addr;
use validator::Validate;

use crate::config::CONFIG;
use crate::core::models::{ArpProxyPod, IfaceConfigPod, TapInfo};
use crate::core::routing_interface::GATEWAY_STATE_HANDLE;
use crate::core::utils::{
    bind_iface_to_table, enable_forwarding, get_ifindex, get_mac_address, parse_mac, run_ip,
    unbind_iface_from_table, validate_vni, with_table,
};

use ainari_api::common_functions::map_internal_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_interface_structs::*;
use ainari_api_structs::user_context::UserContext;
use torii_common::{ArpProxy, IfaceConfig};

#[api_operation(
    tag = "network_interface",
    summary = "Register new tap-device",
    description = r###"Create a new TAP device and dynamically attach the eBPF overlay program.

The device stays intentionally unnumbered - no IP address and no subnet are
attached to it, and any leftover address is flushed. Everything the VM needs from
a gateway is provided by eBPF instead: the XDP ARP responder answers the requests
of the VM (using the MAC of the TAP device itself) and the routing maps forward
the traffic. Because no subnet is claimed on the host side, several VMs of the
same subnet can be attached to the same host.

The device is also registered as a port of its tenant. That registration is the
only source of the VNI for everything the VM sends, which is what lets two VMs on
this host carry the very same address: they differ by the port their frames arrive
on, and every lookup downstream is keyed by that tenant.

In addition to the eBPF state, a host route and a permanent neighbour entry for
the VM are programmed into the kernel. They are what lets the kernel hand IPsec
protected traffic over to the right TAP after decrypting it, without ever needing
an address or an ARP exchange on this link. For a tenant other than the shared one
they go into that tenant's own routing table, selected by an `ip rule` on this TAP
- the kernel has no VNI, so the ingress interface is the only thing left to tell
the tenants apart on that path."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn register_tap_internal(
    body: Json<TapReq>,
    _context: UserContext,
) -> Result<CreatedJson<TapResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;
    validate_vni(body.vni).map_err(ErrorResponse::BadRequest)?;

    let name = &body.tap_name;
    let exists = get_ifindex(name) > 0;

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
        && previous_vni != body.vni
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
    let vm_mac = body.vm_mac.as_deref().and_then(parse_mac);
    let vm_ip = body.vm_ip.map(u32::from).unwrap_or(0);

    if body.vm_mac.is_some() && vm_mac.is_none() {
        return Err(ErrorResponse::BadRequest("Invalid vm_mac".to_string()));
    }

    // Give the kernel a way to reach the VM as well. Decrypted IPsec traffic is
    // routed by the kernel, and an unnumbered link can neither be resolved via
    // ARP nor picked by a subnet route - so both are stated explicitly. In a
    // tenant of its own the entries live in that tenant's table, which the rule
    // below selects for everything arriving on this TAP.
    let table = CONFIG.network.tenant_table(body.vni);
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
            vni: body.vni,
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
                vni: body.vni,
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
                if let Err(e) = overlay.attach(name, XdpFlags::SKB_MODE) {
                    println!(
                        "Note: eBPF attach on {} failed (maybe already attached?): {}",
                        name, e
                    );
                } else {
                    println!(
                        "Dynamically attached overlay_ingress to {} (tenant {})",
                        name, body.vni
                    );
                }
            }
        }
    }

    let resp = TapResp {
        success: true,
        message: format!("TAP device configured successfully in tenant {}", body.vni),
        tap_name: name.to_string(),
        vni: body.vni,
    };

    Ok(CreatedJson(resp))
}
