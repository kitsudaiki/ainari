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

//! Helpers of the tests, which simulate several gateways within one process.

use std::collections::HashSet;
use std::net::Ipv4Addr;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

use crate::core::mls_key_exchange::keys::BASE_LENGTH;

use crate::core::mls_key_exchange::commit::{
    Addition, CommitChange, merge_pending, pending_change, stage_change, stage_update,
};
use crate::core::mls_key_exchange::group::{create_group, create_key_packages, decode_key_package};
use crate::core::mls_key_exchange::keys::{DesiredConnection, desired_sas, remove_idle_keys};
use crate::core::mls_key_exchange::messages::{self, OutgoingMessage, Processed, process_message};
use crate::core::mls_key_exchange::network_keys::{EpochId, NetworkKeys, PairKeys};
use crate::core::mls_key_exchange::pair_messages::{PairMessage, handle_message, mark_sent, take};
use crate::core::mls_key_exchange::pairs::{PairKernel, drive_pairs, step};
use crate::core::mls_key_exchange::state::MlsState;
use ainari_api_structs::mls_structs::SigningKey;
use ainari_api_structs::mls_structs::*;
use ainari_api_structs::network_crypto_structs::CryptoDirection;
use chrono::Utc;
use openmls::prelude::OpenMlsProvider;
use openmls_rust_crypto::RustCrypto;
use uuid::Uuid;

/// Signing-key of hanami within the tests
pub(crate) fn grant_key() -> SigningKey {
    SigningKey::from_bytes(&[5u8; 32])
}

/// Signs a grant for a gateway to join the group of a tenant.
pub(crate) fn grant_for(state: &MlsState, vni: u32) -> MlsGrant {
    let identity = state.identity.as_ref().unwrap();
    MlsGrant::sign(
        &MlsGrantPayload {
            vni,
            client_id: identity.client_id.clone(),
            signature_key: BASE64.encode(identity.signer.public()),
            action: MlsGrantAction::Add,
            issued_at: 0,
            expires_at: i64::MAX,
        },
        &grant_key(),
    )
}

/// Creates a gateway with an identity, which is only held in memory.
pub(crate) fn new_gateway(client_id: &str) -> MlsState {
    let mut state = MlsState::new_in_memory();
    state.create_identity(client_id.to_string()).unwrap();
    state
}

/// Prepares the addition of `invited` by `inviter` with a fresh key-package and the grant of
/// `grant_owner`.
pub(crate) fn addition_with_grant(
    inviter: &MlsState,
    invited: &MlsState,
    grant_owner: &MlsState,
    vni: u32,
) -> Addition {
    let client_id = invited.client_id().unwrap().to_string();
    let encoded = create_key_packages(invited, 1).unwrap().remove(0);
    Addition {
        key_package: decode_key_package(inviter, &client_id, &encoded).unwrap(),
        client_id,
        grant: grant_for(grant_owner, vni),
    }
}

/// Prepares the addition of `invited` by `inviter` with a fresh key-package and its grant.
pub(crate) fn addition(inviter: &MlsState, invited: &MlsState, vni: u32) -> Addition {
    addition_with_grant(inviter, invited, invited, vni)
}

/// Lets `inviter` invite `invited` into the group of the tenant and delivers the messages to
/// `invited` and to all other `members`.
pub(crate) fn invite(
    inviter: &mut MlsState,
    invited: &mut MlsState,
    members: &mut [&mut MlsState],
    vni: u32,
) {
    let addition = addition(inviter, invited, vni);
    let messages = stage_change(inviter, vni, vec![addition], &[]).unwrap();
    merge_pending(inviter, vni).unwrap();
    deliver(&messages, invited, members);
}

/// Delivers messages to the gateways, which are named as their recipients.
pub(crate) fn deliver(
    messages: &[OutgoingMessage],
    first: &mut MlsState,
    others: &mut [&mut MlsState],
) {
    for message in messages {
        for recipient in &message.recipients {
            let gateway: &mut MlsState = if first.client_id() == Some(recipient.as_str()) {
                first
            } else {
                others
                    .iter_mut()
                    .find(|it| it.client_id() == Some(recipient.as_str()))
                    .expect("unknown recipient")
            };
            process_message(
                gateway,
                message.message_type,
                &message.payload,
                &message.grants,
                &grant_key().verifying_key(),
            )
            .unwrap();
        }
    }
}

pub(crate) fn secret(state: &MlsState, vni: u32) -> Vec<u8> {
    state.groups[&vni]
        .export_secret(state.provider.crypto(), "test", b"", 32)
        .unwrap()
}

pub(crate) fn ip(last: u8) -> Ipv4Addr {
    Ipv4Addr::new(192, 168, 0, last)
}

pub(crate) fn gateway(last: u8) -> Ipv4Addr {
    Ipv4Addr::new(10, 0, 0, last)
}

/// One gateway of the rotation-tests with one VM, which talks to the VM of the other one
pub(crate) struct TestGateway {
    pub(crate) conn: DesiredConnection,
    pub(crate) local_gateway_ip: Ipv4Addr,
    pub(crate) keys: NetworkKeys,
}

impl TestGateway {
    pub(crate) fn new(local_vm: u8, local_gateway: u8, remote_vm: u8, remote_gateway: u8) -> Self {
        TestGateway {
            conn: DesiredConnection {
                local_ip: ip(local_vm),
                remote_ip: ip(remote_vm),
                peer_gateway_ip: gateway(remote_gateway),
            },
            local_gateway_ip: gateway(local_gateway),
            keys: NetworkKeys::new(1, vec![1u8; BASE_LENGTH], &[gateway(remote_gateway)]),
        }
    }

    pub(crate) fn peers(&self) -> [Ipv4Addr; 1] {
        [self.conn.peer_gateway_ip]
    }

    pub(crate) fn spis(&self, direction: CryptoDirection) -> HashSet<u32> {
        desired_sas(
            &self.keys,
            &RustCrypto::default(),
            5,
            &[self.conn],
            self.local_gateway_ip,
        )
        .unwrap()
        .into_iter()
        .filter(|desired| desired.direction == direction)
        .map(|desired| desired.sa.spi)
        .collect()
    }

    pub(crate) fn record(&mut self, epoch: u64) {
        let peers = self.peers();
        self.keys
            .record(epoch, vec![epoch as u8; BASE_LENGTH], &peers, None, 0);
    }

    /// Switches the outgoing traffic to an epoch of the current group, as the pair does after
    /// the `Ready` of the peer
    pub(crate) fn switch(&mut self, epoch: u64) {
        let id = (self.keys.generation, epoch);
        for pair in self.keys.pairs.values_mut() {
            pair.egress = Some(id);
        }
    }

    /// Retires the older incoming epochs, as the pair does after the `Switched` of the peer
    pub(crate) fn retire_older(&mut self, epoch: u64) {
        let id = (self.keys.generation, epoch);
        for pair in self.keys.pairs.values_mut() {
            let older: Vec<EpochId> = pair.ingress.range(..id).copied().collect();
            for old in older {
                pair.ingress.remove(&old);
                pair.retiring.entry(old).or_default();
            }
        }
    }

    pub(crate) fn check_counters(
        &mut self,
        counter: &mut impl FnMut(Ipv4Addr, Ipv4Addr, u32) -> Result<Option<u64>, String>,
    ) -> bool {
        remove_idle_keys(
            &mut self.keys,
            &RustCrypto::default(),
            5,
            &[self.conn],
            self.local_gateway_ip,
            counter,
        )
    }

    /// Starts a new group with its own base secrets, like after izakaya lost its database
    pub(crate) fn replace(&mut self, epoch: u64) {
        let peers = self.peers();
        self.keys
            .replace(epoch, vec![100 + epoch as u8; BASE_LENGTH], &peers, 0);
    }

    pub(crate) fn record_new_group(&mut self, epoch: u64) {
        let peers = self.peers();
        self.keys
            .record(epoch, vec![100 + epoch as u8; BASE_LENGTH], &peers, None, 0);
    }
}

/// Checks, that the receiver has the key, which the sender uses, in both directions.
pub(crate) fn in_sync(a: &TestGateway, b: &TestGateway) -> bool {
    let a_egress = a.spis(CryptoDirection::Egress);
    let b_egress = b.spis(CryptoDirection::Egress);
    a_egress.len() == 1
        && b_egress.len() == 1
        && a_egress.is_subset(&b.spis(CryptoDirection::Ingress))
        && b_egress.is_subset(&a.spis(CryptoDirection::Ingress))
}

pub(crate) const A: &str = "10.0.0.1";

pub(crate) const B: &str = "10.0.0.2";

pub(crate) const C: &str = "10.0.0.3";

pub(crate) fn addr(id: &str) -> Ipv4Addr {
    id.parse().unwrap()
}

/// Builds a group of the gateways, whose keys are recorded and in use. The pairs still have
/// to tell each other, as after a join.
pub(crate) fn joined(ids: &[&str]) -> Vec<MlsState> {
    let mut gateways: Vec<MlsState> = ids.iter().map(|id| new_gateway(id)).collect();
    create_group(&mut gateways[0], 5).unwrap();
    for i in 1..gateways.len() {
        let (first, rest) = gateways.split_at_mut(i);
        let (invited, _) = rest.split_first_mut().unwrap();
        let (inviter, others) = first.split_first_mut().unwrap();
        let mut others: Vec<&mut MlsState> = others.iter_mut().collect();
        invite(inviter, invited, &mut others, 5);
    }
    for gateway in &mut gateways {
        gateway.reset_epochs(5).unwrap();
    }
    gateways
}

/// Turns a queued message into the form, in which izakaya hands it to a recipient
pub(crate) fn as_received(message: &OutgoingMessage, sender: &str) -> MlsMessageResp {
    MlsMessageResp {
        uuid: Uuid::new_v4(),
        group_id: message.group_id.clone(),
        epoch: message.epoch,
        message_type: message.message_type,
        sender: sender.to_string(),
        recipient: message.recipients[0].clone(),
        payload: message.payload.clone(),
        grants: message.grants.clone(),
        created_at: Utc::now(),
    }
}

pub(crate) fn current(state: &MlsState) -> EpochId {
    let keys = &state.network_keys[&5];
    (keys.generation, state.groups[&5].epoch().as_u64())
}

/// Kernel of the tests, which accepts every key, or refuses the outgoing ones
#[derive(Default)]
pub(crate) struct TestKernel {
    pub(crate) refuse_egress: bool,
}

impl PairKernel for TestKernel {
    fn apply(&mut self, _keys: &NetworkKeys) {}

    fn installed(
        &self,
        _keys: &NetworkKeys,
        _peer: Ipv4Addr,
        _id: EpochId,
        direction: CryptoDirection,
    ) -> bool {
        !(self.refuse_egress && direction == CryptoDirection::Egress)
    }
}

/// Runs a step of a test-gateway and returns the messages it sends
pub(crate) fn step_of(gateway: &mut TestGateway) -> Vec<PairMessage> {
    let step = step(&mut gateway.keys, &mut TestKernel::default(), 0);
    step.messages
        .into_iter()
        .map(|(kind, id, peers)| {
            mark_sent(&mut gateway.keys, kind, id, &peers);
            PairMessage {
                kind,
                generation: id.0,
                epoch: id.1,
                peers: peers.iter().map(|peer| peer.to_string()).collect(),
            }
        })
        .collect()
}

pub(crate) fn deliver_to(messages: Vec<PairMessage>, from: &TestGateway, to: &mut TestGateway) {
    let own = to.local_gateway_ip.to_string();
    for message in messages {
        take(&mut to.keys, &own, from.local_gateway_ip, &message);
    }
}

/// Removes the old incoming keys of a test-gateway, as if no packet arrives with them
pub(crate) fn idle(gateway: &mut TestGateway) {
    for _ in 0..2 {
        gateway.check_counters(&mut |_, _, _| Ok(Some(0)));
    }
}

pub(crate) fn arrived(gateway: &TestGateway, id: EpochId) -> bool {
    let pair = &gateway.keys.pairs[&gateway.conn.peer_gateway_ip];
    pair.target.is_none() && pair.egress == Some(id)
}

/// Moves the group of the gateways into a new epoch, which `silent` gateways don't follow
pub(crate) fn rotate(gateways: &mut [MlsState], silent: &[usize]) {
    let (committer, others) = gateways.split_first_mut().unwrap();
    let commit = stage_update(committer, 5).unwrap().unwrap();
    merge_pending(committer, 5).unwrap();
    committer.apply_commit(5, &CommitChange::default()).unwrap();
    for (i, gateway) in others.iter_mut().enumerate() {
        if silent.contains(&(i + 1)) {
            continue;
        }
        process(gateway, &commit);
        gateway.apply_commit(5, &CommitChange::default()).unwrap();
    }
}

pub(crate) fn process(gateway: &mut MlsState, message: &OutgoingMessage) {
    messages::process_message(
        gateway,
        message.message_type,
        &message.payload,
        &message.grants,
        &grant_key().verifying_key(),
    )
    .unwrap();
}

/// Lets all gateways, which are not `silent`, move their pairs and deliver their messages, and
/// returns the number of messages, which each gateway sent
pub(crate) fn drive_all(gateways: &mut [MlsState], silent: &[usize]) -> Vec<usize> {
    let mut sent = vec![0; gateways.len()];
    let mut outgoing = Vec::new();
    for (i, gateway) in gateways.iter_mut().enumerate() {
        if silent.contains(&i) {
            continue;
        }
        drive_pairs(gateway, 5, &mut TestKernel::default(), 0);
        let sender = gateway.client_id().unwrap().to_string();
        sent[i] = gateway.pending_out.len();
        outgoing.extend(gateway.pending_out.drain(..).map(|m| (sender.clone(), m)));
    }
    for (sender, message) in outgoing {
        for (i, gateway) in gateways.iter_mut().enumerate() {
            let id = gateway.client_id().unwrap().to_string();
            if silent.contains(&i) || !message.recipients.contains(&id) {
                continue;
            }
            handle_message(
                gateway,
                &as_received(&message, &sender),
                &grant_key().verifying_key(),
            )
            .unwrap();
        }
    }
    sent
}

/// Lets the first gateway commit a change and the others follow it, as the agent does
pub(crate) fn commit(gateways: &mut [MlsState], additions: Vec<Addition>, removals: &[String]) {
    let (committer, others) = gateways.split_first_mut().unwrap();
    let messages = match additions.is_empty() && removals.is_empty() {
        true => stage_update(committer, 5).unwrap().into_iter().collect(),
        false => stage_change(committer, 5, additions, removals).unwrap(),
    };
    let change = pending_change(committer, 5);
    merge_pending(committer, 5).unwrap();
    committer.apply_commit(5, &change).unwrap();

    for message in &messages {
        for gateway in others.iter_mut() {
            let id = gateway.client_id().unwrap().to_string();
            if !message.recipients.contains(&id) {
                continue;
            }
            let processed = messages::process_message(
                gateway,
                message.message_type,
                &message.payload,
                &message.grants,
                &grant_key().verifying_key(),
            )
            .unwrap();
            match processed {
                Processed::Advanced { change, .. } => {
                    gateway.apply_commit(5, &change).unwrap();
                }
                Processed::Joined { .. } => {
                    gateway.reset_epochs(5).unwrap();
                }
                _ => {}
            }
        }
    }
}

pub(crate) fn pair(gateway: &MlsState, peer: &str) -> PairKeys {
    gateway.network_keys[&5].pairs[&addr(peer)].clone()
}
