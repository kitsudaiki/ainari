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

use crate::core::models::{ArpProxyPod, TapInfo};
use crate::core::routing_interface::GATEWAY_STATE_HANDLE;
use crate::core::utils::{enable_forwarding, get_ifindex, get_mac_address, parse_mac, run_ip};

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_interface_structs::*;
use ainari_api_structs::user_context::UserContext;
use torii_common::ArpProxy;

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

In addition to the eBPF state, a host route and a permanent neighbour entry for
the VM are programmed into the kernel. They are what lets the kernel hand IPsec
protected traffic over to the right TAP after decrypting it, without ever needing
an address or an ARP exchange on this link."###,
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

    let name = &body.tap_name;
    let exists = get_ifindex(name) > 0;

    if !exists {
        std::process::Command::new("ip")
            .args(["tuntap", "add", "mode", "tap", name])
            .status()
            .map_err(|e| ErrorResponse::InternalError(format!("Failed to create TAP: {}", e)))?;
    }

    std::process::Command::new("ip")
        .args(["link", "set", name, "up"])
        .status()
        .map_err(|e| ErrorResponse::InternalError(format!("Failed to bring TAP up: {}", e)))?;

    std::process::Command::new("ethtool")
        .args(["-K", name, "tx", "off", "rx", "off"])
        .status()
        .map_err(|e| {
            ErrorResponse::InternalError(format!("Failed to disable offloading: {}", e))
        })?;

    // The TAP must not own any address: the eBPF datapath is the gateway of the
    // VM, and an address here would collide with every other TAP serving the
    // same subnet on this host.
    let _ = std::process::Command::new("ip")
        .args(["addr", "flush", "dev", name])
        .status();

    let ifindex = get_ifindex(name);
    if ifindex == 0 {
        return Err(ErrorResponse::InternalError(format!(
            "TAP {} has no interface index",
            name
        )));
    }

    let tap_mac = get_mac_address(name);
    let vm_mac = body.vm_mac.as_deref().and_then(parse_mac);
    let vm_ip = body.vm_ip.map(u32::from).unwrap_or(0);

    if body.vm_mac.is_some() && vm_mac.is_none() {
        return Err(ErrorResponse::BadRequest("Invalid vm_mac".to_string()));
    }

    // Give the kernel a way to reach the VM as well. Decrypted IPsec traffic is
    // routed by the kernel, and an unnumbered link can neither be resolved via
    // ARP nor picked by a subnet route - so both are stated explicitly.
    if vm_ip != 0 {
        let vm_ip_str = Ipv4Addr::from(vm_ip).to_string();
        let route = format!("{}/32", vm_ip_str);
        run_ip(&["route", "replace", &route, "dev", name])
            .map_err(ErrorResponse::InternalError)?;

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
            .map_err(ErrorResponse::InternalError)?;
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
            return Err(ErrorResponse::InternalError(
                "eBPF Map error (ARP proxy)".to_string(),
            ));
        }

        st.taps.insert(name.to_string(), TapInfo { tap_mac, vm_mac });
    }

    // DYNAMIC eBPF ATTACHMENT
    if !exists {
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
                    println!("Dynamically attached overlay_ingress to {}", name);
                }
            }
        }
    }

    let resp = TapResp {
        success: true,
        message: "TAP device configured successfully".to_string(),
        tap_name: name.to_string(),
    };

    Ok(CreatedJson(resp))
}
