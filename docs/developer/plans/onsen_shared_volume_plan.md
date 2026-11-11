# Plan: Shared volumes in onsen and live migration in sakura

At the moment onsen only stores files (images, snapshots, backups) in a local directory
(`storage.location`), and the root-disk of a virtual_machine is always a raw file in
`local_vm_storage_path` of its sakura-host (`boot_disk`). Because of this a virtual_machine can
only be migrated cold (hanami `core/migration/job.rs`, sakura `migration/transfer.rs`), where the
whole disk is copied from host to host.

This plan adds **volumes** as a new resource: block-storage, which is stored by an onsen in a
storage-backend, which is always LUKS-encrypted and which can be attached by every sakura-host
over the network. A virtual_machine, whose root-disk is a volume, can be migrated live with the
migration-functions of cloud-hypervisor, because its disk doesn't have to be copied anymore.

This is the **first iteration**: it only contains the backend-abstraction and the local backend,
whose volumes are files on the local disk of the onsen-host, which the sakura-hosts mount over NFS.
This is primarily for testing and has no redundancy, which is accepted for now. The Ceph-backend
for production is the second iteration and is planned in
[onsen_ceph_backend_plan.md](onsen_ceph_backend_plan.md).

## 1. Requirements

| #   | Requirement                                                                                                       |
| --- | ----------------------------------------------------------------------------------------------------------------- |
| R1  | The local disk-backend of onsen stays usable for testing. The backend is selected by the config, multiple backends must be possible in the future (ceph, longhorn, …). For now: local + one other. This iteration: local, the other one (Ceph) is the second iteration. |
| R2  | Onsen stores shared volumes, which are reachable over the network (NFS or similar).                                |
| R3  | Every sakura-host can attach a volume as disk of a VM, so live migration is possible.                              |
| R4  | Volumes are always LUKS-encrypted in the storage. Only the sakura-host, which attached the volume, sees the plain data. |
| R5  | Ryokan knows, which volume is stored in the backend of which onsen.                                               |
| R6  | Volumes are a new resource type beside images and backups, with their own table in ryokan.                        |
| R7  | On VM-creation the user selects a local disk (like today) or a new volume with a custom size.                     |
| R8  | Deleting a VM with a volume detaches the volume, the volume stays.                                                |
| R9  | On VM-creation an already existing volume can be selected.                                                        |

## 2. Decisions

| Topic                     | Decision                                                                                                                                                                                                                                                                                                                                                                                         |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Backend-abstraction       | Onsen gets two traits: `FileBackend` (the existing file-API for images, snapshots, backups) and `VolumeBackend` (create, delete, resize, info, connection-info of a volume). The config selects exactly one backend per onsen-instance with `[storage] backend = "local"`. A new backend only implements the traits, adds a config-section and a variant of `VolumeConnection`, the REST-API of onsen stays the same. The traits must not contain anything NFS-specific, so the Ceph-backend of the second iteration fits in without changes.        |
| Volumes of the local backend | Sparse files `<storage.location>/volumes/<backend_id>.img` on the local disk of the onsen-host, which are exported read-write over **NFS** by the onsen-host and mounted by the sakura-hosts. The export itself (`/etc/exports`) is configured by the deployer, not by onsen, so onsen doesn't need root-permissions. Primarily for testing: the onsen-host is a single point of failure and the performance is limited, which is accepted for this iteration.                                                                                                       |
| One onsen, one backend    | A ryokan-host-entry gets the backend-type and the capability `supports_volumes`, which the onsen sends on registration. Volumes are only placed on hosts with this capability, which are no backup-targets.                                                                                                                                                                                       |
| Encryption                | LUKS2 on the sakura-host, never in onsen. Ryokan generates a secret per volume in omamori (like for images). Sakura gets it with the token of the user at the attachment, gives it to `cryptsetup` over stdin (`--key-file=-`), never writes it to a disk and never sends it to onsen. Onsen and the storage only see the cipher-text.                                                                 |
| LUKS-format               | The volume is formatted by the **first sakura-host**, which attaches it, not by onsen, because onsen must never know the key. Ryokan tracks `is_initialized`; sakura only formats, when ryokan says `false` **and** `cryptsetup isLuks` fails, so an existing volume is never formatted again by mistake.                                                                                             |
| PBKDF                     | The passphrase is a random 256-bit key, so `--pbkdf pbkdf2 --pbkdf-force-iterations 1000` is enough and avoids the ~1 GiB memory and the seconds of argon2id on every attachment (important during a live migration).                                                                                                                                                                              |
| Device-name               | The plain device is always `/dev/mapper/ainari-vol-<backend_id>` on every host (by the `backend_id`, so the old object and the copy of a restore never collide), so the config of cloud-hypervisor, which is transferred on live migration, is valid on the target without changes.                                                                                                                                                                                              |
| Use of a volume           | A volume is the **root-disk** of a VM in this plan. `disk_type` of a VM is `LOCAL` (like today) or `VOLUME`. Additional data-volumes are an open point (section 11).                                                                                                                                                                                                                                  |
| Size of a local disk      | The size of a local root-disk is part of the **vm-type** in hanami (`disk_size`, like cores and memory), not of the create-request anymore. Hanami sends it with the reservation to the sakura-host.                                                                                                                                                                                       |
| Disk-space and scheduling | Local disk-space of a sakura-host is only reserved for a **local root-disk**, never for a volume. The reservation in hanami gets only the flag `disk_type` (`local` / `volume`), no size and no volume. For `local` the scheduler checks and allocates cores, memory and the `disk_size` of the vm-type on the sakura-host (in one transaction, like today). For `volume` it **only checks and allocates cores and memory**, so a host with a full disk can still run volume-VMs. |
| Flow of the VM-creation   | Hanami never gets a disk-size or a volume. For a VM with a volume, the client (SDK, CLI, dashboard) **first creates the volume** with its custom size in ryokan (or selects an existing one), then reserves the VM in hanami with `disk_type = volume` and gives the UUID of the volume to the create-call of the sakura-host. Sakura decides by `is_initialized` of the volume, if the image is written into it (new volume) or if it boots from it (existing volume). |
| Exclusive attachment      | A volume is attached to at most one VM. The status-change `AVAILABLE` → `ATTACHING` runs in one transaction in ryokan, so two VMs can never write into the same volume. The only exception is the short overlap during a live migration (section 6), which ryokan tracks with `migration_host_uuid`.                                                                                                   |
| Owner                     | A volume belongs to a project (`project_id`, `owner_id` like images). It survives the deletion of its VM and is deleted explicitly by the user. A volume, which is attached, can't be deleted (`409`).                                                                                                                                                                                              |
| Live migration            | Only VMs with `disk_type = VOLUME` are migrated live, VMs with a local disk keep the existing cold migration. Hanami decides it per VM by its `disk_type`. The memory-stream of cloud-hypervisor contains plain data of the guest, so it is never sent unencrypted over the network (section 6).                                                                                                                         |

## 3. Data-model

### 3.1 ryokan – table `volumes`

Diesel-migration for mysql and sqlite, documented in `docs/deployer/database/ryokan_db.md`.

| Column                | Type                     | Description                                                                                             |
| --------------------- | ------------------------ | ------------------------------------------------------------------------------------------------------- |
| `uuid`                | `VARCHAR(40)` PK         | UUID of the volume, stays the same for the whole life of the volume, also over a restore                  |
| `backend_id`          | `VARCHAR(40) NOT NULL`   | name of the volume in the backend of onsen (a random UUID), which changes with a restore (section 6.4)    |
| `name`                | `VARCHAR(256)`           | name given by the user                                                                                   |
| `host_uuid`           | `VARCHAR(40) NOT NULL`   | onsen-host, in whose backend the volume is stored (R5)                                                   |
| `backend_type`        | `VARCHAR(16) NOT NULL`   | `LOCAL` (later also `CEPH`), copy of the host-value at creation, so a volume can be checked without the host-entry   |
| `size`                | `BIGINT NOT NULL`        | size in GiB (the LUKS-header is subtracted from the usable size)                                         |
| `secret_uuid`         | `VARCHAR(40) NOT NULL`   | secret in omamori, which is the LUKS-passphrase                                                          |
| `is_initialized`      | `BOOLEAN NOT NULL`       | LUKS-header was written by a sakura-host                                                                  |
| `is_snapshot`         | `BOOLEAN NOT NULL DEFAULT FALSE` | volume is a snapshot of the root-disk of a volume-VM, like `is_snapshot` of `images` (section 6.4) |
| `volume_status`       | `VARCHAR(16) NOT NULL`   | `CREATING`, `AVAILABLE`, `ATTACHING`, `IN_USE`, `DETACHING`, `RESIZING`, `RESTORING`, `DELETING`, `ERROR`            |
| `attached_vm_uuid`    | `VARCHAR(40)` NULL       | VM, which uses the volume                                                                                |
| `attached_host_uuid`  | `VARCHAR(40)` NULL       | sakura-host, on which the volume is open                                                                 |
| `migration_host_uuid` | `VARCHAR(40)` NULL       | target sakura-host during a live migration, otherwise `NULL`                                             |
| `restore_host_uuid`, `restore_backend_id`, `restore_secret_uuid` | NULL | location and secret of the copy of a snapshot during a restore, otherwise `NULL` (section 6.4) |
| `restore_snapshot_uuid` | `VARCHAR(40)` NULL     | snapshot-volume, which is restored right now                                                              |
| `owner_id`, `project_id`, `created_*`, `updated_*` |  | like `images`                                                                                    |

Table `hosts`: new columns `backend_type VARCHAR(16) NOT NULL DEFAULT 'LOCAL'` and
`supports_volumes BOOLEAN NOT NULL DEFAULT FALSE`, which are updated on every registration of the
onsen.

### 3.2 sakura – table `virtual_machines`

- `disk_type VARCHAR(16) NOT NULL DEFAULT 'LOCAL'` (`LOCAL` / `VOLUME`), set by the reservation
  of hanami
- `volume_uuid VARCHAR(40) NULL`, set by the create-call
- `disk_size` stays: for `LOCAL` it is the value of the vm-type, which hanami sends on reservation,
  for `VOLUME` it is `0` (the size belongs to the volume in ryokan), so it always matches the
  disk-space, which hanami allocated on the host.
- `image_uuid` becomes optional for a VM, which boots from an existing volume (R9). The column is
  already nullable, only `VirtualMachineEntry` and the responses have to be changed.
- `root_disk_path` is `/dev/mapper/ainari-vol-<backend_id>` for volumes.

### 3.3 hanami – tables `vm_types` and meta-virtual-machines

- `vm_types`: new column `disk_size BIGINT NOT NULL DEFAULT 5` (GiB, same meaning as the
  `disk_size` of the create-request today, which is the default of sakura as well). Existing
  vm-types get the default.
- Meta-virtual-machines: `disk_type` and the allocated `disk_size` (`0` for `VOLUME`), so the
  migration-job knows, if the VM is migrated live, and the delete-watcher and the migration release
  exactly the allocated disk-space. The volume itself is not known by hanami.
- `select_host`: for `LOCAL` the host needs free cores, memory and the `disk_size` of the vm-type;
  for `VOLUME` the disk-space is neither checked nor allocated, only cores and memory.

### 3.4 miko – quota

New fields `max_volume` (number) and `max_volume_size` (sum in GiB) in the quota of a project,
checked by ryokan on creation and resize of a volume.

## 4. Changes in onsen

### 4.1 Config

```toml
[storage]
backend = "local"      # only "local" in this iteration

[storage.local]
location = "/var/lib/ainari/onsen"
# address and export-path of the NFS-export of "<location>/volumes", which the sakura-hosts mount;
# without these values the onsen doesn't support volumes
nfs_server = "10.0.0.5"
nfs_export = "/var/lib/ainari/onsen/volumes"
```

- The old `storage.location` stays readable for one release (mapped to `local`), so existing
  configs don't break. `example_configs/ainari/onsen.toml` and the config-docs are updated.
- `backend` is an enum in the config, so an unknown value already fails at parsing. A missing
  section of the selected backend stops onsen at startup with a clear error.

### 4.2 Structure

```text
src/binaries/onsen/src/
├── core/backend/
│   ├── mod.rs           # traits FileBackend + VolumeBackend, enum Backend from the config
│   └── local.rs         # files: the former core/file_operations.rs; volumes: sparse files + NFS-info
└── api/http_endpoints/volume/…
```

`VolumeBackend` (native `async fn` in the trait, dispatched by the enum `Backend`, so no
`async_trait` is needed; all sizes in bytes, a multiple of 512):

```rust
pub trait VolumeBackend {
    fn supports_volumes(&self) -> bool;
    async fn create_volume(&self, id: &Uuid, size: u64) -> Result<VolumeInfoResp, AinariError>; // repeatable
    async fn delete_volume(&self, id: &Uuid) -> Result<(), AinariError>;   // idempotent
    async fn resize_volume(&self, id: &Uuid, size: u64) -> Result<VolumeInfoResp, AinariError>; // only grow
    async fn volume_info(&self, id: &Uuid) -> Result<VolumeInfoResp, AinariError>; // incl. copy-state
    /// starts the copy of a whole volume within the backend in the background
    async fn copy_volume(&self, source_id: &Uuid, target_id: &Uuid) -> Result<(), AinariError>;
    async fn volume_connection(&self, id: &Uuid) -> Result<VolumeConnection, AinariError>;
}
```

A backend without support of volumes (local backend without NFS-values) rejects all functions
with `409`. The directory `<storage.location>/volumes` is reserved for the volumes: the file-API
rejects paths into it and doesn't list it.

Onsen only knows the `backend_id` of a volume (`id` above), never the UUID of the volume in
ryokan. So ryokan can switch a volume to another object in the backend without any change in
onsen.

The local backend copies within the onsen in chunks of 1 MiB and skips the chunks, which only
contain zeros, so the copy stays sparse (no `cp`, because the nix-based image contains no
coreutils; a reflink is not used). The copy is written into `<target_id>.img.partial`, which is
renamed to `<target_id>.img` at the end. `volume_info` reports the copy-state from it: `COPYING`
(partial-file and running copy), `READY` (final file) or `FAILED` (partial-file without running
copy, e.g. after a restart of onsen; the partial-file is deleted then, so the next request returns
`404`). A not existing volume is `404`, so `info` has no field `exists`. The copy of the encrypted
file needs no key, so onsen still never sees plain data.

`VolumeConnection` (in `ainari_api_structs::storage_structs`, so sakura uses the same type):

```rust
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VolumeConnection {
    Nfs { server: String, export: String, file_name: String },
}
```

The enum is tagged, so the second iteration only adds a variant (`Rbd`), and sakura rejects an
unknown variant with a clear error instead of misinterpreting it.

### 4.3 Internal API (only ryokan calls it)

| Endpoint                                         | Description                                          |
| ------------------------------------------------ | ---------------------------------------------------- |
| `POST   /v1alpha/volume/internal`                | `{ backend_id, size }` (bytes) → creates the empty volume, `201` |
| `GET    /v1alpha/volume/{backend_id}/internal`   | state (`READY`/`COPYING`/`FAILED`), size, used size; `404` if not existing |
| `PUT    /v1alpha/volume/{backend_id}/size/internal` | grow the volume                                    |
| `DELETE /v1alpha/volume/{backend_id}/internal`   | delete the volume, not existing is no error           |
| `POST   /v1alpha/volume/{backend_id}/copy/internal` | `{ target_backend_id }` → starts the copy in the background, `202` |
| `GET    /v1alpha/volume/{backend_id}/connection/internal` | `VolumeConnection` for sakura                |
| `GET    /v1alpha/backend/internal`               | backend-type, `supports_volumes`, total/free capacity |

The registration at ryokan (`core/ryokan_interaction.rs`) additionally sends `backend_type` and
`supports_volumes` (extension of `HostCreateReq`).

## 5. Changes in ryokan

- Table `volumes` + `database/volume_table.rs` with the status-machine as transactions
  (`try_attach`, `finish_attach`, `start_detach`, `finish_detach`, `start_migration`,
  `finish_migration`, `abort_migration`, `mark_initialized`).
- `onsen_functions::select_volume_host` (random out of the hosts with `supports_volumes`, no
  backup-targets; later by free capacity of `GET /backend/internal`).
- User-API:

| Endpoint                               | Description                                                    |
| -------------------------------------- | -------------------------------------------------------------- |
| `POST   /v1alpha/volume`               | `{ name, size }` → create a volume, also the first step of the creation of a VM with a new volume |
| `GET    /v1alpha/volume`               | list the volumes of the project                                 |
| `GET    /v1alpha/volume/{uuid}`        | get a volume                                                    |
| `PUT    /v1alpha/volume/{uuid}/size`   | grow a volume, only when `AVAILABLE` in phase 1                 |
| `DELETE /v1alpha/volume/{uuid}`        | delete a volume, `409` when attached                            |
| `GET    /v1alpha/volume/count`         | count for the dashboard                                         |

- Internal API for sakura (no other component creates volumes):

| Endpoint                                              | Description                                                                                       |
| ----------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `GET  /v1alpha/volume/{uuid}/internal`                | check of the volume by the create-endpoint of sakura (project, status, `is_initialized`, size)     |
| `POST /v1alpha/volume/{uuid}/attach/internal`         | `{ vm_uuid, sakura_host_uuid }` → `ATTACHING`, returns `VolumeConnection`, `secret_uuid`, `is_initialized` |
| `POST /v1alpha/volume/{uuid}/attached/internal`       | `{ initialized: bool }` → `IN_USE`                                                                 |
| `POST /v1alpha/volume/{uuid}/detach/internal`         | `IN_USE` → `AVAILABLE`, clears the attachment                                                      |
| `POST /v1alpha/volume/{uuid}/migration/internal`      | `{ target_host_uuid }` → sets `migration_host_uuid`, returns the connection for the target          |
| `POST /v1alpha/volume/{uuid}/migration/finish/internal` | `{ success }` → moves `attached_host_uuid` or clears `migration_host_uuid`                        |
| `POST /v1alpha/volume/{uuid}/force_detach/admin`      | admin, for a volume of a dead sakura-host (section 11)                                             |
| `POST /v1alpha/volume/snapshot/internal`              | `{ uuid, name, size, secret_uuid? }` → registers a snapshot-volume with `is_snapshot = true` and status `CREATING`, creates it in onsen; like `init_image_internal` for image-snapshots, with an optional existing secret |
| `PUT  /v1alpha/volume/{uuid}/status/internal`         | `{ status }` → `AVAILABLE` or `ERROR` at the end of the snapshot-task, like `update_image_internal` |
| `POST /v1alpha/volume/{uuid}/restore/internal`        | `{ snapshot_uuid }` → starts the copy of the snapshot in onsen (section 6.4), status `RESTORING`; repeated calls return the state of the copy and, when `READY`, the `VolumeConnection` and `secret_uuid` of the copy |
| `POST /v1alpha/volume/{uuid}/restore/finish/internal` | `{ success }` → switches the volume to the copy or removes the copy (section 6.4)                   |

- The attachment is only allowed for the project of the volume, so a user can't attach the volume of
  another project to their VM.
- Snapshot-volumes:
  - can't be the root-disk of a VM (`409` on create), so a snapshot never changes;
  - can only be attached by the snapshot-tasks of sakura, as write-target while the snapshot is
    created and **read-only** (`attach/internal` with `read_only: true`) for a restore;
  - can't be resized;
  - are listed with `is_snapshot` in `GET /v1alpha/volume` and can be deleted by the user like
    every other volume;
  - count to `max_volume` and `max_volume_size` of the quota, which is checked immediately at the
    registration, like the image-quota for image-snapshots.
- Delete: `DELETING` → onsen-delete → secret in omamori deleted → entry removed. A failed delete
  stays `DELETING` and can be repeated.
- `get_project_resource_count_internal` counts the volumes, so a project with volumes can't be
  deleted.
- `delete_host_admin` is rejected, while volumes exist on the onsen-host.

## 6. Changes in sakura

### 6.1 Attach / detach (`core/volume/…`)

New module with one connector per type of `VolumeConnection` (a trait `VolumeConnector` with
`open` / `close`, which returns the block-device, so the LUKS-part below is shared by all
connectors):

- **NFS**: mount the export once per onsen-host to
  `/var/lib/ainari/sakura/volumes/<onsen_host_uuid>` (`mount -t nfs4 -o hard,noatime`), then
  `losetup --direct-io=on --find --show <file>`.
- Then: `cryptsetup luksFormat` (only on first use, see section 2) and
  `cryptsetup open --key-file=- <dev> ainari-vol-<backend_id>`.
- Detach in the opposite order: `cryptsetup close`, `losetup -d`; the NFS-mount is
  removed, when no volume of the onsen-host is open anymore.
- All steps are idempotent (an already open mapping or an already mapped device is reused), so a
  failed task can be repeated, and they run as steps of the existing tasks of the VM, so they are
  serialized with all other tasks of the VM.
- `DiskConfig` of cloud-hypervisor with `direct: true` for volumes, so no data stays in the
  page-cache of the source during a live migration.

### 6.2 Create VM

`VirtualMachineCreateTaskReq` (sakura) gets `volume_uuid: Option<Uuid>`, `image_uuid` becomes
optional. The create-endpoint checks the request against the reserved `disk_type`, before the task
is queued:

- `LOCAL`: `volume_uuid` must be empty (`400`), `image_uuid` is required.
- `VOLUME`: `volume_uuid` is required (`400`), the volume is read over
  `GET volume/{uuid}/internal` and must be `AVAILABLE` and of the same project (`409` / `404`
  otherwise). A not initialized volume requires `image_uuid`, an initialized one requires an empty
  `image_uuid` (`400`), so it is clear, that the existing content is used and not overwritten.

The flow in `create_ch_virtual_machine.rs`:

| Root-disk                        | Flow                                                                                                                                                                                       |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `LOCAL`                          | unchanged: download image, decrypt, `qemu-img convert` to `boot_disk`, `resize +disk_size` (the value of the vm-type)                                                                       |
| `VOLUME`, not initialized (new)  | attach → format → download and decrypt the image → `qemu-img convert -O raw` directly into `/dev/mapper/ainari-vol-<backend_id>` (volume must be at least the virtual size of the image, otherwise the task fails) → the guest grows its partition with cloud-init |
| `VOLUME`, initialized (existing) | attach the volume, no image is downloaded                                                                                                                                                  |

The seed-image stays a local file, because it is small and is copied on migration like today.

### 6.3 Delete VM (R8)

`delete_ch_virtual_machine.rs`: after the VM is stopped, the volume is detached
(`cryptsetup close`, unmap, `detach/internal` at ryokan), only the local files are removed. If the
detach at ryokan fails, the VM-entry is **not** removed, so the task can be repeated and the
volume is never left in `IN_USE` without a VM.

### 6.4 Other functions, which use the root-disk

- **Snapshots** of a volume-VM are handled like the snapshots of a local VM, but instead of a new
  image, which is marked as snapshot, a **new volume, which is marked as snapshot**, is created:

  | Step                       | Local root-disk (today)                                                       | Volume root-disk                                                                                                                         |
  | -------------------------- | ----------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
  | register (endpoint)        | `init_image_internal` with `is_snapshot = true`, new or given secret           | `volume/snapshot/internal` with the size of the source-volume, new or given secret                                                       |
  | copy (task)                | pause VM → `qemu-img convert` to qcow2 → resume → encrypt → upload into onsen   | attach the new volume → LUKS-format it with its own secret → pause VM → copy raw from `ainari-vol-<source>` into `ainari-vol-<snapshot>` (`qemu-img convert -O raw`, sparse) → resume → detach the new volume |
  | finish                     | image `READY`                                                                  | volume `AVAILABLE` (`status/internal`), `is_initialized = true`                                                                            |
  | failure                    | image is deleted                                                               | volume is set to `ERROR` and deleted (onsen + secret)                                                                                      |

  - `snapshot_save_v1_0` decides by the `disk_type` of the VM, the request (`TaskSnapshotSaveReq`
    with `name` and `secret_uuid`) and the response stay the same. `SnapshotKind` gets the variant
    `Volume { volume_uuid }` beside `Image { image_uuid }`.
  - The plain data only exists in the two mapper-devices on the sakura-host of the VM, both volumes
    stay encrypted in the storage (R4). The snapshot gets its own secret, so deleting the source
    volume or its secret doesn't affect the snapshot.
  - The copy is crash-consistent like today (pause without flush of the guest), the hint to run
    `sync` before stays in the API-description.
- **Restore** (`snapshot_restore_v1_0`) of a volume-VM accepts only a snapshot-volume of the same
  project (an image-snapshot is rejected with `400`, and vice versa for local VMs). The content of
  the snapshot is **not** copied into the volume of the VM. Instead the snapshot is copied within
  onsen and the VM is switched to the copy, so the old content stays untouched until the VM runs
  with the copy, and every step before can be reverted without loss of data:

  1. **Prepare** (sakura): reject a migrating VM, shut the VM down like today.
  2. **Copy** (ryokan → onsen): `restore/internal` generates a new `backend_id` on the onsen-host of
     the snapshot, clones the secret of the snapshot in omamori (`clone_secret`), stores both in
     `restore_*` and starts the copy in onsen. Sakura polls `restore/internal`, until the copy is
     `READY`. If the volume of the VM is bigger than the snapshot, ryokan grows the copy to this
     size, before it is `READY`.
  3. **Switch** (sakura): close the LUKS-mapping of the old object, open the copy with the cloned
     secret as `/dev/mapper/ainari-vol-<new backend_id>`, set it as `root_disk_path` and boot the
     VM.
  4. **Finish** (boot successful): `restore/finish/internal { success: true }` → ryokan moves
     `restore_*` into `host_uuid`, `backend_id` and `secret_uuid` in one transaction, then deletes
     the old object in onsen and the old secret in omamori. The volume keeps its UUID and its name,
     so the VM-entry in sakura and all references of the user stay valid.
  5. **Rollback** (failure in step 2 or 3): close the copy, open the old object again, boot the VM
     with it, `restore/finish/internal { success: false }` → ryokan deletes the copy and the cloned
     secret and clears `restore_*`. The VM runs with its old content like before.

  - "Boot successful" means, that cloud-hypervisor booted the VM (`boot_vm`), like for all other
    boots. A broken guest-OS is not detected without a guest-agent; the snapshot still exists, so
    the restore can be repeated.
  - Crash of sakura or ryokan in between: the `restore_*`-columns are the persistent state. After
    a restart of sakura (`restart_after_host_restart.rs`) a VM, whose volume still has `restore_*`
    set, is rolled back (step 5), because the switch was not confirmed. A failed deletion of the old
    object in step 4 is repeated, it never touches the data of the VM.
  - The copy has the same LUKS-volume-key as the snapshot, because onsen copies the encrypted file
    as it is. With its cloned secret, the lifecycles of both are independent (deleting the snapshot
    doesn't affect the restored volume). A re-encryption of the copy (`cryptsetup reencrypt`) is not
    done, because both belong to the same project.
  - The copy is not counted to the quota, because it replaces the old object. The onsen-host must
    have enough free space for the time of the restore, otherwise the restore is rejected with `409`
    before the VM is shut down. The duration of the copy is accepted for the local backend.
- isekai-backups (`save_backup_disk`, `restore_backup_disk`): work on the mapper-device instead of
  the file; `qemu-img` handles both. Restore writes into the volume, which must be big enough. It should use the same
  switch-flow as the snapshot-restore later (write into a new object, switch on success), which is
  part of the isekai-support in step V8.4.
- `restart_after_host_restart.rs`: open the volumes of all VMs again (NFS-mount, map, LUKS-open with
  the secret of omamori), before the VMs are started. This needs a token, so it uses the same
  mechanism as today for the image-download after a restart.
- Cold migration (`migration/transfer.rs`): for `VOLUME` only the seed-image is transferred, the
  target attaches the volume, after the source detached it.
- Host-resources: only VMs with a local root-disk use disk-space of the host. The reported
  `disk_space` of the host (minus `reserved_disk`) is unchanged, because it is only used for
  local disks.

### 6.5 Live migration

Cloud-hypervisor provides `vm.receive-migration` (target) and `vm.send-migration` (source). Its
stream is not encrypted and contains the memory of the guest, so it must not go plain over the
network (R4).

New steps (as tasks of the VM, in `migration/live.rs`):

1. **target `prepare_live`**: attach the volume with the connection of `migration/internal`
   (`migration_host_uuid`), copy the seed-image from the source (existing transfer), start a
   cloud-hypervisor process and call `receive-migration` on a local unix-socket.
2. **channel**: sakura forwards the unix-socket of the source over a TLS-connection of the internal
   API to the target (new internal websocket/upgrade-endpoint, authenticated with the internal
   API-key and a one-time migration-token from hanami), which writes it into the unix-socket of its
   cloud-hypervisor. So the stream is protected by the same TLS like all internal traffic.
3. **source `send_live`**: `send-migration` to its local unix-socket. Cloud-hypervisor flushes and
   pauses the VM at the end and the target continues it.
4. **finish**: source closes its LUKS-mapping and unmaps the volume, ryokan
   `migration/finish/internal { success: true }`.
5. **failure**: target stops its process and detaches, the source continues the VM (it was not
   stopped), ryokan `migration/finish/internal { success: false }`.

During the steps 1–3 the volume is open on two hosts at the same time, but only the source writes
into it. This is unavoidable for live migration and is limited by the status in ryokan to the two
hosts of the migration.

## 7. Changes in hanami

- vm-types: `disk_size` in `VmTypeCreateReq`, `VmTypeUpdateReq` and the responses, validated
  (`> 0`) like cores and memory.
- `VirtualMachineCreateReq`: `disk_size` is **removed**, new field `disk_type` (`local` /
  `volume`, default `local`). Hanami gets neither a size nor a volume-UUID. The API is
  `v1alpha`, so the field is removed without a compatibility-phase, the CLI, the SDKs and the
  dashboard are changed in the same release.
- `reserve`: takes the `disk_size` of the vm-type for `local` and `0` for `volume` and sends it
  with `disk_type` to the sakura-host (`VirtualMachineInternalCreateReq`).
- The internal reservation (`VirtualMachineReserveInternalReq`, used by isekai for the restore)
  keeps its `disk_size` and gets `disk_type`, because a restored VM must get its original size,
  also if its vm-type was changed in between.
- `select_host` gets the `disk_type`:
  - `local`: filter and allocate cores, memory and the `disk_size` of the vm-type (like today).
  - `volume`: filter and allocate **only cores and memory**, the disk-space of the host is
    ignored, so also a host with a full local disk is selected.
  - `release_host_resources` (failed reservation, delete-watcher, migration) releases the
    allocated `disk_size` of the meta-virtual-machine, which is `0` for volume-VMs.
- Cold migration of a local VM: the target host needs free disk-space like today. A volume-VM
  (cold or live) only needs free cores and memory on the target.
- `core/migration/job.rs`: new variant `LiveMigration` with the steps of section 6.5. The network
  of the target (step 2 of the cold migration) is prepared **before** the handover, so the VM is
  reachable directly after it continues on the target. The gateways are switched after the
  handover (short packet-loss is accepted in phase 1).
- `migrate_virtual_machine_admin`: new flag `live` (default `true` for volume-VMs, rejected with
  `409` for local disks).
- `host_restore.rs`, `delete_watcher.rs`: handle `disk_type`.

## 8. Deployment and other parts of the repo

- `dockerfiles/{debian_based,nix_based}`: sakura needs `cryptsetup` and `nfs-common`; the
  NFS-server (`nfs-kernel-server`) runs on the onsen-host and is set up by the deployer, not in
  the onsen-image (only the test-setup brings its own). Sakura needs the
  device-mapper and loop-devices in its container (`/dev/mapper`, `/dev/loop-control`, already
  privileged because of cloud-hypervisor).
- `example_configs/ainari/onsen.toml`, `sakura.toml` (new section `[volumes]` with
  `mount_path`), config-docs in `docs/deployer/config/`.
- `docker-compose.yml`: onsen with local backend and NFS-export. A kernel-NFS-server in a
  container needs privileges, so the live-migration-tests run in the vagrant-setup
  (`docs/developer/local_testing/vagrant_setup.md`) with two sakura-hosts.
- `deploy/operator`: `Ainari`-CRD gets `onsen.backend` (only `local` with its PVC and the
  NFS-address in this iteration), so the field doesn't have to be changed for Ceph later.
- Clients (CLI, SDKs, dashboard) orchestrate the creation of a VM with a volume:
  1. `--volume-size <GiB>`: `POST /v1alpha/volume` at ryokan, wait until the volume is
     `AVAILABLE`. `--volume <uuid>`: use the existing volume.
  2. Reserve the VM at hanami with `disk_type = volume`.
  3. `create` at the sakura-host with `volume_uuid` (and `image_uuid` for a new volume).
  4. If step 2 or 3 fails, a volume, which was created in step 1, is deleted again, an existing
     volume is never deleted.
- CLI `ainarictl`: `volume create|list|get|resize|delete`, `vm create --volume-size …` /
  `--volume <uuid>` (without both: local disk of the vm-type), `vm-type create|update --disk-size`,
  `vm migrate --live`.
- Go- and Python-SDK: same functions, plus a helper for the flow above.
- Dashboard: volume-tab in the project, disk-size in the vm-type-dialog, disk-selection in the
  VM-create-dialog (local disk of the vm-type / new volume with size / existing volume).
- Docs: architecture-overview, user-docs for volumes, sequence-diagrams
  `docs/developer/sequence_diagrams/volume_attach.md` and `vm_live_migration.md`, `ryokan_db.md`,
  `sakura_db.md`, `hanami_db.md`, `miko_db.md`.

## 9. Phases

The work is split into small steps, which are reviewed and merged one by one. Every step is one
pull-request, contains its own unit-tests and docs (config-docs, `*_db.md`) and leaves the
develop-branch working, so a VM can be created and deleted like before after every step. A step
only depends on the steps in its column "Depends on".

### V1 – onsen: backend-abstraction (no change of the behavior)

| Step | Content                                                                                                                                              | Depends on | Done when                                                                    |
| ---- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- | ---------------------------------------------------------------------------- |
| V1.1 | New config `[storage] backend` + `[storage.local]`, old `storage.location` still readable, example-config, config-docs                               | –          | both config-formats are parsed, all existing tests pass                       |
| V1.2 | Traits `FileBackend` / `VolumeBackend`, enum `Backend`, local `FileBackend` with the functions of `core/file_operations.rs` merged into `core/backend/local.rs`, file- and directory-endpoints use the trait | V1.1       | file-API unchanged (existing http-tests of onsen pass)                        |

### V2 – onsen: volumes of the local backend

| Step | Content                                                                                                                                       | Depends on | Done when                                                                     |
| ---- | --------------------------------------------------------------------------------------------------------------------------------------------- | ---------- | ----------------------------------------------------------------------------- |
| V2.1 | Shared structs (`VolumeConnection`, volume-requests/-responses) in `ainari_api_structs`, client-functions in `ainari_clients`                  | –          | structs serialize as documented (unit-tests for the tagged enum)               |
| V2.2 | Local `VolumeBackend`: create (sparse), delete, resize, info, connection (NFS-values from the config) + internal endpoints                    | V1.2, V2.1 | unit- and http-tests of the endpoints, path-checks like `file_operations`      |
| V2.3 | Copy within the backend (`.partial`-file, copy-states) + `copy/internal`                                                                      | V2.2       | unit-tests of all copy-states, also after a simulated restart                  |
| V2.4 | `backend/internal`, registration sends `backend_type` / `supports_volumes`; ryokan: new columns in `hosts`, stored on registration, shown in the host-admin-API | V2.2 | `host list` shows the backend; an onsen without NFS-values has `supports_volumes = false` |

### V3 – ryokan and miko: volumes as resource

| Step | Content                                                                                                                                                          | Depends on | Done when                                                                     |
| ---- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- | ----------------------------------------------------------------------------- |
| V3.1 | Migration `volumes` (mysql + sqlite, incl. `backend_id`, `is_snapshot`, attachment-, migration- and `restore_*`-columns) + `volume_table.rs` with the status-machine, `ryokan_db.md` | – | unit-tests of all transitions of the status-machine                         |
| V3.2 | miko: `max_volume` / `max_volume_size` in the quota, `miko_db.md`, clients                                                                                        | –          | quota-API returns and updates the new fields                                  |
| V3.3 | User-API create / list / get / count / delete, `select_volume_host`, quota-check, secret in omamori, resource-count of the project, `delete_host_admin`-check       | V2.4, V3.1, V3.2 | volume can be created and deleted over the API; quota and project-delete rejected correctly |
| V3.4 | Resize (only `AVAILABLE`, only grow)                                                                                                                              | V3.3       | http-tests for grow, shrink (`400`) and attached (`409`)                       |
| V3.5 | CLI and SDKs: `volume create / list / get / resize / delete`                                                                                                      | V3.4       | CLI-tests like the other resources                                            |

### V4 – hanami: disk-size in the vm-type and scheduling

| Step | Content                                                                                                                                                                                       | Depends on | Done when                                                                  |
| ---- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- | -------------------------------------------------------------------------- |
| V4.1 | `vm_types.disk_size` (migration, API, validation), CLI / SDKs / dashboard-field for vm-types                                                                                                   | –          | vm-type with disk-size can be created and updated                           |
| V4.2 | `VirtualMachineCreateReq`: `disk_size` removed, `disk_type` added; reserve sends the disk-size of the vm-type; sakura: `disk_type` column; CLI / SDKs / dashboard send no disk-size anymore (only `local` yet) | V4.1 | VM-creation works like before, disk has the size of the vm-type |
| V4.3 | `select_host` and release by `disk_type` (`volume`: only cores and memory), `disk_type` and allocated `disk_size` in the meta-virtual-machines, internal reservation of isekai                   | V4.2       | unit-tests of the scheduler for both cases, release gives back exactly the allocation |

### V5 – sakura: VMs with a volume as root-disk

| Step | Content                                                                                                                                                       | Depends on       | Done when                                                                        |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | -------------------------------------------------------------------------------- |
| V5.1 | ryokan: `GET volume/{uuid}/internal`, `attach` / `attached` / `detach` (project-check, exclusive attachment)                                                     | V3.3             | unit-tests: no double attach, wrong project rejected                              |
| V5.2 | sakura: module `core/volume` with the trait `VolumeConnector`, NFS-connector, LUKS open / close / format (not used yet)                                         | V2.1             | unit-tests of the commands; root-integration-test (`#[ignore]`): cipher-text only in the file |
| V5.3 | Test-environment: `cryptsetup` / `nfs-common` in the sakura-images, NFS-export in docker-compose and vagrant, sakura-config `[volumes]`                        | V2.2             | a sakura-container can mount the export of the test-onsen                         |
| V5.4 | Create VM with a volume: `volume_uuid` / optional `image_uuid` in the create-request, checks against `disk_type` and `is_initialized`, image into a new volume, boot from an existing one, `DiskConfig` with `direct` | V4.3, V5.1, V5.2, V5.3 | E2E 1 (new volume) and 3 (double use) |
| V5.5 | Delete VM detaches the volume (VM-entry only removed after a successful detach)                                                                                | V5.4             | E2E 1 and 2 (volume `AVAILABLE`, reused by a new VM)                               |
| V5.6 | Restart after a host-restart opens the volumes again                                                                                                            | V5.4             | E2E 6                                                                             |
| V5.7 | Client-flow in CLI and SDKs (`--volume-size` / `--volume`, cleanup of an own volume on failure)                                                                 | V3.5, V5.4       | CLI-tests for both options and the cleanup                                        |

### V6 – snapshots of volume-VMs

| Step | Content                                                                                                                                       | Depends on | Done when                                                                                   |
| ---- | --------------------------------------------------------------------------------------------------------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------- |
| V6.1 | ryokan: `volume/snapshot/internal`, `status/internal`, rules of snapshot-volumes (no root-disk, read-only attachment, no resize, quota)        | V5.1       | unit-tests of all rules                                                                     |
| V6.2 | sakura: snapshot-save of volume-VMs (`SnapshotKind::Volume`)                                                                                   | V5.4, V6.1 | E2E 8 (save part): snapshot is a new volume with `is_snapshot`, encrypted                     |
| V6.3 | ryokan: `restore/internal` (copy in onsen, cloned secret, `restore_*`) and `restore/finish/internal` (switch in one transaction, then delete of the old object) | V2.3, V6.1 | unit-tests: switch keeps UUID and name, failure leaves the volume unchanged, second restore `409` |
| V6.4 | sakura: restore of volume-VMs with switch and rollback, rollback after a restart of sakura                                                     | V6.2, V6.3 | E2E 8 (restore part), 9 and 10                                                                |

### V7 – migration of volume-VMs

| Step | Content                                                                                                                                 | Depends on | Done when                                                              |
| ---- | --------------------------------------------------------------------------------------------------------------------------------------- | ---------- | ---------------------------------------------------------------------- |
| V7.1 | Cold migration of volume-VMs: only the seed-image is transferred, attach on the target after the detach on the source, no disk-space on the target | V5.5 | cold migration of a volume-VM in vagrant, file still there |
| V7.2 | ryokan: `migration/internal`, `migration/finish/internal`, `force_detach/admin`                                                          | V5.1       | unit-tests: second host only during a migration                         |
| V7.3 | sakura: TLS-channel for the migration-stream (unix-socket ↔ internal API ↔ unix-socket), one-time migration-token, standalone            | –          | test with a dummy-stream over the channel, rejected without valid token |
| V7.4 | sakura: steps `prepare_live`, `send_live`, finish and rollback as tasks of the VM                                                        | V7.2, V7.3 | manual live migration over the internal endpoints in vagrant            |
| V7.5 | hanami: job-variant `LiveMigration`, flag `live` in `migrate_virtual_machine_admin`, CLI / SDKs                                          | V4.3, V7.4 | E2E 4 and 5                                                             |

### V8 – completion

| Step | Content                                                                                                         | Depends on | Done when                                         |
| ---- | --------------------------------------------------------------------------------------------------------------- | ---------- | ------------------------------------------------- |
| V8.1 | Dashboard: volume-tab, disk-selection in the VM-create-dialog, snapshot-volumes                                  | V5.7, V6.4 | manual test of the dialogs                         |
| V8.2 | Operator: `onsen.backend` in the `Ainari`-CRD (local with PVC and NFS-address), zarf                             | V2.4       | operator-tests with envtest                        |
| V8.3 | Docs: architecture, user-docs for volumes, sequence-diagrams `volume_attach.md` and `vm_live_migration.md`       | V7.5       | docs build without warnings                        |
| V8.4 | isekai: backup and restore of volume-VMs (restore with the switch-flow of V6.4)                                  | V6.4       | backup and restore of a project with a volume-VM   |

After V5 the requirements R2, R4–R9 and the local part of R1 are fulfilled, after V7 also R3. The
second backend of R1 (Ceph) is the next iteration
([onsen_ceph_backend_plan.md](onsen_ceph_backend_plan.md)).

## 10. Tests

- Unit-tests onsen: config-parsing (old and new format), local volume-backend (create sparse,
  delete idempotent, resize only grow, path-checks like `file_operations`), connection-info,
  copy (sparse, copy-states `COPYING`/`READY`/`FAILED`, partial-file after a restart is `FAILED`).
- Unit-tests ryokan: status-machine of `volumes` (no double attach, no delete while attached,
  migration-host only during migration, wrong project rejected), host-selection (no backup-target,
  only `supports_volumes`), quota; snapshot-volumes: can't be the root-disk of a VM, only
  read-only attachment for a restore, no resize, quota is checked at the registration.
  Restore: `finish { success: true }` switches `backend_id`, `host_uuid` and `secret_uuid` in one
  transaction and keeps UUID and name; `finish { success: false }` leaves the volume unchanged;
  a second restore of the same volume during a running one → `409`.
- Unit-tests sakura: `DiskConfig` for volumes, building of the `cryptsetup`/`losetup`/`mount`
  commands, unknown `VolumeConnection`-variant rejected, format only when not initialized and no LUKS-header,
  checks of the create-request (`volume_uuid` vs. `disk_type`, `image_uuid` vs. `is_initialized`).
- Unit-tests hanami: `disk_size` of the vm-type is checked and allocated for local disks;
  for volumes only cores and memory are checked and allocated, a host with a full disk is still
  selected; release gives back exactly the allocated disk-space; `disk_size` is not accepted
  anymore in `VirtualMachineCreateReq`.
- Integration (needs root, marked `#[ignore]` like other host-tests): attach a file over loop +
  LUKS, write, detach, attach again and read; `cryptsetup isLuks` on the raw file is true and the
  written plain-text is not found in the raw file.
- E2E in `testing/local_stack` / vagrant:
  0. Create VM with a local disk → disk has the size of the vm-type and the used disk-space of
     the host grows by it; a volume-VM doesn't change the used disk-space.
  1. Create a volume, create a VM with it and an image, write a file, delete VM → volume
     `AVAILABLE`.
  2. Create a new VM with the same volume and without image → file is still there.
  3. Second VM with the same volume → `409`.
  4. Live migration of a running VM between two sakura-hosts: ping and an open ssh-session keep
     running, file written before the migration is there.
  5. Live migration fails in the middle (target killed) → VM keeps running on the source.
  6. Restart of a sakura-host → volume-VMs are running again.
  7. Delete of an attached volume → `409`; delete of a project with volumes → rejected.
  8. Snapshot of a running volume-VM → new volume with `is_snapshot = true`, no new image; change
     a file in the VM, restore the snapshot → the old content is back; the raw file of the
     snapshot is LUKS-encrypted; a VM with the snapshot-volume as root-disk → `409`; restore of an
     image-snapshot into a volume-VM → `400`.
  9. Restore with a failure after the switch (e.g. boot of the copy fails) → VM runs again with
     its old content, the copy is gone, the volume has its old `backend_id`.
  10. Successful restore → the volume keeps its UUID and name, the old object and its secret are
      deleted; kill sakura during the copy → after its restart the restore is rolled back.

## 11. Open points

- **Data-volumes**: additional volumes beside the root-disk (hot-plug with `vm.add-disk` of
  cloud-hypervisor). The data-model already allows it with a separate attachment-table later.
- **Fencing**: when a sakura-host dies, its volumes stay `IN_USE`. `force_detach/admin` releases
  them, but the admin must make sure, that the host is really off, otherwise two hosts write into
  the same volume. Automatic fencing comes with the Ceph-backend (blocklisting of the dead host).
- **Encryption of local disks**: local root-disks are still not encrypted on the sakura-host
  (today's behavior). They could use the same LUKS-code later.
- **Snapshots of volumes in the backend** instead of copying the whole disk on the sakura-host, for
  faster snapshots and isekai-backups (with RBD-snapshots in the Ceph-backend). The
  snapshot-volume stays the resource in ryokan, only the copy moves into onsen.
- **New VM from a snapshot-volume**: a snapshot can't be the root-disk itself. Cloning it into a
  new volume with the copy-endpoint of onsen (like the restore), which is then used as existing
  volume, is left for later.
- **Redundancy of the local backend**: none by design, the onsen-host and its disk are a single
  point of failure. Production-setups use the Ceph-backend of the next iteration.
- **NFS-security**: the export is restricted to the network of the sakura-hosts by the deployer.
  Because only cipher-text is stored, this is accepted for the test-backend; Kerberos (`sec=krb5p`)
  is not planned.
- **Online resize** of an attached volume (`cryptsetup resize` + `vm.resize-disk`).
- **Ceph** as second backend: [onsen_ceph_backend_plan.md](onsen_ceph_backend_plan.md).
