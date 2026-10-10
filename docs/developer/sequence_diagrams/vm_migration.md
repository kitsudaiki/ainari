# Cold migration of a virtual machine

An admin can move a virtual machine to another sakura-host. The migration is a cold migration: the
virtual machine is shut down on its current host, its disk is transferred directly to the new host
and it is booted there again, if it was running before. It keeps its UUID, its internal address,
its MAC-address, its TAP-device, its proxy-port and its packet-filters.

hanami orchestrates the migration in a background-job. The sakura-hosts only provide the single
steps, which run as tasks of the virtual machine, so they are serialized with all other tasks of
it, like a start or a snapshot.

## Components

| Component | Role |
|---|---|
| **hanami** | Accepts the request of the admin, allocates the resources on the new host and runs the background-job, which calls all other components. It moves the network of the virtual machine between the gateways and rolls the migration back, if a step fails. |
| **miko** | Renews the token of the admin during the migration, because the transfer of a large disk can take longer than the lifetime of a token. |
| **sakura** (source) | *Prepares* the virtual machine: shuts it down and freezes it in the state `MIGRATING`. Serves its description and its files to the target and removes its copy at the end. |
| **sakura** (target) | *Imports* the virtual machine: creates it with the same identity, pulls its seed-image and its root-disk from the source and boots it. |
| **torii** (hosts) | Get the TAP-device, the routes and the packet-filters of the virtual machine on the new host and forget them on the old host. |
| **torii** (edge) | Routes the address of the virtual machine to the new host and points the proxy-port of the virtual machine to the new host. |
| **izakaya** | Gets the MLS-grant for the torii of the new host, if the network is encrypted, and its revocation for the old host (see [Key-exchange of the network-encryption](network_crypto_key_exchange.md)). |

```mermaid
flowchart LR
    admin["admin"]
    hanami["hanami<br/>background-job"]
    miko["miko<br/>token renewal"]
    source["sakura (source)<br/>prepare, serve files, remove"]
    target["sakura (target)<br/>import, boot"]
    toriiS["torii (source)"]
    toriiT["torii (target)"]
    edge["torii (edge)<br/>routes, proxy"]

    admin -- "POST .../migrate/admin" --> hanami
    hanami <--> miko
    hanami -- "tasks" --> source
    hanami -- "tasks" --> target
    target -- "pulls description and files<br/>(internal api, zstd-stream)" --> source
    hanami -- "TAP, routes, filters" --> toriiT
    hanami -- "forget the VM" --> toriiS
    hanami -- "route, proxy" --> edge
```

## States of the virtual machine

The virtual machine exists on both hosts during the migration. The state `MIGRATING` blocks every
other operation on it: sakura rejects start, stop, reboot and the snapshots, hanami rejects the
deletion and changes of the packet-filters.

```mermaid
stateDiagram-v2
    state "source host" as source {
        [*] --> RUNNING_S
        [*] --> STOPPED_S
        RUNNING_S --> MIGRATING_S: prepare (graceful shutdown)
        STOPPED_S --> MIGRATING_S: prepare
        MIGRATING_S --> removed: remove (after success)
        MIGRATING_S --> STOPPED_S: cancel (after failure)
        STOPPED_S --> RUNNING_S: cancel with boot
        RUNNING_S: RUNNING
        STOPPED_S: STOPPED
        MIGRATING_S: MIGRATING
        removed: entry removed completely
    }
    state "target host" as target {
        [*] --> MIGRATING_T: import accepted
        MIGRATING_T --> RUNNING_T: files received, booted
        MIGRATING_T --> STOPPED_T: files received, not booted
        MIGRATING_T --> ERROR_T: import failed
        ERROR_T --> removed_T: remove (rollback)
        MIGRATING_T: MIGRATING
        RUNNING_T: RUNNING
        STOPPED_T: STOPPED
        ERROR_T: ERROR
        removed_T: entry removed completely
    }
```

A removed virtual machine leaves no deleted entry behind, unlike a normal deletion. The deleted
entries are reported to hanami, when the host registers itself again, which would delete the
virtual machine, which still exists on the other host. It also allows to migrate the virtual
machine back to a host, which ran it before.

## Start of the migration

The request is checked completely and the resources are allocated on the new host, before it is
accepted. Everything afterwards runs in the background.

```mermaid
sequenceDiagram
    actor A as admin
    participant H as hanami
    participant DB as hanami database
    participant M as miko

    A->>H: POST /virtual_machine/{uuid}/migrate/admin<br/>{target_host_uuid}
    H->>H: only admins (401)
    H->>DB: VM, its current host and the target host
    H->>H: target is not the current host (400),<br/>both hosts exist (404)
    H->>DB: address of the VM, has a host-address (409)
    H->>H: mark the VM as migrated (409, if it is already)
    H->>M: PUT /v1alpha/token
    M-->>H: new token with its lifetime
    H->>DB: allocate the resources of the VM on the target host,<br/>same isolation as the current host (409)
    H-->>A: 202 Accepted {virtual_machine_uuid, source_host_uuid, target_host_uuid}
    H->>H: start the background-job
```

### Renewal of the token

The sakura-hosts accept their internal endpoints only with a valid token, so the job holds a
session, which renews the token at miko, as soon as half of its lifetime is over. Every call of the
job takes its token from this session.

```mermaid
sequenceDiagram
    participant J as migration-job
    participant S as session
    participant M as miko

    J->>S: context()
    alt half of the lifetime of the token is over
        S->>M: PUT /v1alpha/token (current token)
        M-->>S: new token, lifetime
    end
    S-->>J: context with a valid token
```

## Step 1: prepare the virtual machine on the source host

The steps on the sakura-hosts are tasks. hanami follows every task by polling it every 2 seconds,
until it is finished or failed.

```mermaid
sequenceDiagram
    participant H as hanami
    participant S as sakura (source)
    participant CH as cloud-hypervisor (source)

    H->>S: GET /virtual_machine/{uuid}/internal
    S-->>H: vm_state
    H->>H: boot = (vm_state == RUNNING)
    H->>S: POST /virtual_machine/{uuid}/migration/prepare/internal
    S-->>H: task
    Note over S: task in the queue of the VM
    S->>S: only a RUNNING or STOPPED VM can be prepared
    opt VM is running
        S->>CH: power-button
        S->>S: wait up to 60s for the shutdown of the guest
        opt guest didn't shut down
            S->>CH: hard shutdown
        end
    end
    opt cloud-hypervisor process still runs
        S->>CH: shutdown of the process<br/>(it holds a lock on the root-disk)
    end
    S->>S: state MIGRATING
    loop every 2s
        H->>S: GET /task/{uuid}
    end
    S-->>H: Finished
```

## Step 2: move the network to the target host

The VM is shut down now, so its network can be moved, before it is started on the target host.
The packet-filters are applied on the new torii, before the VM can send anything, so it is never
reachable without them. Every call only changes, what differs, so the steps can be repeated.

```mermaid
sequenceDiagram
    participant H as hanami
    participant DB as hanami database
    participant TT as torii (target)
    participant I as izakaya
    participant E as torii (edge)
    participant TO as torii (other hosts)

    Note over H,TT: attach the VM to the target host
    H->>TT: register TAP (name, tenant, MAC, IP of the VM)
    H->>TT: route onto the TAP<br/>(replaces a route towards the old host)
    H->>DB: packet-filters of the VM
    loop ingress and egress
        H->>TT: clear the filter
        H->>TT: add the stored ip-ranges and ports
    end
    opt network is encrypted and the target is not the edge
        H->>I: MLS-grant for the torii of the target host
    end
    loop every other VM of the network on another host
        H->>TT: route towards the VM on its host
    end

    Note over H,TO: point all other gateways to the target host
    opt target is not the edge
        H->>E: route of the VM towards the target host
    end
    loop every other host with a VM of the network (also the source)
        H->>TO: route of the VM towards the target host
    end
```

## Step 3: import the virtual machine on the target host

The target host pulls the files directly from the source host over the internal api of the source
host. The files are sent as a single zstd-frame each, which contains the size and a checksum of the
content. The root-disk is a sparse file, whose unused parts are zeros, which compress to almost
nothing, so the source host needs no temporary copy of it. The target host skips the blocks of
zeros, so the root-disk stays sparse.

```mermaid
sequenceDiagram
    participant H as hanami
    participant T as sakura (target)
    participant S as sakura (source)
    participant CH as cloud-hypervisor (target)

    H->>T: POST /virtual_machine/migration/internal<br/>{uuid, source_address, boot}
    T->>S: GET /virtual_machine/{uuid}/migration/internal
    S-->>T: description (name, size, image, public-key,<br/>network, IP, TAP, MAC, owner, project)
    T->>T: create the VM with the same identity,<br/>state MIGRATING
    T-->>H: task
    Note over T: task in the queue of the VM
    loop seed, then root_disk
        T->>S: GET /virtual_machine/{uuid}/migration/file/{file}/internal
        S->>S: VM is MIGRATING, no cloud-hypervisor process
        S-->>T: X-Migration-File-Size + zstd-stream
        T->>T: decompress into {file}.part,<br/>skip blocks of zeros
        T->>T: check frame-end, checksum and size
        T->>T: rename {file}.part to {file}
    end
    T->>T: store the paths of the files
    alt boot
        T->>CH: start process, create and boot the VM
        T->>T: state RUNNING
    else
        T->>T: state STOPPED
    end
    loop every 2s
        H->>T: GET /task/{uuid}
    end
    T-->>H: Finished
```

## Step 4 and 5: take over and clean up the source host

The VM runs on the target host now. A failure from here on doesn't roll the migration back, but
is logged for a manual cleanup.

```mermaid
sequenceDiagram
    participant H as hanami
    participant DB as hanami database
    participant E as torii (edge)
    participant TS as torii (source)
    participant I as izakaya
    participant S as sakura (source)

    Note over H,E: take over
    H->>DB: host of the VM = target host
    H->>DB: host of the address of the VM = target host
    H->>E: PUT /proxy/{uuid}/internal (external address of the target host)
    E->>E: restart the proxy on the same port

    Note over H,S: clean up the source host
    alt source is the edge or runs another VM of the network
        H->>TS: clear the packet-filters of the VM<br/>(its route leads to the target host now)
    else
        H->>TS: delete the routes towards the VM<br/>(drops its packet-filters)
        H->>TS: delete the routes towards the other VMs of the network
        opt network is encrypted
            H->>I: revoke the MLS-grant of the source host
        end
    end
    H->>S: DELETE /virtual_machine/{uuid}/migration/internal
    S-->>H: task
    S->>S: only a MIGRATING or ERROR VM is removed
    S->>S: remove all files of the VM,<br/>remove its entry completely
    loop every 2s
        H->>S: GET /task/{uuid}
    end
    S-->>H: Finished
    H->>DB: release the resources of the VM on the source host
    H->>H: unmark the VM
```

The TAP-device stays on the source host, like with the deletion of a virtual machine, because
torii has no endpoint to remove it.

## Rollback

A failure before the take-over moves the VM back to the source host. Every step is tried, also if
another one failed, and only undoes, what was already done. The only exception is the removal from
the target host: the removal runs after the import in the queue of the VM, so it also covers an
import, whose end was not seen, and refuses to remove a VM, which was imported successfully.
Without its confirmation, the rollback stops, because the VM could otherwise run on both hosts with
the same address.

```mermaid
sequenceDiagram
    participant H as hanami
    participant DB as hanami database
    participant T as sakura (target)
    participant N as torii (all)
    participant S as sakura (source)

    opt import was requested
        H->>T: DELETE /virtual_machine/{uuid}/migration/internal
        alt removed, or the VM was never created there
            T-->>H: Finished / 404
        else removal failed or the VM runs there
            T-->>H: Error
            H->>H: stop the rollback, the VM stays frozen on the source,<br/>resources stay allocated on both hosts,<br/>log for a manual cleanup
        end
    end
    opt network was moved
        H->>N: attach the VM to the source host again
        H->>N: point all gateways to the source host
        H->>N: release the target host
    end
    opt prepare was requested
        H->>S: POST /virtual_machine/{uuid}/migration/cancel/internal {boot}
        S->>S: MIGRATING -> STOPPED<br/>(nothing, if the VM was not prepared)
        opt boot
            S->>S: start the VM
        end
    end
    H->>DB: release the resources of the VM on the target host
    H->>H: unmark the VM
```

## Limits

- The job lives only in the memory of hanami. If hanami restarts during a migration, the VM stays
  frozen in the state `MIGRATING` on the source host and has to be cleaned up by hand.
- The task-queues of sakura only live in its memory. At its start, sakura marks all tasks as failed,
  which didn't end before its restart. So a step of a migration, which was interrupted by a restart
  of its host, fails and the migration is rolled back.
- The files are transferred over the internal api of the sakura-hosts. They are only encrypted, if
  the address of the source host, which it registered at hanami, uses `https`.
