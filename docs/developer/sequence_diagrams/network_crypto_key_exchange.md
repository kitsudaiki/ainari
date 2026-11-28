# Key-exchange of the network-encryption

The traffic between the virtual machines of a network, which run on different hosts, is encrypted
with IPsec. The keys of these encrypted routes are never sent over the wire: all torii of a
network form an [MLS](https://www.rfc-editor.org/rfc/rfc9420.html) group, and each torii derives
the keys of its IPsec Security Associations (SAs) from the secret of the current epoch of the
group.

A network, which was created with `disable_encryption: true`, skips all of this: the routes
between the hosts of the network are not encrypted, hanami signs no grants for it, so its torii
never subscribe to a group of it.

## Components

| Component | Role |
|---|---|
| **hanami** | Decides, which torii may be member of which group. It signs a membership-grant for every host, which gets a VM of a network, refreshes all grants every 5 minutes and revokes a grant, when the last VM of the network is gone from the host. When a host registers itself again after a restart, it pins the new identity of its torii and changes the routes of the other gateways to the new address of the host. |
| **izakaya** | Shares the key-packages, delivers the MLS-messages and coordinates the groups: one queue of changes per group, which it hands to one committer per group in batches, and the regular key-rotation. It removes members, which stay silent. It has no part in the switch of the keys. Its state lives in its database, so several instances can run next to each other. |
| **torii** (VM-host) | MLS-client with its underlay-address as identity. A background-loop (the *agent*) subscribes to the groups, makes the changes as committer, processes the messages and installs the derived keys in the kernel. It switches the keys towards every other torii on its own, only coordinated with that torii. |
| **torii** (edge) | Never member of a group, the routes from and to it are never encrypted. |

```mermaid
flowchart LR
    hanami["hanami<br/>signs membership-grants"]
    izakaya["izakaya<br/>key-packages, messages,<br/>queue, committer"]
    db[("database of izakaya")]
    toriiA["torii A<br/>agent + xfrm"]
    toriiB["torii B<br/>agent + xfrm"]
    edge["torii (edge)<br/>no MLS"]

    hanami -- "identity (pinning)" --> toriiA
    hanami -- "identity (pinning)" --> toriiB
    hanami -- "signed grants" --> izakaya
    izakaya --- db
    toriiA <-->|poll every second| izakaya
    toriiB <-->|poll every second| izakaya
    toriiA <-.->|IPsec ESP| toriiB
    toriiA -.-|unencrypted| edge
    toriiB -.-|unencrypted| edge
```

All instances of izakaya compare times, which another instance wrote, like the last contact of a
torii or the start of an operation. So they don't use their own clocks, but the clock of the
database-server, which they share. Their own clocks may differ, for example after a virtual
machine was paused.

### Trust

hanami is the only one, who decides about the membership. Its public key is part of the configs of
izakaya and of every torii. A grant names the vni, the identity of the torii and its MLS
signature-key, which hanami pins with the first grant of a host:

- izakaya only accepts grants with a valid signature of hanami and only queues the addition of a
  torii, which has a grant.
- The committer checks the grant again, before it adds a torii, and only claims a key-package, which
  has exactly the granted signature-key.
- Every other member checks every addition of a commit against the grants, which are attached to
  the commit, before it follows the commit.

So neither a compromised izakaya nor a compromised torii can bring a torii into a group, which
hanami didn't allow.

The switch of the keys is coordinated by the two torii of a pair with MLS-messages, which are
encrypted and signed within the group (see [Key-change of a pair](#key-change-of-a-pair)). Which
pairs move to a new epoch, every member reads from the commit itself. So izakaya can hold messages
back, which only delays the switch, but it can't make a torii switch or remove keys too early.
It can still remove a torii, because only izakaya sees, which torii is silent; every member logs
every removal of a peer with a warning, which names the committer.

The pinned identity is only replaced, when the sakura-host registers itself again, which needs the
registration-key, which already decides, which hosts take part at all (see
[Restart of a sakura-host](#restart-of-a-sakura-host)).

The internal endpoints of torii are authorized by the internal API-key alone, without the token of
a user, because hanami calls them also from its background-jobs, like the restore of a host. They
are only reachable over the internal port of torii.

## Placing a VM: identity-pinning and grant

hanami grants the membership with every VM of an encrypted network, which it places on a host,
before it creates the encrypted routes of the VM. It doesn't wait for the group: the torii join it
on their own.

```mermaid
sequenceDiagram
    participant H as hanami
    participant DB as hanami database
    participant TB as torii B (new host)
    participant I as izakaya

    H->>TB: register TAP and the local route of the new VM
    TB->>TB: rule "from VM iif TAP lookup tenant-table",<br/>everything else of the TAP is unreachable
    TB->>TB: exempt the TAP from the reverse-path filter<br/>(the other interfaces keep theirs)
    H->>TB: GET /network_interface/underlay/internal
    TB-->>H: underlay-address 10.0.0.2
    H->>TB: GET /network_crypto/mls/identity/internal
    TB->>TB: create the MLS-identity with the first call<br/>(underlay-address + signature-key)
    TB-->>H: client_id 10.0.0.2, signature_key K
    H->>H: client_id has to be the underlay-address of the host
    H->>DB: pin K for the host, if no key is pinned yet
    DB-->>H: pinned key
    alt pinned key is K
        H->>H: sign grant (vni, 10.0.0.2, K, add, expires_at)
        H->>I: POST /mls_grant/internal
        I->>I: verify the signature of hanami and store the grant
        I-->>H: 204
    else pinned key is another one
        H-->>H: 409 Conflict, the host gets no grant anymore,<br/>until it is registered again
    end
    H->>TB: encrypted routes towards the other hosts of the network
    H->>H: encrypted routes on the other hosts towards the new VM
```

### A further VM on the same host

The membership belongs to the host and the network, not to a single VM. A further VM of the
network on a host, whose torii is already member of the group, changes nothing for the group:

```mermaid
sequenceDiagram
    participant H as hanami
    participant I as izakaya
    participant T as torii (already member)

    H->>I: grant (vni, T, pinned key, add)
    I->>I: replace the stored grant, no operation is queued
    H->>T: TAP and routes of the new VM
    T->>T: derive the SAs of the new connections<br/>from the epochs of their pairs
    T->>I: subscribe(vni, has_group = true)
    I-->>T: member
    Note over H,T: no commit, no new epoch, no key-change
```

### Refresh of the grants

Every instance of hanami refreshes all grants regularly from its own database: every host with a
VM of a network and a pinned signature-key gets its grant again. This keeps the grants of the
hosts, which keep their VMs, from expiring and brings them back, if izakaya lost them. The torii
is not asked for its key here, the pinned key is used.

```mermaid
sequenceDiagram
    participant H as hanami
    participant DB as hanami database
    participant M as miko
    participant I as izakaya

    loop every mls_grant_refresh_interval (5 minutes)
        H->>M: GET /v1alpha/endpoints
        H->>DB: all addresses: pairs of host and vni,<br/>without the networks with disabled encryption
        loop every pair, whose host has a pinned key and is not the edge
            H->>H: sign grant (vni, host, pinned key, add, expires_at)
            H->>I: POST /mls_grant/internal
            I->>I: replace the stored grant of the host and vni
        end
    end
```

## Background-loop of the torii

Every second the agent of a torii runs through the same steps. Which groups it subscribes to only
depends on its own routes: the network has a VM behind this torii and an encrypted route towards
another host.

```mermaid
sequenceDiagram
    participant A as agent of torii
    participant K as kernel (xfrm)
    participant I as izakaya

    loop every second
        A->>K: read the packet counters of the retiring incoming SAs,<br/>remove the ones, whose counter stood still

        opt every 30 seconds
            A->>I: GET /key_package/{client_id}/count/internal
            alt less than 5 left
                A->>A: create 10 key-packages, persist their private parts
                A->>I: POST /key_package/internal
                I->>I: validate the key-packages (signature, lifetime, identity)
            end
        end

        A->>A: networks = local VM and encrypted route in the same vni
        loop every network, which was never subscribed or not within the last 30 seconds
            A->>I: POST /mls_group/{vni}/subscribe/internal (has_group)
            I-->>A: create | joining | member, or 403 without grant
        end
        loop every held group, whose network is gone from this torii
            A->>I: POST /mls_group/{vni}/unsubscribe/internal
            A->>A: delete the group and its key material
            A->>K: remove all SAs and policies of the network
        end

        A->>I: GET /mls_message/{client_id}/internal (counts as sign of life)
        loop every message, in the order of group and epoch
            A->>A: process welcome, commit, operation or application message
            A->>I: DELETE /mls_message/{client_id}/{uuid}/internal
        end

        A->>K: move every pair with a target one step on
        A->>I: POST /mls_message/internal: queued Ready and Switched,<br/>which izakaya didn't accept yet
    end
```

An application message of an epoch, which the group didn't reach yet, because its commit is not
processed yet, waits in the state of the torii and is processed after the next commit.

## Creating a group and adding a second torii

The first torii, which subscribes to a network, creates its group and becomes its committer. Every
further torii is added by the committer. Both get grants from hanami, when their VMs are placed.

```mermaid
sequenceDiagram
    participant TA as torii A (committer)
    participant I as izakaya
    participant TB as torii B

    Note over TA,TB: both have a grant and an encrypted route of vni 5

    TA->>I: subscribe(vni 5, has_group = false)
    I->>I: grant of A exists, no group yet:<br/>group {committer A, epoch 0, members [A]}
    I-->>TA: create
    TA->>TA: create the group with epoch 0, no pairs yet

    TB->>I: subscribe(vni 5, has_group = false)
    I->>I: grant of B exists and is not expired:<br/>queue operation add(B, grant)
    I-->>TB: joining
    I->>I: wait commit_delay (1 second) for further changes
    I->>TA: operation {adds: [B with grant]} to the committer

    TA->>TA: verify the grant: signature of hanami, action add,<br/>vni 5, client B, not expired
    TA->>I: POST /key_package/B/claim/internal (signature_key K)
    I-->>TA: newest key-package of B with key K
    TA->>TA: validate the key-package: identity B, key K
    TA->>TA: stage one commit with all additions
    TA->>I: send welcome to B
    TA->>TA: merge: epoch 1, new pair with B,<br/>target epoch 1
    TA->>I: POST /mls_group/5/operation/internal<br/>(success, epoch 1, members [A, B], refused [])
    I->>I: group {epoch 1, members [A, B]},<br/>hand out the next queued change

    TB->>TB: welcome: join the group, pair with A:<br/>epoch 1 is the only epoch, already in use
    Note over TA,TB: key-change of the pair to epoch 1, see below
```

All additions and removals, which are queued within `commit_delay`, are handed to the committer
as one batch and are made with one commit. An addition, which the committer can't verify, is left
out and reported as `refused`, the others go on; izakaya queues the refused one again, until it
ran out of retries. A key-rotation is never part of a batch with additions or removals.

## Adding a torii to a group with more members

With more members, the commit of the addition goes to all other members. The grants of the new
torii are attached to it, so every member can check, that hanami allowed every addition.

An addition only moves the pairs with the new torii to the new epoch. The pairs between the other
members keep their epochs, so the new torii can't derive the keys of their traffic.

```mermaid
sequenceDiagram
    participant TA as torii A (committer)
    participant I as izakaya
    participant TB as torii B (member)
    participant TC as torii C (new)

    I->>TA: operation {adds: [C with grant]}
    TA->>TA: verify the grant, claim and validate the key-package of C
    TA->>TA: stage one commit with all additions
    TA->>I: send commit to B, with the grant of C attached
    TA->>I: send welcome to C
    TA->>TA: merge: epoch 2, pair A-C gets target epoch 2,<br/>pair A-B stays with its epoch
    TA->>I: operation done (epoch 2, members [A, B, C])

    TB->>I: poll
    I-->>TB: commit(epoch 2, grant of C)
    TB->>TB: check every addition against the grants:<br/>signature of hanami, vni, identity and signature-key
    alt covered by the grants
        TB->>TB: merge: epoch 2, pair B-C gets target epoch 2,<br/>pair B-A stays with its epoch
    else an addition without valid grant
        TB->>TB: refuse the commit, stay in epoch 1
        Note over TB: never tells C, that it is ready,<br/>so C never sends to B with epoch 2
    end

    TC->>I: poll
    I-->>TC: welcome
    TC->>TC: join the group, pairs with A and B:<br/>epoch 2 is the only epoch
    Note over TA,TC: key-change of the pairs A-C and B-C
```

A torii, which lost its group and is added again, is removed and added in the same commit. This
only moves its own pairs as well.

## Key-change of a pair

A *pair* is a torii and one other torii in one network. All connections between the VMs of the
two torii change their keys together. Every pair has its own epoch for its outgoing traffic, which
can lag behind the epoch of the group, and a *target*, the epoch it moves to. The epochs of a pair
are counted per *generation*: every group of the network, which a torii creates or joins, is a new
generation, so the epochs of a newer group are always newer.

Which pairs get a new epoch as target, every member reads from the commit:

| Commit | Pairs, which move |
|---|---|
| adds or re-adds a torii | only the pairs with the added torii |
| removes a torii for good | all pairs, so the removed torii knows no key in use anymore |
| neither (key-rotation) | all pairs |

The two torii of a pair tell each other with two MLS application messages, how far they are:
`Ready` (I can receive with the target) and `Switched` (I send with the target). The messages
are encrypted and signed within the group, so izakaya only delivers them. A message only counts
for the pair with its sender and only, if it names the receiver and is newer than the last one.
It names the epoch only within the group, because every torii counts its generations on its own;
the receiver can only decrypt it with its current group, so it belongs to its current generation.
Every pair moves on its own and only waits for its own kernel and for its peer:

```mermaid
sequenceDiagram
    participant KA as kernel A
    participant TA as torii A
    participant I as izakaya
    participant TB as torii B
    participant KB as kernel B

    Note over TA,TB: both processed the commit into epoch E:<br/>pair A-B has target E, sends with E-1

    rect rgb(235, 245, 255)
    Note over TA,TB: ingress
    TA->>KA: add incoming SAs of epoch E
    TB->>KB: add incoming SAs of epoch E
    TA->>I: Ready(E) for B
    TB->>I: Ready(E) for A
    end

    rect rgb(235, 255, 235)
    Note over TA,TB: egress, once the peer is ready
    I-->>TA: Ready(E) of B
    TA->>KA: add outgoing SA of epoch E
    TA->>KA: ip xfrm policy update: pin the policy to the SPI of epoch E (atomic)
    TA->>KA: delete outgoing SA of epoch E-1
    TA->>I: Switched(E) for B
    I-->>TB: Ready(E) of A
    TB->>KB: add outgoing SA of epoch E, switch the policy, delete the old one
    TB->>I: Switched(E) for A
    end

    rect rgb(255, 235, 235)
    Note over TA,TB: old ingress, once the peer switched
    I-->>TA: Switched(E) of B
    TA->>KA: read the packet counters of the incoming SAs of epoch E-1
    loop every second
        TA->>KA: counter stood still since the last read? delete the SA
    end
    I-->>TB: Switched(E) of A
    TB->>KB: the same with its incoming SAs of epoch E-1
    end

    Note over TA,TB: the pair arrived at epoch E
```

- Without a `Switched` of the peer, old incoming keys are never removed. The peer only sends it
  after its policies point at the new keys, so every packet with the old keys was handed to its
  kernel before. An old incoming SA is kept as long as packets arrive with it.
- A step, which fails, is tried again with the next loop; the reason is shown in the status of
  the pair. A pair, which doesn't move, keeps its old keys, which still work on both ends.
- A new target, while a pair is still moving, replaces the old one. The incoming keys of the old
  target stay, until the peer switched to something newer.
- `Ready` and `Switched` wait in the state of the torii, until izakaya accepted them, and the
  torii remembers, what it already sent. So a restart sends nothing twice and loses nothing.

The state of all pairs is shown by `GET /v1alpha/network_crypto/mls/status/internal` of the torii:
per pair the epochs of its outgoing and incoming keys, its target, the step it waits for
(`ingress`, `peer_ready`, `egress`, `peer_switched`, `old_ingress_in_use` or `nothing`) and its
last error.

### Key-derivation

All members export the same base secret from an epoch, as soon as they enter it, and derive the
SA of every direction of every VM-pair from it. The receiving torii derives exactly the key, which
the sending torii uses.

```mermaid
flowchart LR
    epoch["MLS epoch E"] -- "export_secret, label: ainari ipsec base v1, 32 bytes" --> base["base secret of epoch E<br/>persisted, as long as a pair uses E"]
    base -- "HKDF-expand, info: ainari ipsec sa v1:vni:src_vm@src_gw->dst_vm@dst_gw, 40 bytes" --> okm["40 bytes"]
    okm --> key["bytes 0..32: AES-256 key"]
    okm --> salt["bytes 32..36: GCM salt"]
    okm --> spi["bytes 36..40: SPI, highest bit set"]
```

## Removing a torii

A torii only leaves the group, when the last VM of the network is deleted from its host. As long
as another VM of the network runs there, only the connections of the deleted VM are removed. After
the last one, hanami removes the routes and revokes the grant. The torii itself unsubscribes as
well, as soon as its routes of the network are gone. Both lead to the same removal, which is only
queued once.

```mermaid
sequenceDiagram
    participant H as hanami
    participant I as izakaya
    participant TA as torii A (committer)
    participant TB as torii B (member)
    participant TC as torii C (leaving)

    H->>H: mark the address of the deleted VM as deleted,<br/>read the remaining addresses of the network
    alt another VM of the network runs on the host of C
        H->>TC: delete the routes to and from the deleted VM
        TC->>TC: remove the SAs, policies and rules of its connections
        Note over H,TC: C stays member, the group doesn't change
    else it was the last VM of the network on the host of C
        H->>TC: delete the routes of the network
        H->>H: sign revocation (vni, C, remove)
        H->>I: POST /mls_grant/internal
        I->>I: delete the grant of C, drop queued additions of C,<br/>queue operation remove(C)

        TC->>TC: agent: no route of the network left
        TC->>I: unsubscribe(vni)
        I->>I: remove(C) is queued already
        TC->>TC: delete the group and all keys of the network

        I->>TA: operation {removes: [C]}
        TA->>TA: stage one commit with all removals
        TA->>I: send commit to B
        TA->>TA: merge: epoch E+1, the pair with C ends,<br/>all other pairs get target E+1
        TA->>I: operation done (epoch E+1, members [A, B])
        TB->>TB: process the commit: epoch E+1, the same,<br/>warn: A removed C
        Note over TA,TB: key-change of the pair A-B to epoch E+1:<br/>afterwards, no key known to C is accepted anymore
    end
```

If the leaving torii is the committer, another living member becomes committer before the removal
is handed out. If it is the last member, the group is deleted together with its queue.

## Regular key-rotation

Groups, whose membership doesn't change, get new keys as well. This also keeps the sequence
numbers of the SAs far away from their end.

```mermaid
sequenceDiagram
    participant I as izakaya
    participant TA as torii A (committer)
    participant TB as torii B

    loop every second
        I->>I: tick: group with at least 2 members,<br/>last change older than key_rotation_interval (1 hour)?
    end
    I->>I: queue operation update, reset the rotation-time
    I->>TA: operation {update: true}, alone
    TA->>TA: self-update of its own leaf: epoch E+1 with a fresh secret
    TA->>I: send commit to B
    TA->>TA: merge: commit adds and removes nobody,<br/>all pairs get target E+1
    TA->>I: operation done (epoch E+1)
    TB->>TB: process the commit: all pairs get target E+1
    Note over TA,TB: key-change of every pair to epoch E+1
```

An update, which is queued next to a batch with a removal, is done with that batch, because the
removal moves all pairs as well.

## Changed routes

The keys only exist for the routes, which the torii has. A route, which is added, changed or
deleted, brings the keys of its network in line right away, without any message to the izakaya.

```mermaid
sequenceDiagram
    participant H as hanami
    participant T as torii
    participant K as kernel (xfrm)

    H->>T: add / update / delete route of vni 5
    T->>T: program the eBPF-datapath
    T->>T: connections = local VMs x encrypted remote VMs of vni 5
    T->>K: add missing incoming SAs of the incoming epochs<br/>of the pair with the gateway of the remote VM
    T->>K: rules "from remote VM to local VM lookup tenant-table"<br/>for every new connection
    T->>K: add missing outgoing SAs of the epoch of the pair, pin the policies to them
    T->>K: delete SAs, policies and rules of connections, which are gone
    Note over T,K: an encrypted route without keys, for example of a pair,<br/>which didn't switch yet, stays fail-closed: its block-policies<br/>drop the traffic instead of sending it unencrypted
```

### Path of an encrypted packet

The eBPF-datapath hands the packets of an encrypted route to the kernel, which encrypts, decrypts
and routes them. The routing rules make sure, that a packet only takes the table of its tenant:

```mermaid
sequenceDiagram
    participant VA as VM A
    participant TA as torii A (eBPF + kernel)
    participant TB as torii B (kernel + eBPF)
    participant VB as VM B

    VA->>TA: packet to VM B over its TAP
    TA->>TA: eBPF: route of VM B is encrypted, pass to the kernel
    TA->>TA: rule "from VM A iif TAP": tenant-table, next hop torii B<br/>(another source ends at the unreachable-rule)
    TA->>TB: xfrm out-policy: ESP with the SA of the epoch of the pair A-B
    TB->>TB: xfrm: decrypt with one of the incoming SAs,<br/>in/fwd-policies demand ESP for this address-pair
    TB->>TB: rule "from VM A to VM B": tenant-table, route over the TAP<br/>(never the default route of the underlay)
    TB->>VB: packet over the TAP of VM B
```

## Failures

### Restart of a torii

The MLS-state of a torii is persisted in its database, so a restart of the torii alone, which keeps
its database and its address, changes nothing for the group. A restart of the whole sakura-host,
which gives the torii a new address, is described in
[Restart of a sakura-host](#restart-of-a-sakura-host).

```mermaid
sequenceDiagram
    participant T as torii
    participant K as kernel (xfrm)
    participant I as izakaya

    T->>T: restart: restore routes, MLS-identity, groups, pairs<br/>and the queued Ready and Switched
    T->>K: install the SAs of all pairs again
    T->>I: subscribe(vni, has_group = true)
    I-->>T: member
    T->>I: send the queued Ready and Switched,<br/>which izakaya didn't accept yet
    Note over T: the pairs go on, where they stopped
```

### Torii lost its MLS-state

The MLS-identity of a torii is part of its MLS-state. A torii, which lost its database, while its
sakura-host keeps running, comes back with the same address, but with a new signature-key. hanami
pinned the old key, and the grant at izakaya still names it, so the committer finds no key-package
with the granted key and refuses the addition. The encrypted routes of the torii stay fail-closed,
until sakura registers its host again, for example with its next restart, which lets hanami pin
the new key.

```mermaid
sequenceDiagram
    participant T as torii (lost its state)
    participant I as izakaya
    participant C as committer

    T->>T: new MLS-identity: same address, new signature-key K2
    T->>I: upload key-packages with key K2
    T->>I: subscribe(vni, has_group = false)
    alt T was the only member
        I->>I: start the group from scratch
        I-->>T: create
    else other members exist
        I->>I: T is committer? Then another living member becomes committer
        I->>I: queue operation add(T) with its grant (key K1)
        I-->>T: joining
        I->>C: operation {adds: [T with grant with K1]}
        C->>I: claim a key-package of T with key K1
        I-->>C: 404, T has only key-packages with K2
        C->>I: operation done (refused: [T, no key-package])<br/>the other changes of the batch go on
        Note over T: stays out of the group, its encrypted routes drop the traffic,<br/>until sakura registers the host again and hanami pins K2
    end
```

### Restart of a sakura-host

In kubernetes, sakura and the torii in front of it keep their databases and the disks of the VMs
in a directory of the node (`sakura.host_data_path`, `<pod-name>/sakura` and `<pod-name>/torii`),
which outlives the pod. A sakura-host, whose pod is recreated, still loses:

- the cloud-hypervisor processes of its VMs, which end together with sakura.
- the address of the torii in front of it. The torii restores its TAP-devices, routes and
  packet-filters from its database, but its MLS-identity is its address, so it drops its old
  MLS-state and starts with a new identity and without groups. The torii of the other hosts and
  the torii at the edge still route the VMs of the host to its old address, and the groups still
  contain its old identity.

Sakura registers its host with every start again. hanami takes this as the sign, that the torii
may have a new address: it forgets the pinned identity and, in the background, pins the new one
and changes the routes of the torii at the edge and of the other hosts to the new address. Sakura
boots the VMs again, which were running. Every step only changes, what differs, so a host, which
kept its address, for example because only sakura restarted, is left as it is.

```mermaid
sequenceDiagram
    participant S as sakura (restarted)
    participant TN as torii of the host (new address)
    participant H as hanami
    participant E as torii (edge)
    participant TO as torii of another host
    participant I as izakaya

    TN->>TN: restore TAPs, routes and packet-filters from its database
    TN->>TN: MLS-identity 10.42.8.12 doesn't match the address 10.42.8.13:<br/>drop the MLS-state, new identity 10.42.8.13
    S->>H: POST /host/internal (registration-key)
    H->>H: forget the pinned identity of the host,<br/>remember the old one (10.42.8.12)
    H-->>S: 201
    H->>H: start the restore of the host in the background

    S->>S: remove the API-sockets without process
    S->>S: boot every VM again, which was running<br/>(cloud-hypervisor opens or creates its TAP)

    loop until the torii answers
        H->>TN: list routes
    end
    H->>TN: GET /network_interface/underlay/internal
    TN-->>H: new underlay-address of the host (10.42.8.13)
    loop every VM of the host
        H->>E: route to the VM over 10.42.8.13 (updated in place)
    end
    loop every encrypted network of the host
        H->>TN: GET /network_crypto/mls/identity/internal
        H->>H: pin the new identity 10.42.8.13 with its key
        H->>I: grant (vni, 10.42.8.13, key, add)
        H->>I: revocation (vni, 10.42.8.12, remove)
    end
    loop every VM of the same networks on other hosts
        H->>TN: route to the other VM over its host
        H->>TO: route back to the VM of the host over 10.42.8.13 (updated in place)
    end
    Note over H: a run with a failed step is repeated after 30 seconds,<br/>up to 5 times

    TN->>I: subscribe(vni, has_group = false)
    alt another member is alive
        I-->>TN: joining, the committer adds the new torii
    else no other member is alive
        I->>I: start the group from scratch
        I-->>TN: create
    end
    Note over TN,TO: key-change of the pairs with the new torii:<br/>the VMs reach each other encrypted again
```

Addresses without a VM, for example of a reservation, which failed halfway, are left out.

### Silent members

Every torii polls the izakaya every second. A member, which didn't for 120 seconds
(`member_timeout`), is gone, for example because its host came back in a new pod with a new
identity, and is removed from its groups. As long as it is member, only its own pairs wait for it
(see [Slow peer](#slow-peer)), and the others can't remove the keys, which it knows. A torii,
which was only cut off, isn't lost: it subscribes again with its grant and is added again.

If no other member of a group is alive anymore, nobody can add a torii, which subscribes. The
group is started from scratch with that torii then. A member, which only was cut off, joins the new
group again, which replaces its old one without losing a packet (see
[Restart of izakaya](#restart-of-izakaya)).

Both only happen, once the izakaya itself runs longer than the timeout, because before that, a
torii, which didn't contact it yet, can't be told apart from a gone one.

```mermaid
sequenceDiagram
    participant I as izakaya
    participant TA as torii A (committer)
    participant TB as torii B (gone)
    participant TC as torii C (new)

    loop tick every second
        I->>I: member without contact for 120 seconds?
    end
    I->>I: B is silent: queue operation remove(B)
    I->>TA: operation {removes: [B]}
    TA->>I: operation done (members [A])
    Note over TA: the pair with B ends with all its keys

    Note over TA: later A is gone as well
    TC->>I: subscribe(vni, has_group = false)
    I->>I: no other member alive for 45 seconds
    I->>I: start the group from scratch
    I-->>TC: create
```

### Dead committer

A torii counts as dead, when it didn't poll for 45 seconds. An operation, which is not finished
after 30 seconds, is handed out again as a whole batch, if necessary to another committer. After 3
attempts it is given up.

```mermaid
sequenceDiagram
    participant I as izakaya
    participant TA as torii A (committer)
    participant TB as torii B

    I->>TA: operation {adds: [C, D]}
    Note over TA: crashed, no poll anymore
    loop tick every second
        I->>I: batch in flight for more than 30 seconds?
    end
    I->>I: retries + 1 for all rows of the batch,<br/>A didn't poll for 45 seconds: B becomes committer
    I->>TB: operation {adds: [C, D]}
    TB->>TB: verify the grants, add C and D with one commit
    TB->>I: operation done
```

### Slow peer

A torii, which is down, slow or refused a commit, only holds back its own pairs. The other pairs
move on, and izakaya hands out the next change right after the last one. Nobody loses a key: a
pair, whose peer doesn't answer, keeps sending with its old epoch, which the peer can still
decrypt, and keeps the incoming keys of the old epoch, until the peer switched.

```mermaid
sequenceDiagram
    participant TA as torii A
    participant TB as torii B (slow)
    participant TC as torii C

    Note over TA,TC: commit into epoch E moves all pairs,<br/>Ready and Switched are delivered by izakaya
    TA->>TC: Ready(E)
    TC->>TA: Ready(E)
    Note over TB: down, or refused the commit of epoch E
    TA->>TC: Switched(E)
    TC->>TA: Switched(E)
    Note over TA,TC: pair A-C arrived at epoch E
    Note over TA,TB: pair A-B keeps waiting for the Ready of B:<br/>A keeps sending with epoch E-1, which B can decrypt
```

A torii, which stays silent, is removed after `member_timeout`, which ends its pairs (see
[Silent members](#silent-members)).

### Restart of izakaya

All state of izakaya is in its database, so a restart of one instance changes nothing.

If the database is lost, the grants, the key-packages, the queues and the groups are lost with it.
Everything comes back on its own:

- hanami brings the grants back with its next refresh.
- The torii upload new key-packages, as soon as they see, that izakaya has less than 5 of them.
- The torii renew their subscriptions every 30 seconds. The first one gets `create` and starts a
  new group, the others join it.

The new group is the next generation. Its pairs move to it like to any new epoch: the torii keep
sending with the keys of the old group, until the peer of the pair is ready with the new one, and
keep the incoming keys of the old group, until the peer switched. So the replacement of the group
doesn't lose a packet either. A pair, whose peer doesn't join the new group within 120 seconds,
ends with all its keys, and its connections fall back to fail-closed.

```mermaid
sequenceDiagram
    participant H as hanami
    participant TA as torii A
    participant I as izakaya (empty database)
    participant TB as torii B

    Note over TA,TB: old group with epoch 37: SAs of epoch 37 in use

    TA->>I: subscribe(vni, has_group = true)
    I-->>TA: 403, no grant
    H->>I: refresh of the grants for A and B
    TA->>I: key-packages, as less than 5 are left
    TB->>I: key-packages, as less than 5 are left

    TA->>I: subscribe(vni, has_group = true)
    I-->>TA: create
    TA->>TA: new group, generation 2, epoch 0:<br/>B is no member yet, the pair keeps<br/>the keys of the old group (epoch 37)

    TB->>I: subscribe(vni, has_group = true)
    I-->>TB: joining
    I->>TA: operation {adds: [B with grant]}
    TA->>I: welcome to B
    TA->>TA: epoch 1: pair A-B gets target (2, 1),<br/>keeps sending with (1, 37)
    TA->>I: operation done (epoch 1)
    TB->>TB: welcome: new group, generation 2:<br/>pair B-A gets target (2, 1), keeps sending with (1, 37)

    Note over TA,TB: key-change of the pair to (2, 1)
    TA->>TB: Ready(2, 1)
    TB->>TA: Ready(2, 1)
    Note over TA,TB: both send with the new group, both still accept the old one
    TA->>TB: Switched(2, 1)
    TB->>TA: Switched(2, 1)
    Note over TA,TB: the keys of the old group are removed, once no packet uses them
```
