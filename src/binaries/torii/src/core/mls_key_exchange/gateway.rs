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

//! Connects the MLS-state with the gateway: installs the keys of the networks and moves the
//! pairs on, with the locks of both states.

use crate::config::CONFIG;
use crate::core::crypto::sa_packet_count;
use crate::core::ebpf_interface::EBPF_INTERFACE_HANDLE;
use crate::core::ebpf_interface::EBPFInterface;
use crate::core::mls_key_exchange::kernel;
use crate::core::mls_key_exchange::keys;
use crate::core::mls_key_exchange::network_keys::{EpochId, NetworkKeys};
use crate::core::mls_key_exchange::pairs::{self, PairKernel};
use crate::core::mls_key_exchange::state::{MLS_STATE_HANDLE, MlsState};
use crate::core::utils::get_local_ip;
use ainari_api_structs::network_crypto_structs::CryptoDirection;
use openmls::prelude::*;
use openmls_rust_crypto::RustCrypto;
use std::net::Ipv4Addr;

/// Brings the keys of a network in line with its recorded epochs. Errors are logged, because the
/// change, which caused the call, already happened.
///
/// # Arguments
/// * `mls` - The locked MLS-client
/// * `ebpf_interf` - The locked gateway state
/// * `vni` - Tenant of the network
pub fn apply_keys(mls: &MlsState, ebpf_interf: &mut EBPFInterface, vni: u32) {
    let keys = mls.network_keys.get(&vni);
    if let Err(e) = kernel::apply_network_keys(ebpf_interf, keys, mls.provider.crypto(), vni) {
        log::error!("Failed to apply the keys of tenant {vni}: {e}");
    }
}

/// Brings the keys of a network in line with its routes, after a route was added, changed or
/// removed.
///
/// # Arguments
/// * `vni` - Tenant of the network
pub async fn refresh_network_keys(vni: u32) {
    let mls = MLS_STATE_HANDLE.lock().await;
    let mut ebpf_interf = EBPF_INTERFACE_HANDLE.lock().await;
    apply_keys(&mls, &mut ebpf_interf, vni);
}

/// Installs the keys of all groups again after a restart of the gateway.
pub async fn restore_network_keys() {
    let mls = MLS_STATE_HANDLE.lock().await;
    let mut ebpf_interf = EBPF_INTERFACE_HANDLE.lock().await;
    for vni in mls.network_keys.keys() {
        apply_keys(&mls, &mut ebpf_interf, *vni);
    }
}

/// Access to the keys of a network in the kernel of this gateway
struct GatewayKernel<'a> {
    ebpf_interf: &'a mut EBPFInterface,
    crypto: RustCrypto,
    vni: u32,
    local_gateway_ip: Ipv4Addr,
}

impl PairKernel for GatewayKernel<'_> {
    fn apply(&mut self, keys: &NetworkKeys) {
        if let Err(e) =
            kernel::apply_network_keys(self.ebpf_interf, Some(keys), &self.crypto, self.vni)
        {
            log::error!("Failed to apply the keys of tenant {}: {e}", self.vni);
        }
    }

    fn installed(
        &self,
        keys: &NetworkKeys,
        peer: Ipv4Addr,
        id: EpochId,
        direction: CryptoDirection,
    ) -> bool {
        kernel::pair_installed(
            self.ebpf_interf,
            keys,
            &self.crypto,
            self.vni,
            peer,
            id,
            direction,
            self.local_gateway_ip,
        )
    }
}

/// Moves the pairs of all networks on (see `pairs::step`).
pub async fn drive_all_pairs() {
    let mut mls = MLS_STATE_HANDLE.lock().await;
    let Some(local_gateway_ip) = get_local_ip(&CONFIG.network.underlay_iface) else {
        return;
    };
    let mut ebpf_interf = EBPF_INTERFACE_HANDLE.lock().await;
    let vnis: Vec<u32> = mls.network_keys.keys().copied().collect();
    let now = chrono::Utc::now().timestamp();
    let mut changed = false;
    for vni in vnis {
        let mut kernel = GatewayKernel {
            ebpf_interf: &mut ebpf_interf,
            crypto: RustCrypto::default(),
            vni,
            local_gateway_ip,
        };
        changed |= pairs::drive_pairs(&mut mls, vni, &mut kernel, now);
    }
    if changed && let Err(e) = mls.save() {
        log::error!("Failed to persist the MLS-state: {e:?}");
    }
}

/// Removes the incoming keys of the retiring epochs of all networks, which no packet uses
/// anymore (see `keys::remove_idle_keys`).
pub async fn remove_idle_network_keys() {
    let mut guard = MLS_STATE_HANDLE.lock().await;
    let mls = &mut *guard;
    let vnis: Vec<u32> = mls
        .network_keys
        .iter()
        .filter(|(_, keys)| keys.pairs.values().any(|pair| !pair.retiring.is_empty()))
        .map(|(vni, _)| *vni)
        .collect();
    if vnis.is_empty() {
        return;
    }
    let Some(local_gateway_ip) = get_local_ip(&CONFIG.network.underlay_iface) else {
        return;
    };

    let mut ebpf_interf = EBPF_INTERFACE_HANDLE.lock().await;
    let mut changed = false;
    for vni in vnis {
        let connections = keys::desired_connections(&ebpf_interf.routes, &ebpf_interf.taps, vni);
        let Some(network_keys) = mls.network_keys.get_mut(&vni) else {
            continue;
        };
        if keys::remove_idle_keys(
            network_keys,
            mls.provider.crypto(),
            vni,
            &connections,
            local_gateway_ip,
            &mut sa_packet_count,
        ) {
            changed = true;
            apply_keys(mls, &mut ebpf_interf, vni);
        }
    }
    if changed && let Err(e) = mls.save() {
        log::error!("Failed to persist the MLS-state: {e:?}");
    }
}
