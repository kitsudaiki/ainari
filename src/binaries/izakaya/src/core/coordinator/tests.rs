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

//! Tests of the coordination of the groups.

use super::*;
use serial_test::serial;
use uuid::Uuid;

use ainari_api_structs::mls_structs::*;

use crate::api::http_endpoints::mls_grant::store_mls_grant_internal_v1_0::store_grant;
use crate::api::http_endpoints::mls_group::finish_mls_operation_internal_v1_0::operation_done;
use crate::api::http_endpoints::mls_group::subscribe_mls_group_internal_v1_0::subscribe;
use crate::api::http_endpoints::mls_group::unsubscribe_mls_group_internal_v1_0::unsubscribe;
use crate::database::mls_message_table::{self, list_mls_messages};
use crate::database::mls_operation_table::STATUS_QUEUED;

const KEY_SEED: [u8; 32] = [3u8; 32];

fn key() -> SigningKey {
    SigningKey::from_bytes(&KEY_SEED)
}

/// Every test uses its own tenant, so the tests don't see each others groups
fn new_vni() -> u32 {
    (Uuid::new_v4().as_u128() % 100_000) as u32 + 1_000
}

fn grant(vni: u32, client_id: &str, action: MlsGrantAction, now: i64) -> MlsGrant {
    MlsGrant::sign(
        &MlsGrantPayload {
            vni,
            client_id: client_id.to_string(),
            signature_key: format!("key-of-{client_id}"),
            action,
            issued_at: now,
            expires_at: now + 600,
        },
        &key(),
    )
}

fn allow(vni: u32, client_id: &str, now: i64) {
    store_grant(
        &grant(vni, client_id, MlsGrantAction::Add, now),
        &key().verifying_key(),
        now,
    )
    .unwrap();
}

fn subscribe_as(vni: u32, client_id: &str, has_group: bool, now: i64) -> MlsSubscribeAction {
    let req = MlsSubscribeReq {
        client_id: client_id.to_string(),
        has_group,
    };
    subscribe(vni, &req, now).unwrap()
}

/// Takes all pending messages of a gateway of one type and acknowledges them.
fn take(client_id: &str, message_type: MlsMessageType) -> Vec<String> {
    let ctx = system_context();
    list_mls_messages(client_id)
        .unwrap()
        .into_iter()
        .filter(|message| message.message_type == message_type.to_string())
        .map(|message| {
            mls_message_table::delete_mls_message(client_id, &message.uuid, &ctx)
                .ok()
                .unwrap();
            message.payload
        })
        .collect()
}

fn take_operations(client_id: &str) -> Vec<MlsOperation> {
    take(client_id, MlsMessageType::Operation)
        .iter()
        .map(|payload| serde_json::from_str(payload).unwrap())
        .collect()
}

fn done(vni: u32, op: &MlsOperation, committer: &str, epoch: u64, members: &[&str], now: i64) {
    let req = MlsOperationDoneReq {
        op_uuid: op.op_uuid,
        client_id: committer.to_string(),
        success: true,
        epoch,
        members: members.iter().map(|it| it.to_string()).collect(),
        reason: None,
        refused: Vec::new(),
    };
    operation_done(vni, &req, now).unwrap();
}

/// Lists the gateways, which an operation adds
fn added(op: &MlsOperation) -> Vec<&str> {
    op.adds.iter().map(|add| add.client_id.as_str()).collect()
}

fn client(name: &str) -> String {
    format!("{name}-{}", Uuid::new_v4())
}

#[test]
#[serial]
fn a_gateway_without_grant_can_not_subscribe() {
    let vni = new_vni();
    let now = now_secs();
    let req = MlsSubscribeReq {
        client_id: client("a"),
        has_group: false,
    };
    assert!(matches!(
        subscribe(vni, &req, now),
        Err(ErrorResponse::Forbidden(_))
    ));
}

#[test]
#[serial]
fn a_forged_grant_is_refused() {
    let now = now_secs();
    let forged = MlsGrant::sign(
        &MlsGrantPayload {
            vni: 1,
            client_id: "a".to_string(),
            signature_key: "k".to_string(),
            action: MlsGrantAction::Add,
            issued_at: now,
            expires_at: now + 600,
        },
        &SigningKey::from_bytes(&[9u8; 32]),
    );
    assert!(matches!(
        store_grant(&forged, &key().verifying_key(), now),
        Err(ErrorResponse::BadRequest(_))
    ));
}

#[test]
#[serial]
fn a_gateway_is_added_by_the_committer() {
    let vni = new_vni();
    let now = now_secs();
    let (a, b) = (client("a"), client("b"));
    allow(vni, &a, now);
    allow(vni, &b, now);

    // the first gateway creates the group, the second one is added by it
    assert_eq!(
        subscribe_as(vni, &a, false, now),
        MlsSubscribeAction::Create
    );
    assert_eq!(
        subscribe_as(vni, &b, false, now),
        MlsSubscribeAction::Joining
    );
    let ops = take_operations(&a);
    assert_eq!(ops.len(), 1);
    assert_eq!(added(&ops[0]), vec![b.as_str()]);
    assert!(ops[0].removes.is_empty());
    assert!(!ops[0].update);
    assert!(take_operations(&b).is_empty());

    // the committer reports the new epoch, nobody waits for the members afterwards
    done(vni, &ops[0], &a, 1, &[&a, &b], now);
    let group = load_group(vni).unwrap().unwrap();
    assert_eq!(group.epoch, 1);
    assert_eq!(group.members.len(), 2);
    assert!(open_operations(vni).unwrap().is_empty());

    // a member, which still holds the group, is just a member
    assert_eq!(subscribe_as(vni, &b, true, now), MlsSubscribeAction::Member);
}

#[test]
#[serial]
fn the_next_change_follows_right_after_the_last_one() {
    let vni = new_vni();
    let now = now_secs();
    let (a, b, c) = (client("a"), client("b"), client("c"));
    for gateway in [&a, &b, &c] {
        allow(vni, gateway, now);
    }
    subscribe_as(vni, &a, false, now);
    subscribe_as(vni, &b, false, now);
    subscribe_as(vni, &c, false, now);

    // only the addition of b is in flight, c waits
    let ops = take_operations(&a);
    assert_eq!(ops.len(), 1);
    assert_eq!(added(&ops[0]), vec![b.as_str()]);

    // with the report of the committer, the addition of c is handed out
    done(vni, &ops[0], &a, 1, &[&a, &b], now);
    let ops = take_operations(&a);
    assert_eq!(ops.len(), 1);
    assert_eq!(added(&ops[0]), vec![c.as_str()]);
}

#[test]
#[serial]
fn a_revoked_gateway_is_removed() {
    let vni = new_vni();
    let now = now_secs();
    let (a, b) = (client("a"), client("b"));
    allow(vni, &a, now);
    allow(vni, &b, now);
    subscribe_as(vni, &a, false, now);
    subscribe_as(vni, &b, false, now);
    let ops = take_operations(&a);
    done(vni, &ops[0], &a, 1, &[&a, &b], now);

    store_grant(
        &grant(vni, &b, MlsGrantAction::Remove, now),
        &key().verifying_key(),
        now,
    )
    .unwrap();
    let ops = take_operations(&a);
    let removal = ops.iter().find(|op| !op.removes.is_empty()).unwrap();
    assert_eq!(removal.removes, vec![b.clone()]);

    // without grant, b can't join again
    let req = MlsSubscribeReq {
        client_id: b.clone(),
        has_group: false,
    };
    assert!(matches!(
        subscribe(vni, &req, now),
        Err(ErrorResponse::Forbidden(_))
    ));
}

/// Reports, that the committer refused an operation, because it has no group
fn refuse_without_group(vni: u32, op: &MlsOperation, committer: &str, now: i64) {
    let req = MlsOperationDoneReq {
        op_uuid: op.op_uuid,
        client_id: committer.to_string(),
        success: false,
        epoch: 0,
        members: Vec::new(),
        reason: Some(format!("this gateway has no MLS-group for tenant {vni}")),
        refused: Vec::new(),
    };
    operation_done(vni, &req, now).unwrap();
}

/// Builds a group of the given gateways, whose first one is the committer
fn joined_group(vni: u32, gateways: &[&String], now: i64) {
    for gateway in gateways {
        allow(vni, gateway, now);
        subscribe_as(vni, gateway, false, now);
    }
    for epoch in 1..gateways.len() as u64 {
        let ops = take_operations(gateways[0]);
        let members = &gateways[..=epoch as usize];
        let members: Vec<&str> = members.iter().map(|it| it.as_str()).collect();
        done(vni, &ops[0], gateways[0], epoch, &members, now);
    }
}

#[test]
#[serial]
fn a_group_is_created_again_after_its_last_members_left_at_once() {
    let vni = new_vni();
    let now = now_secs();
    let (a, b) = (client("a"), client("b"));
    joined_group(vni, &[&a, &b], now);

    // both gateways drop the group at the same time, so the committer a refuses the removal
    // of b, because it already left the group itself
    unsubscribe(vni, &b, now).unwrap();
    let ops = take_operations(&a);
    assert_eq!(ops[0].removes, vec![b.clone()]);
    refuse_without_group(vni, &ops[0], &a, now);
    unsubscribe(vni, &a, now).unwrap();
    assert!(load_group(vni).unwrap().is_none());

    // later both need the encryption again: the first creates the group, the second joins
    assert_eq!(
        subscribe_as(vni, &b, false, now),
        MlsSubscribeAction::Create
    );
    assert_eq!(
        subscribe_as(vni, &a, false, now),
        MlsSubscribeAction::Joining
    );
    let ops = take_operations(&b);
    assert_eq!(added(&ops[0]), vec![a.as_str()]);
}

#[test]
#[serial]
fn a_committer_without_group_is_replaced_by_a_member_with_it() {
    let vni = new_vni();
    let now = now_secs();
    let (a, b, c) = (client("a"), client("b"), client("c"));
    joined_group(vni, &[&a, &b, &c], now);

    // a lost the group, so it refuses the removal of b, while c still holds the group
    unsubscribe(vni, &b, now).unwrap();
    let ops = take_operations(&a);
    refuse_without_group(vni, &ops[0], &a, now);

    let group = load_group(vni).unwrap().unwrap();
    assert_eq!(group.members, vec![c.clone()]);
    assert_eq!(group.committer, c);

    // a joins again over c
    assert_eq!(
        subscribe_as(vni, &a, false, now),
        MlsSubscribeAction::Joining
    );
    let ops = take_operations(&c);
    assert_eq!(added(&ops[0]), vec![a.as_str()]);
}

#[test]
#[serial]
fn the_keys_are_rotated_regularly() {
    let vni = new_vni();
    let now = now_secs();
    let (a, b) = (client("a"), client("b"));
    allow(vni, &a, now);
    allow(vni, &b, now);
    subscribe_as(vni, &a, false, now);
    subscribe_as(vni, &b, false, now);
    let ops = take_operations(&a);
    done(vni, &ops[0], &a, 1, &[&a, &b], now);
    tick(now + 1, 60).unwrap();
    assert!(take_operations(&a).is_empty());

    // both gateways keep polling in the meantime
    touch(&a, now + 60).unwrap();
    touch(&b, now + 60).unwrap();
    tick(now + 60, 60).unwrap();
    let ops = take_operations(&a);
    assert_eq!(ops.len(), 1);
    assert!(ops[0].update);
}

#[test]
#[serial]
fn the_last_member_takes_the_group_with_it() {
    let vni = new_vni();
    let now = now_secs();
    let a = client("a");
    allow(vni, &a, now);
    subscribe_as(vni, &a, false, now);

    unsubscribe(vni, &a, now).unwrap();
    assert!(load_group(vni).unwrap().is_none());

    // the next gateway starts with a new group
    assert_eq!(subscribe_as(vni, &a, true, now), MlsSubscribeAction::Create);
}

#[test]
#[serial]
fn a_silent_member_is_removed() {
    let vni = new_vni();
    let now = now_secs();
    let (a, b) = (client("a"), client("b"));
    allow(vni, &a, now);
    allow(vni, &b, now);
    subscribe_as(vni, &a, false, now);
    subscribe_as(vni, &b, false, now);
    let ops = take_operations(&a);
    done(vni, &ops[0], &a, 1, &[&a, &b], now);
    take_operations(&a);

    // b stays silent, a keeps polling
    let later = now + 200;
    touch(&a, later).unwrap();

    // an izakaya, which only runs since a moment, can't tell, who is silent
    remove_dead_members(later, 120, later - 10).unwrap();
    assert!(!take_operations(&a).iter().any(|op| !op.removes.is_empty()));

    remove_dead_members(later, 120, now).unwrap();
    tick(later + STALL_TIMEOUT + 1, 3600).unwrap();
    let ops = take_operations(&a);
    let removal = ops
        .iter()
        .find(|op| !op.removes.is_empty())
        .expect("b is removed");
    assert_eq!(removal.removes, vec![b.clone()]);
}

#[test]
#[serial]
fn a_group_without_living_member_is_left_alone() {
    let vni = new_vni();
    let now = now_secs();
    let (a, b) = (client("a"), client("b"));
    allow(vni, &a, now);
    allow(vni, &b, now);
    subscribe_as(vni, &a, false, now);
    subscribe_as(vni, &b, false, now);
    let ops = take_operations(&a);
    done(vni, &ops[0], &a, 1, &[&a, &b], now);

    remove_dead_members(now + 200, 120, now - 200).unwrap();
    let group = load_group(vni).unwrap().unwrap();
    assert_eq!(group.members.len(), 2);
    assert!(
        !open_operations(vni)
            .unwrap()
            .iter()
            .any(|op| op.kind == MlsOperationKind::Remove.to_string())
    );
}

#[test]
#[serial]
fn a_group_without_living_members_starts_again() {
    let vni = new_vni();
    let now = now_secs();
    let (a, b, c) = (client("a"), client("b"), client("c"));
    allow(vni, &a, now);
    allow(vni, &b, now);
    allow(vni, &c, now);
    subscribe_as(vni, &a, false, now);
    subscribe_as(vni, &b, false, now);
    let ops = take_operations(&a);
    done(vni, &ops[0], &a, 1, &[&a, &b], now);

    // a and b are gone, while c comes along
    let later = now + LIVENESS_TIMEOUT + 10;
    assert_eq!(
        subscribe_as(vni, &c, false, later),
        MlsSubscribeAction::Create
    );
    let group = load_group(vni).unwrap().unwrap();
    assert_eq!(group.members, vec![c.clone()]);
    assert_eq!(group.committer, c);
    assert!(open_operations(vni).unwrap().is_empty());
}

#[test]
#[serial]
fn a_gateway_joins_while_a_member_is_alive() {
    let vni = new_vni();
    let now = now_secs();
    let (a, b, c) = (client("a"), client("b"), client("c"));
    allow(vni, &a, now);
    allow(vni, &b, now);
    allow(vni, &c, now);
    subscribe_as(vni, &a, false, now);
    subscribe_as(vni, &b, false, now);
    let ops = take_operations(&a);
    done(vni, &ops[0], &a, 1, &[&a, &b], now);

    // b is gone, but a still polls
    let later = now + LIVENESS_TIMEOUT + 10;
    touch(&a, later).unwrap();
    assert_eq!(
        subscribe_as(vni, &c, false, later),
        MlsSubscribeAction::Joining
    );
}

/// Reports, that the committer left all gateways of an operation out
fn refuse_gateways(vni: u32, op: &MlsOperation, committer: &str, epoch: u64, now: i64) {
    let req = MlsOperationDoneReq {
        op_uuid: op.op_uuid,
        client_id: committer.to_string(),
        success: false,
        epoch,
        members: vec![committer.to_string()],
        reason: Some("every change was refused".to_string()),
        refused: op
            .adds
            .iter()
            .map(|add| MlsOperationRefusal {
                client_id: add.client_id.clone(),
                reason: "no key-package".to_string(),
            })
            .collect(),
    };
    operation_done(vni, &req, now).unwrap();
}

#[test]
#[serial]
fn a_refused_gateway_is_queued_again_until_it_ran_out_of_retries() {
    let vni = new_vni();
    let now = now_secs();
    let (a, b) = (client("a"), client("b"));
    allow(vni, &a, now);
    allow(vni, &b, now);
    subscribe_as(vni, &a, false, now);
    subscribe_as(vni, &b, false, now);

    for _ in 0..MAX_OPERATION_RETRIES {
        let ops = take_operations(&a);
        assert_eq!(ops.len(), 1);
        assert_eq!(added(&ops[0]), vec![b.as_str()]);
        refuse_gateways(vni, &ops[0], &a, 0, now);
    }
    let ops = take_operations(&a);
    assert_eq!(ops.len(), 1);
    refuse_gateways(vni, &ops[0], &a, 0, now);

    // the group is unchanged, and b is only added again with its next subscription
    assert!(take_operations(&a).is_empty());
    assert_eq!(load_group(vni).unwrap().unwrap().members, vec![a.clone()]);
    subscribe_as(vni, &b, false, now);
    assert_eq!(added(&take_operations(&a)[0]), vec![b.as_str()]);
}

/// Sets the commit-delay for a test and resets it, when the test ends
struct CommitDelay;

impl CommitDelay {
    fn set(delay: i64) -> Self {
        COMMIT_DELAY.store(delay, Ordering::Relaxed);
        CommitDelay
    }
}

impl Drop for CommitDelay {
    fn drop(&mut self) {
        COMMIT_DELAY.store(0, Ordering::Relaxed);
    }
}

/// Lets `a` create a group and `others` subscribe at the same time, and returns the batch,
/// which is handed to `a` after the commit-delay
fn subscribe_together(vni: u32, a: &str, others: &[&String], now: i64) -> MlsOperation {
    allow(vni, a, now);
    subscribe_as(vni, a, false, now);
    for gateway in others {
        allow(vni, gateway, now);
        assert_eq!(
            subscribe_as(vni, gateway, false, now),
            MlsSubscribeAction::Joining
        );
    }
    // the additions wait for each other
    tick(now, 3600).unwrap();
    assert!(take_operations(a).is_empty());

    tick(now + 1, 3600).unwrap();
    let mut ops = take_operations(a);
    assert_eq!(ops.len(), 1);
    ops.remove(0)
}

#[test]
#[serial]
fn gateways_which_subscribe_together_are_added_with_one_commit() {
    let _delay = CommitDelay::set(DEFAULT_COMMIT_DELAY);
    let vni = new_vni();
    let now = now_secs();
    let (a, b, c, d) = (client("a"), client("b"), client("c"), client("d"));

    let op = subscribe_together(vni, &a, &[&b, &c, &d], now);
    let mut expected = vec![b.as_str(), c.as_str(), d.as_str()];
    expected.sort();
    let mut adds = added(&op);
    adds.sort();
    assert_eq!(adds, expected);
    assert!(op.removes.is_empty());
    assert!(!op.update);

    // the whole batch is finished with one report
    done(vni, &op, &a, 1, &[&a, &b, &c, &d], now + 1);
    assert!(open_operations(vni).unwrap().is_empty());
    assert_eq!(load_group(vni).unwrap().unwrap().members.len(), 4);
}

#[test]
#[serial]
fn a_stalled_batch_is_retried_as_a_whole() {
    let _delay = CommitDelay::set(DEFAULT_COMMIT_DELAY);
    let vni = new_vni();
    let now = now_secs();
    let (a, b, c) = (client("a"), client("b"), client("c"));
    let op = subscribe_together(vni, &a, &[&b, &c], now);

    tick(now + 1 + STALL_TIMEOUT, 3600).unwrap();
    assert!(take_operations(&a).is_empty());
    tick(now + 2 + STALL_TIMEOUT, 3600).unwrap();
    let retried = take_operations(&a);
    assert_eq!(retried.len(), 1);
    assert_eq!(retried[0].op_uuid, op.op_uuid);
    assert_eq!(retried[0].adds.len(), 2);
}

#[test]
#[serial]
fn only_the_refused_gateway_of_a_batch_is_queued_again() {
    let _delay = CommitDelay::set(DEFAULT_COMMIT_DELAY);
    let vni = new_vni();
    let now = now_secs();
    let (a, b, c) = (client("a"), client("b"), client("c"));
    let op = subscribe_together(vni, &a, &[&b, &c], now);

    let req = MlsOperationDoneReq {
        op_uuid: op.op_uuid,
        client_id: a.clone(),
        success: true,
        epoch: 1,
        members: vec![a.clone(), b.clone()],
        reason: None,
        refused: vec![MlsOperationRefusal {
            client_id: c.clone(),
            reason: "no key-package".to_string(),
        }],
    };
    operation_done(vni, &req, now + 1).unwrap();

    // without a round to wait for, c is handed out again right away
    let open = open_operations(vni).unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].client_id.as_deref(), Some(c.as_str()));
    assert_eq!(open[0].retries, 1);
    let ops = take_operations(&a);
    assert_eq!(ops.len(), 1);
    assert_eq!(added(&ops[0]), vec![c.as_str()]);
}

#[test]
#[serial]
fn a_key_rotation_is_never_combined_with_additions() {
    let vni = new_vni();
    let now = now_secs();
    let (a, b, c) = (client("a"), client("b"), client("c"));
    joined_group(vni, &[&a, &b], now);
    let _delay = CommitDelay::set(DEFAULT_COMMIT_DELAY);

    allow(vni, &c, now);
    subscribe_as(vni, &c, false, now);
    // the rotation is queued, when the addition of c is due
    tick(now + 1, 0).unwrap();
    let ops = take_operations(&a);
    assert_eq!(ops.len(), 1);
    assert_eq!(added(&ops[0]), vec![c.as_str()]);
    assert!(!ops[0].update);

    // the rotation waits for its own turn
    let update = MlsOperationKind::Update.to_string();
    let open = open_operations(vni).unwrap();
    assert!(
        open.iter()
            .any(|op| op.kind == update && op.status == STATUS_QUEUED)
    );
}

#[test]
#[serial]
fn a_key_rotation_is_covered_by_a_removal() {
    let vni = new_vni();
    let now = now_secs();
    let (a, b, c) = (client("a"), client("b"), client("c"));
    joined_group(vni, &[&a, &b, &c], now);
    let _delay = CommitDelay::set(DEFAULT_COMMIT_DELAY);

    unsubscribe(vni, &c, now).unwrap();
    assert!(take_operations(&a).is_empty());
    tick(now + 1, 0).unwrap();
    let ops = take_operations(&a);
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].removes, vec![c.clone()]);
    assert!(!ops[0].update);

    // the removal renews all keys, so the rotation is done with it
    done(vni, &ops[0], &a, 3, &[&a, &b], now + 1);
    assert!(open_operations(vni).unwrap().is_empty());
}

#[test]
#[serial]
fn the_time_comes_from_the_database() {
    let database = db_handle::database_time().unwrap();
    assert!((database - Utc::now().timestamp()).abs() <= 2);
    assert!((now_secs() - database).abs() <= 2);
}
