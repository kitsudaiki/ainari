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

//! Programming of the floating IPs into the NAT maps of the datapath.
//!
//! Used by the floating-ip endpoints and by the restore of the persisted state at startup.

use std::net::Ipv4Addr;

use torii_common::{FipTarget, RouteKey};

use crate::core::models::{FipTargetPod, FloatingIp, RouteKeyPod};
use crate::core::state::GatewayState;

use ainari_api::errors::ErrorResponse;

/// Points a floating IP at the internal address of a VM.
///
/// Both NAT maps are updated: the inbound direction is keyed by the floating IP alone and carries
/// the tenant of the VM in its value, the outbound direction is keyed by `(vni, internal_ip)`,
/// because the internal address is precisely the thing that may repeat across tenants.
///
/// # Arguments
/// * `st` - The locked gateway state
/// * `floating_ip` - The public address
/// * `vni` - Tenant of the internal address
/// * `internal_ip` - Address of the VM behind the floating IP
///
/// # Returns
/// `Ok(())` once both maps are programmed, `Conflict` if the floating IP already points at
/// another VM, or `InternalError` if an eBPF map refused the entry
pub fn add_floating_ip(
    st: &mut GatewayState,
    floating_ip: Ipv4Addr,
    vni: u32,
    internal_ip: Ipv4Addr,
) -> Result<(), ErrorResponse> {
    // A floating IP names one VM of one tenant. Handing the same one to a second
    // tenant would make the inbound direction ambiguous, so it is refused.
    if let Some(existing) = st.floating_ips.get(&floating_ip)
        && (existing.vni != vni || existing.internal_ip != internal_ip)
    {
        return Err(ErrorResponse::Conflict(format!(
            "{} already points at {} in tenant {}",
            floating_ip, existing.internal_ip, existing.vni
        )));
    }

    st.floating_ips
        .insert(floating_ip, FloatingIp { vni, internal_ip });

    let target = FipTarget {
        vni,
        ip: u32::from(internal_ip),
    };
    if st
        .fip_dnat_map
        .insert(u32::from(floating_ip), FipTargetPod(target), 0)
        .is_err()
    {
        log::error!("eBPF Map error (DNAT)");
        return Err(ErrorResponse::InternalError("Internal Error".to_string()));
    }

    let snat_key = RouteKeyPod(RouteKey::new(vni, u32::from(internal_ip)));
    if st
        .fip_snat_map
        .insert(snat_key, u32::from(floating_ip), 0)
        .is_err()
    {
        log::error!("eBPF Map error (SNAT)");
        return Err(ErrorResponse::InternalError("Internal Error".to_string()));
    }

    Ok(())
}

/// Drops a floating IP from the bookkeeping and from both NAT maps.
///
/// # Arguments
/// * `st` - The locked gateway state
/// * `floating_ip` - The public address
///
/// # Returns
/// The removed floating IP, or `None` if it was not registered
pub fn remove_floating_ip(st: &mut GatewayState, floating_ip: Ipv4Addr) -> Option<FloatingIp> {
    let entry = st.floating_ips.remove(&floating_ip)?;

    let _ = st.fip_dnat_map.remove(&u32::from(floating_ip));
    let _ = st.fip_snat_map.remove(&RouteKeyPod(RouteKey::new(
        entry.vni,
        u32::from(entry.internal_ip),
    )));

    Some(entry)
}
