# Plan: Ceph-backend for onsen

Second iteration of the storage-backends of onsen. It builds on the first iteration
([onsen_shared_volume_plan.md](onsen_shared_volume_plan.md)), which adds the backend-traits
`FileBackend` and `VolumeBackend`, the local backend with NFS, volumes as resource in ryokan, the
attachment with LUKS in sakura and the live migration. This plan only adds Ceph as second backend
for production-setups, without changes of the REST-APIs and of the volume-flows.

## 1. Prerequisites

- The first iteration is complete up to V7 (live migration with the local backend).
- `VolumeConnection` is a tagged enum and sakura has the trait `VolumeConnector`, so this
  iteration only adds variants and implementations.
- `hosts.backend_type` and `volumes.backend_type` exist in ryokan.

## 2. Decisions

| Topic               | Decision                                                                                                                                                                                                                                              |
| ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Volumes             | **RBD-images** in a dedicated pool. RBD is the standard for live migration of VMs and gives real block-devices on the sakura-hosts (no loop-device and no file-system in between).                                                                    |
| Files               | Images, snapshots and backups are stored in a **CephFS**, which is mounted on the onsen-host. So `FileBackend` stays a POSIX-directory and `core/file_operations.rs` is reused unchanged. RADOS-objects or the RGW are not used.                       |
| Access to the cluster | Phase 1 uses the `rbd`-CLI, like sakura uses `qemu-img` and `cloud-localds`, because the rust-bindings of librados/librbd are not maintained well. A switch to librbd is possible later without changes of the API.                                |
| Encryption          | Unchanged: LUKS2 on the sakura-host on top of the mapped RBD-device. The built-in RBD-encryption of librbd is **not** used, because it only works with librbd/qemu and not with krbd, which cloud-hypervisor needs for a block-device.                     |
| Cephx-users         | Onsen has its own user with rights for the volume-pool and the CephFS. The sakura-hosts share one user, which can only read and write the volume-pool (`profile rbd pool=ainari-volumes`), and can't create or delete images. Only the name of the user is sent to sakura, the key is deployed on the hosts. |
| Multiple onsens     | Multiple onsen-instances can use the same cluster and pool. Ryokan still stores the onsen-host of every volume (`host_uuid`), the onsen-host only owns the management of the volume, not the data.                                                        |
| Fencing             | When a sakura-host is regarded as dead, its client is blocklisted in Ceph (`ceph osd blocklist add`), before its volumes are force-detached. So the dead host can't write into a volume anymore, which is attached by another host.                    |

## 3. Changes in onsen

### 3.1 Config

```toml
[storage]
backend = "ceph"

[storage.ceph]
# mount-point of the CephFS for images, snapshots and backups
file_location = "/mnt/cephfs/ainari"
conf_path = "/etc/ceph/ceph.conf"
client_name = "client.ainari-onsen"
keyring_path = "/etc/ceph/ceph.client.ainari-onsen.keyring"
volume_pool = "ainari-volumes"
# cephx-user, which the sakura-hosts use; only its name is sent, the key is deployed on the hosts
sakura_client_name = "client.ainari-sakura"
```

- Validation at startup: an unreachable cluster, a missing pool or a `file_location`, which is not
  a mounted CephFS, stops onsen with a clear error.
- New example-config `example_configs/ainari/onsen_ceph.toml` and its config-docs.

### 3.2 Backend

New file `src/binaries/onsen/src/backend/ceph.rs`:

- `FileBackend`: the existing file-operations on `file_location`.
- `VolumeBackend`:
  - `create` → `rbd create --size <MiB> <pool>/<uuid>`
  - `delete` → `rbd rm`, a not existing image is no error
  - `resize` → `rbd resize` (only grow)
  - `info` → `rbd info --format json` / `rbd du`
  - `connection` → `VolumeConnection::Rbd { monitors, pool, image, client_name }`
- `GET /v1alpha/backend/internal` reports `CEPH` and the free capacity of the pool
  (`ceph df --format json`).

### 3.3 Shared structs

`VolumeConnection` in `ainari_api_structs::storage_structs` gets the variant:

```rust
Rbd { monitors: Vec<String>, pool: String, image: String, client_name: String },
```

## 4. Changes in sakura

- New connector `core/volume/rbd.rs` for `VolumeConnector`:
  - `open` → `rbd map --id <client_name> -m <monitors> <pool>/<image>`, returns `/dev/rbdX`;
    an already mapped image is reused (`rbd showmapped --format json`).
  - `close` → `rbd unmap`.
- The LUKS-part, the device-name `/dev/mapper/ainari-vol-<uuid>` and the live migration stay
  unchanged, because they only see the block-device of the connector.
- Config `sakura.toml`, section `[volumes]`: `ceph_conf_path` and `ceph_keyring_path`.
- The `rbd`-kernel-module must be loaded on the sakura-hosts (documented in the installation).

## 5. Changes in ryokan and hanami

- Ryokan: no changes of the API. The host-selection for volumes takes the free capacity of
  `GET /backend/internal` into account, because a Ceph-pool is shared and can be full.
- Fencing: new admin-endpoint in hanami
  `POST /v1alpha/host/{host_uuid}/fence/admin`, which marks the sakura-host as dead, asks
  onsen to blocklist its client (`POST /v1alpha/backend/fence/internal` with the address of the
  host) and afterwards force-detaches its volumes in ryokan. Without Ceph the endpoint only
  force-detaches and warns, that the host must be off.

## 6. Deployment and other parts of the repo

- `dockerfiles/{debian_based,nix_based}`: `ceph-common` for onsen and sakura.
- `deploy/operator`: `onsen.backend = ceph` with a reference to a Rook-cluster, the pool and the
  secrets of the cephx-users. Zarf: optional Rook-package.
- Local testing: a single-node Ceph (`cephadm bootstrap --single-host-defaults` or `microceph`) in
  the vagrant-setup, documented in `docs/developer/local_testing/vagrant_setup.md`. Docker-compose
  stays with the local backend.
- Docs: installation of Ceph and the cephx-users in `docs/deployer/installation/`, config-docs.

## 7. Phases

| Phase | Content                                                                         |
| ----- | ------------------------------------------------------------------------------- |
| C1    | onsen ceph-backend (files on CephFS, volumes as RBD), config, example-config    |
| C2    | sakura RBD-connector, vagrant-setup with single-node Ceph                       |
| C3    | capacity-based host-selection in ryokan, fencing                                |
| C4    | operator/Rook, zarf, docs                                                       |

## 8. Tests

- Unit-tests onsen: config-parsing and validation, building of the `rbd`-commands, parsing of
  `rbd info` / `ceph df`.
- Unit-tests sakura: building of the `rbd map`-command, parsing of `rbd showmapped`.
- E2E in the vagrant-setup with Ceph: the same E2E-tests as in the first iteration (create VM
  with new/existing volume, delete VM, double attach, live migration, restart of a host), and:
  1. The raw RBD-image (`rbd export`) contains only cipher-text.
  2. Fencing: a blocklisted sakura-host can't write into its volume anymore, the volume can be
     attached by another host.
  3. A full pool rejects the creation of a volume with a clear error.

## 9. Open points

- **RBD-snapshots** for snapshots and isekai-backups instead of copying the whole disk with
  `qemu-img`.
- **librbd** instead of the CLI.
- **Longhorn** as third backend (iSCSI-frontend): `VolumeConnection` gets another variant, sakura
  an iSCSI-connector.
