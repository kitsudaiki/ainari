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

//! MLS-based key-exchange between the gateways of a network.
//!
//! Every gateway is a MLS-client and every network has its own MLS-group, which contains the
//! gateways of all hosts with a VM of that network, which talk to each other over encrypted
//! routes. The gateways run themselves (see `agent`): each one subscribes at the izakaya to the
//! group of every network, which it serves, and the izakaya coordinates the changes of the groups
//! and their key-rotations.
//!
//! Who may be member of which group, is decided by hanami alone: a gateway is only added with a
//! membership-grant, which hanami signed for exactly its identity and signature-key. The committer
//! checks the grant before it adds the gateway, and every other member checks it again, before it
//! follows the commit. So neither a compromised izakaya nor a compromised gateway can bring a
//! gateway into a group, which hanami didn't allow.
//!
//! The group never protects any traffic itself: each gateway derives the keys of its IPsec
//! connections from the epochs of the group (see `keys`), so the keys rotate with every change of
//! the group and a removed gateway can't follow anymore. A new epoch is only recorded with the
//! change of the group. The two gateways of every pair then switch to it and retire the older
//! epochs on their own (see `pairs`), so both ends of a connection always have matching keys.

//!
//! The code is split by responsibility:
//!
//! - `agent`: the background-loop of the gateway, `subscriptions`: its subscriptions to the
//!   groups, `committer`: the changes, which it makes as committer of a group
//! - `state`: the persisted MLS-client, `identity`: its identity and the ids of the groups
//! - `group`, `commit`, `messages`: the groups, their changes and the messages between the
//!   gateways
//! - `network_keys`: the epochs and the pairs of every network, `keys`: the IPsec keys derived
//!   from them, `kernel`: installs them into the kernel
//! - `pairs`, `pair_messages`: the key-change of every pair with its peer, `status`: its state
//! - `izakaya`, `gateway`: the connection of the state to the izakaya and to the gateway
//!
//! Lock order: the MLS-state is always locked before the gateway-state.

pub mod agent;
pub mod commit;
pub mod committer;
pub mod gateway;
pub mod group;
pub mod identity;
pub mod izakaya;
pub mod kernel;
pub mod keys;
pub mod messages;
pub mod network_keys;
pub mod pair_messages;
pub mod pairs;
pub mod state;
pub mod status;
pub mod subscriptions;
#[cfg(test)]
pub(crate) mod test_support;

pub use gateway::{refresh_network_keys, restore_network_keys};
