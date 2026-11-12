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

//! State of the key-exchange of the gateway, as the status-endpoint reports it.

use crate::core::mls_key_exchange::network_keys::{EpochId, PairKeys};
use crate::core::mls_key_exchange::state::MlsState;
use ainari_api_structs::network_crypto_structs::{
    MlsEpochIdResp, MlsNetworkStatusResp, MlsPairStatusResp, MlsPairWaitingFor, MlsStatusResp,
};

/// Tells, which step of `step` a pair waits for.
///
/// # Arguments
/// * `pair` - The pair
///
/// # Returns
/// The step
pub fn waiting_for(pair: &PairKeys) -> MlsPairWaitingFor {
    match pair.target {
        // the ready is queued in the same step, which installed the incoming keys
        Some(target) if pair.ready_sent != Some(target) => MlsPairWaitingFor::Ingress,
        Some(target) if pair.egress != Some(target) => match pair.peer_ready >= Some(target) {
            true => MlsPairWaitingFor::Egress,
            false => MlsPairWaitingFor::PeerReady,
        },
        Some(_) => MlsPairWaitingFor::PeerSwitched,
        None if !pair.retiring.is_empty() => MlsPairWaitingFor::OldIngressInUse,
        None => MlsPairWaitingFor::Nothing,
    }
}

/// Describes the state of the key-exchange of the gateway.
///
/// # Arguments
/// * `state` - The MLS-client
///
/// # Returns
/// The pairs of all networks with their epochs and the step, which they wait for
pub fn status(state: &MlsState) -> MlsStatusResp {
    let epoch_id = |id: &EpochId| MlsEpochIdResp {
        generation: id.0,
        epoch: id.1,
    };
    let networks = state
        .network_keys
        .iter()
        .map(|(vni, keys)| MlsNetworkStatusResp {
            vni: *vni,
            generation: keys.generation,
            epoch: state.groups.get(vni).map(|group| group.epoch().as_u64()),
            pairs: keys
                .pairs
                .iter()
                .map(|(peer, pair)| MlsPairStatusResp {
                    peer: peer.to_string(),
                    egress: pair.egress.as_ref().map(epoch_id),
                    target: pair.target.as_ref().map(epoch_id),
                    ingress: pair.ingress.iter().map(epoch_id).collect(),
                    retiring: pair.retiring.keys().map(epoch_id).collect(),
                    waiting_for: waiting_for(pair),
                    stale_since: pair.stale_since,
                    last_error: pair.last_error.clone(),
                })
                .collect(),
        })
        .collect();
    MlsStatusResp {
        client_id: state.client_id().map(str::to_string),
        networks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::mls_key_exchange::pair_messages::{PairMessage, mark_sent};
    use crate::core::mls_key_exchange::pairs::step;
    use crate::core::mls_key_exchange::test_support::{
        A, B, TestGateway, TestKernel, current, deliver_to, idle, joined, step_of,
    };

    #[test]
    fn the_status_tells_the_step_a_pair_waits_for() {
        let mut a = TestGateway::new(1, 1, 2, 2);
        let mut b = TestGateway::new(2, 2, 1, 1);
        let waiting =
            |gateway: &TestGateway| waiting_for(&gateway.keys.pairs[&gateway.conn.peer_gateway_ip]);
        for gateway in [&mut a, &mut b] {
            let pair = gateway.keys.pairs.values_mut().next().unwrap();
            pair.target = None;
        }
        assert_eq!(waiting(&a), MlsPairWaitingFor::Nothing);

        a.record(2);
        assert_eq!(waiting(&a), MlsPairWaitingFor::Ingress);
        let from_a = step_of(&mut a);
        assert_eq!(waiting(&a), MlsPairWaitingFor::PeerReady);

        b.record(2);
        deliver_to(from_a, &a, &mut b);
        b.keys.pairs.values_mut().next().unwrap().peer_ready = Some((1, 2));
        // the switch of b failed, so it waits for its outgoing keys
        let failed = step(
            &mut b.keys,
            &mut TestKernel {
                refuse_egress: true,
            },
            0,
        );
        let mut from_b = Vec::new();
        for (kind, id, peers) in failed.messages {
            mark_sent(&mut b.keys, kind, id, &peers);
            from_b.push(PairMessage {
                kind,
                generation: id.0,
                epoch: id.1,
                peers: peers.iter().map(|peer| peer.to_string()).collect(),
            });
        }
        assert_eq!(waiting(&b), MlsPairWaitingFor::Egress);
        from_b.extend(step_of(&mut b));
        assert_eq!(waiting(&b), MlsPairWaitingFor::PeerSwitched);

        deliver_to(from_b, &b, &mut a);
        step_of(&mut a);
        assert_eq!(waiting(&a), MlsPairWaitingFor::OldIngressInUse);
        idle(&mut a);
        assert_eq!(waiting(&a), MlsPairWaitingFor::Nothing);
    }

    #[test]
    fn the_status_lists_the_pairs_of_every_network() {
        let gateways = joined(&[A, B]);
        let status = status(&gateways[0]);
        assert_eq!(status.client_id.as_deref(), Some(A));
        assert_eq!(status.networks.len(), 1);
        let network = &status.networks[0];
        assert_eq!(network.vni, 5);
        assert_eq!(network.generation, 1);
        assert_eq!(network.epoch, Some(current(&gateways[0]).1));
        assert_eq!(network.pairs.len(), 1);
        let pair = &network.pairs[0];
        assert_eq!(pair.peer, B);
        assert_eq!(pair.waiting_for, MlsPairWaitingFor::Ingress);
        assert_eq!(pair.egress, pair.target);
        assert!(pair.last_error.is_none());
    }
}
