# Plan: Isekai – backup- and restore-orchestrator

Isekai (異世界, "another world") is the new component in `src/binaries/isekai`, which backs up and
restores single virtual machines and whole projects. Isekai **doesn't store any data itself**. It
only orchestrates the other components, collects their metadata and pushes it back on restore. The
result of a backup is one self-contained directory on a dedicated backup-onsen, which contains the
metadata of all components and the encrypted disks, and which can also be imported into another
ainari-deployment for disaster-recovery.

## 1. Decisions

| Topic                | Decision                                                                                                                                                                                                                                                                                                                                                 |
| -------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Storage of bundles   | On onsen-hosts, which an admin marked in ryokan as **backup-target**. Only these hosts are used by isekai, and they are never used for normal images and snapshots. A host can only be marked, when it holds no images or snapshots.                                                                                                                      |
| Backups in ryokan    | A backup is a **separate resource** in ryokan with its own table, not a new kind of image. A bundle consists of multiple files and metadata and must stay consistent, so ryokan tracks and handles it only as one atomic package (create → write → finalize → read only → delete as a whole), never its single files. The `images`-table and its `is_snapshot` stay unchanged.                                  |
| Encryption of bundle | A **backup-passphrase** is provided by the user for each backup. A key is derived from it (Argon2id) and wraps all secrets of the project (incl. the secrets of the disks) and encrypts the metadata. The disk-files stay encrypted like they already are in the onsen. On import omamori unwraps the secrets and re-encrypts them with its own key.         |
| Users                | Users and their project-mappings are **not** part of a backup, because a user can be member of multiple projects. Only the project itself (incl. quota) is restored. Creating users and assigning them to the restored project is done by an admin over the existing API of miko.                                                                          |
| Identities           | **Without `force`**: UUIDs and internal IPs are kept when free, otherwise new UUIDs are generated and all references are rewritten through a mapping-table. Floating IPs are always newly allocated from the pool of the target. **With `force`**: existing resources with the same UUID are overwritten, so the restored state keeps the original UUIDs. |
| Prune                | With the additional flag `prune`, resources which exist in the target project, but not in the backup, are deleted, so the project is reset exactly to the state of the backup. Default `false`.                                                                                                                                                          |
| Project-lock         | The project is locked in miko for the whole backup and the whole restore, so the disks and the metadata of a backup are consistent and a restore can't be disturbed. All write-requests of users to the project are rejected in all components, read-requests stay allowed. The running VMs are not affected.                                              |
| Disk-data            | Reuses the existing snapshot-functions of sakura (`save_ch_virtual_machine` / `restore_ch_virtual_machine`) and the task-queue of the VM, so a backup can't overtake other tasks of the same VM.                                                                                                                                                          |

## 2. Scope of a backup

| Component | Content                                                                                       | Note                                                                         |
| --------- | --------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| miko      | project, quota                                                                                 | no users, no user-project-mappings, no tokens                                |
| omamori   | secrets (`owned_by` user and internal ones of images/snapshots), public-keys                   | payloads only wrapped with the backup-key, never in plain text                |
| ryokan    | images and snapshots of the project (metadata + file)                                          | files are copied as they are (already encrypted)                              |
| hanami    | networks, vm-types, meta-virtual-machines, addresses (IP/MAC), network-filters, floating-ips   | floating IP only as "was attached to VM X", not the address itself as a requirement |
| sakura    | VM-config (cores, memory, disk-size, image, public-key, network, IP, MAC, power-state) + a new root-disk-snapshot | seed is regenerated on restore                                                 |
| torii     | –                                                                                               | derived state, is configured again by hanami (like `host_restore.rs`)        |
| izakaya   | –                                                                                               | MLS-keys belong to the hosts, not to the project                              |
| omikuji   | –                                                                                               | only new audit-log entries for backup/restore                                 |

A **VM-backup** contains only the VM, its root-disk, its public-key, its network-filters and a
reference to its network/image (UUID + name). On restore the network must exist in the target
project, or it is created from the backup, when the network-metadata is included (option
`include_dependencies`, default `true`).

## 3. Layout of the backup-directory

Location on the backup-onsen: `<storage.location>/backups/<backup_uuid>/`

```text
<backup_uuid>/
├── manifest.json            # plain: format-version, ainari-version, kind (project|vm),
│                            # project-id, created_at, kdf-params + salt, list of all files with
│                            # sha256 and size; authenticated with HMAC (key from HKDF)
├── metadata/                # every file AES-256-GCM encrypted with the metadata-key
│   ├── miko.json.enc
│   ├── omamori.json.enc     # secrets: payload wrapped with the wrap-key
│   ├── ryokan.json.enc
│   ├── hanami.json.enc
│   └── sakura.json.enc
├── images/
│   └── <image_uuid>.enc     # images and user-snapshots of the project, copied unchanged
└── disks/
    └── <vm_uuid>.enc        # root-disk-snapshots created for this backup
```

- Keys: `master = Argon2id(passphrase, salt)`, then `HKDF(master)` → `metadata_key`, `wrap_key`,
  `manifest_mac_key`. A wrong passphrase is detected by the manifest-HMAC, before anything is
  changed in the target.
- Each component-file contains `{ "schema_version": n, "data": … }`, so an older bundle can be
  migrated by the component on import.
- The disk-files are encrypted with their own secrets (like every image in the onsen). These
  secrets are part of `omamori.json.enc`, so the bundle is self-contained.
- The directory can be copied 1:1 (rsync, tape, …) to the backup-onsen of another deployment and
  imported there.
- The whole directory belongs to exactly one backup-resource in ryokan (see 4.1). Ryokan doesn't
  know the single files of the bundle, only the package as a whole.

## 4. Changes in the existing components

### 4.1 ryokan – backup-targets

- `host_table`: new column `is_backup_target BOOL NOT NULL DEFAULT false` (diesel-migration).
- New admin-endpoint `PUT /v1alpha/host/{host_uuid}/backup_target/admin` with
  `{ "is_backup_target": bool }` (like `isolation/admin` of hanami)
    - `true` is rejected with `409`, if any image or snapshot exists on the host. The check and
      the update run in one transaction.
    - `false` is rejected with `409`, if backups still exist on the host (directory
      `backups/` on the host isn't empty, checked with `ListDirectory`, so ryokan doesn't depend
      on isekai). With B5 additionally, if a not deleted entry in the `backups`-table references
      the host. The directory-check stays, because it also covers foreign bundles, which were
      copied onto the host, but aren't registered yet.
- The host-selection of `init_image_internal` / `create_image` ignores backup-targets. Adding an
  image on a backup-host is a hard error, not only a filter: `add_image` checks it in the same
  transaction, so a host, which was marked after it was selected, still gets no image.
- `list_host_admin` / `get_host_admin` show the flag.

#### Backups as own resource

The `images`-table stays as it is (`is_snapshot` is kept, there is no new image-kind). A backup is
a new, separate resource with its own table `backups`, because a bundle consists of multiple files
(disks, images, metadata, manifest) and is only consistent as a whole. Ryokan tracks the package,
not its single files, and only allows operations, which keep the package consistent.

Table `backups` (diesel-migration for mysql and sqlite, documented in
`docs/deployer/database/ryokan_db.md`):

| Column            | Type                    | Description                                                                                                  |
| ----------------- | ----------------------- | ------------------------------------------------------------------------------------------------------------ |
| `uuid`            | `VARCHAR(40)` PK        | UUID of the backup; the same UUID like in isekai and the name of the directory `backups/<uuid>/`              |
| `host_uuid`       | `VARCHAR(40)`           | backup-target (`hosts.uuid`), on which the bundle is stored. The onsen-address is resolved over the host.   |
| `directory_path`  | `TEXT`                  | path of the bundle relative to the storage-location of the onsen (`backups/<uuid>`)                          |
| `backup_status`   | `VARCHAR(16)`           | state of the package: `CREATING`, `READY`, `DELETING`, `ERROR`                                               |
| `file_count`      | `INTEGER`               | number of files in the bundle, set by `finalize` / `register`                                                |
| `total_size`      | `BIGINT`                | size of all files of the bundle in bytes, set by `finalize` / `register`                                     |
| `manifest_sha256` | `VARCHAR(64)` nullable  | sha256 of `manifest.json` at `finalize` / `register`, so a later change of the sealed bundle can be detected |
| `owner_id`        | `VARCHAR(256)`          | user, who created the backup                                                                                 |
| `project_id`      | `VARCHAR(256)`          | project of the backup                                                                                        |
| `status`          | `VARCHAR(8)`            | common status-column like in all tables                                                                      |
| `created_at`, `created_by`, `updated_at`, `updated_by`, `deleted_at`, `deleted_by` | like in all tables | soft-delete and audit-fields                              |

Name, kind (project/VM), VM-UUID, ainari-version and the job-state stay in isekai (5.1). Ryokan
only owns the storage-side of the backup, so both tables don't duplicate each other.

Lifecycle of the package (`backup_status`):

- `CREATING`: created by isekai at the start of a backup. Ryokan selects the backup-target (same
  selection like for images, but only backup-targets) or uses the host given by isekai. Files can
  only be written into the bundle in this state.
- `READY`: set by `finalize`. Ryokan gets size and sha256 of all files over `ListDirectory` /
  `GetFileInfo`, stores `file_count`, `total_size`, `manifest_sha256` and seals the bundle: from
  now on it is read only. A bundle without `manifest.json` can't be finalized. Files can only be
  read and copied out of the bundle in this state.
- `DELETING`: the bundle is only deleted as a whole. The state is set before `DeleteDirectory`,
  then the row is soft-deleted. A failed deletion stays in `DELETING` and can be repeated; such a
  bundle can't be read anymore.
- `ERROR`: the bundle is incomplete or inconsistent (e.g. finalize found a missing manifest). It
  can only be deleted.

State-check and state-change run in one transaction, so e.g. a write can't happen into a bundle,
which is finalized at the same time. Backups don't count to the image-quota and don't appear in
`list_image` / `get_image`, because they are in their own table.

New internal endpoints for isekai and sakura (ryokan is the only component, which talks with the
onsen about the bundles, so isekai doesn't need a wireguard-tunnel):

- `POST   /v1alpha/backup/internal` `{ uuid, project_id, host_uuid? }` – creates the package in
  state `CREATING`
- `GET    /v1alpha/backup/{backup_uuid}/internal` and `GET /v1alpha/backup/internal?project_id=…`
- `POST   /v1alpha/backup/{backup_uuid}/finalize/internal` – `CREATING` → `READY`
- `DELETE /v1alpha/backup/{backup_uuid}/internal` – deletes the whole bundle
- `PUT    /v1alpha/backup/{backup_uuid}/file/internal` – writes a metadata-file or the manifest
  into the bundle (only `CREATING`)
- `GET    /v1alpha/backup/{backup_uuid}/file/internal` – reads a metadata-file or the manifest
  from the bundle (only `READY`)
- `POST   /v1alpha/backup/{backup_uuid}/disk/internal` `{ vm_uuid }` – returns the onsen-address
  and the path `disks/<vm_uuid>.enc` for the upload of a root-disk by sakura (only `CREATING`).
  The disk is no image, so no entry in the `images`-table is created.
- `POST   /v1alpha/backup/{backup_uuid}/copy/internal` – copies a file between onsens (image →
  bundle on backup, only `CREATING`; bundle → normal onsen on restore, only `READY`). Ryokan
  streams it chunk by chunk from the source to the target, without storing it.
- `GET    /v1alpha/backup_target/{host_uuid}/scan/internal` – lists the directories in `backups/`,
  which are not registered in the `backups`-table (for the import of foreign bundles)
- `POST   /v1alpha/backup/register/internal` `{ uuid, project_id, host_uuid }` – registers a
  foreign bundle, which was found by `scan`, directly as `READY` (same checks like `finalize`)
- `GET  /v1alpha/project/{project_id}/export/internal` and
  `POST /v1alpha/project/{project_id}/import/internal` (metadata of the images, without backups)

### 4.2 onsen – gRPC (`onsen_upload.proto`)

- New RPCs: `ListDirectory(path)`, `DeleteDirectory(path)`, `GetFileInfo(path)` (size + sha256).
  `is_safe_subpath` is used for all of them like for the existing RPCs.
- Optional, if the copy-performance over ryokan isn't enough: `PushFile(path, target_onsen,
  target_path)` for direct onsen-to-onsen transfer. This needs additional wireguard-peers between
  the onsens, so it is not part of the first version.

### 4.3 sakura

- New internal endpoint `POST /v1alpha/virtual_machine/{uuid}/backup/internal`
  (`TaskBackupSaveReq { backup_uuid, secret_uuid }`). It does the same as `snapshot_save_task`,
  but registers no image in ryokan. Instead it gets the upload-target inside the bundle over
  `POST backup/{backup_uuid}/disk/internal` and uploads the encrypted root-disk there. The
  secret is generated by isekai in omamori (`owned_by = "backup"`) and passed as `secret_uuid`.
  The task-variant `CloudHypervisorVirtualMachineSnapshot` gets a field
  `kind: SnapshotKind` (`Image { image_uuid }` | `BackupDisk { backup_uuid }`) instead of a new
  task. `save_ch_virtual_machine` keeps the copy/encrypt/upload-logic; only the image-specific
  parts (`get_image` / `update_image_status`) move into the caller, so it gets the upload-target
  (onsen-address + path) and the secret as arguments.
  Option `stop_vm`: the VM is shut down before the snapshot and started again afterwards
  (application-consistent instead of only crash-consistent).
- `GET /v1alpha/virtual_machine/{uuid}/export/internal` – config of the VM for `sakura.json`.
- Restore uses the existing `restore_ch_virtual_machine` for existing VMs (`force`). New VMs are
  created over the existing creation path of hanami/sakura, which already supports a snapshot as
  boot-image (see 5.3).
  `restore_ch_virtual_machine` stays unchanged: the disk is copied out of the bundle and
  registered as normal snapshot-image (`is_snapshot`) before, so it never reads from a bundle.

### 4.4 miko

- `GET  /v1alpha/project/{project_id}/export/internal` – project and quota.
- `POST /v1alpha/project/{project_id}/import/internal` – creates the project with its quota, or
  updates name and quota of an existing project with the same ID with `force`. Users and
  mappings are never touched.
- `get_endpoints` gets the address of isekai (`EndpointsResp.isekai`).
- **Project-lock**:
    - `project_table`: new nullable columns `locked_by_job` (UUID of the backup- or restore-job),
      `lock_reason` (`BACKUP` | `RESTORE`) and `locked_at`.
    - `POST   /v1alpha/project/{project_id}/lock/internal` `{ job_uuid, reason }` – sets the lock.
      Rejected with `409`, if the project is already locked, so two backups/restores of the same
      project can't run at the same time.
    - `DELETE /v1alpha/project/{project_id}/lock/internal` `{ job_uuid }` – removes the lock, only
      with the same job-UUID.
    - `DELETE /v1alpha/project/{project_id}/lock/admin` – manual unlock by a global admin, e.g.
      after a failed restore.
    - `validate_token` returns the lock-state of the project in the `UserContext`
      (`project_locked: bool`).
- `ainari_api` (auth-middleware, shared by all services): requests with a method other than
  `GET` to a locked project are rejected with `423 Locked`. Requests with the internal API-key
  (from isekai and between the components during the restore) are not blocked. This way the
  lock is checked in one central place and doesn't have to be implemented in every endpoint.

### 4.5 omamori

- `POST /v1alpha/project/{project_id}/export/internal` with `{ wrap_key }` – returns all secrets of
  the project with the payload wrapped by `wrap_key` (AES-256-GCM, new nonce per secret) and the
  public-keys. The plain payload never leaves omamori.
- `POST /v1alpha/project/{project_id}/import/internal` with `{ wrap_key, secrets, public_keys,
  force, id_map }` – unwraps the payloads and stores them encrypted with its own key (`simple_crypto`).
- `generate_secret_internal` gets `owned_by = "backup"` for the secrets of the disks inside a
  backup, which can't be deleted by the user. Isekai creates them for each disk and deletes them
  together with the backup.
- The `wrap_key` is only transmitted over the internal TLS-connection with the internal API-key
  and is never logged or stored.

### 4.6 hanami

- `GET  /v1alpha/project/{project_id}/export/internal` – networks, vm-types, meta-VMs, addresses,
  network-filters, floating-ips (+ the VM they are attached to).
- `POST /v1alpha/project/{project_id}/import/internal` – networks, vm-types, network-filters. The
  VMs are not created here, but by the normal reserve-flow (see 5.2), so the scheduling of the
  sakura-host and the configuration of torii are reused.
- Floating-ips are allocated again from the pool of the target and attached to the restored VM.
  The old address is only a preference.
- Torii-state is not restored explicitly. It results from the restored networks/VMs, like after
  `host_restore`.

### 4.7 shared libraries

- `ainari_api_structs`: new `backup_structs.rs` with requests/responses of isekai and the
  export-/import-structs of every component (`MikoProjectExport`, `OmamoriProjectExport`, …).
  Isekai only uses these typed structs, so the reference-rewriting (`id_map`) is checked by the
  compiler.
- `ainari_clients`: `backup.rs` (isekai), plus client-functions for the new internal endpoints of
  every component.
- `ainari_common`: generic `encrypt_bytes`/`decrypt_bytes` in `encryption.rs` (AES-256-GCM),
  used for the metadata-files.
- `ainari_common`: generic key-derivation in `kdf.rs` (Argon2id from a passphrase into a
  master-key, HKDF into single keys by label), with zeroize of the keys after usage. It knows
  nothing about backups, so it can be reused in other contexts. The backup-specific labels
  (`ainari-backup-v1 metadata-key`, …) are defined in isekai (`core/keys.rs`).

## 5. Isekai

### 5.1 Structure

Same structure like the other services (actix-web + apistos, diesel, config via `CONFIG_FILE`):

```text
src/binaries/isekai/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── config.rs                # miko-endpoint, database, internal api-key, worker-count,
    │                            # argon2-parameters, max parallel snapshots
    ├── api/
    │   ├── http_server.rs
    │   ├── routes/v1alpha.rs
    │   └── http_endpoints/
    │       ├── backup/          # create_project_backup, create_vm_backup, get, list, delete
    │       ├── restore/         # restore_backup, get_restore, list_restore
    │       └── import/          # scan_backup_target_admin, import_backup_admin
    ├── core/
    │   ├── backup/              # job-steps of the backup
    │   ├── restore/             # job-steps of the restore, id_map, conflict-check
    │   ├── keys.rs              # labels and derivation of metadata_key, wrap_key,
    │   │                        # manifest_mac_key over ainari_common::kdf
    │   ├── manifest.rs          # format, versioning, HMAC, sha256
    │   └── processing/          # job-queue + worker, like the task-queue of sakura
    └── database/
        ├── backup_table.rs      # uuid, name, kind, project_id, vm_uuid, state, ainari_version,
        │                        # created_*, deleted_* (host and size come from the
        │                        # backup-resource in ryokan with the same uuid)
        ├── job_table.rs         # uuid, type (BACKUP|RESTORE), backup_uuid, state, step,
        │                        # messages, options, started/finished
        └── id_map_table.rs      # job_uuid, resource_type, old_uuid, new_uuid
```

Jobs are asynchronous like the tasks in sakura: the endpoint returns a job, which can be
polled. Following the design-decision "Sakura task abort after restart", an active job is set
to `ERROR` after a restart of isekai and isn't resumed. For a backup-job the partial bundle is
deleted and the project is unlocked again, because the project itself wasn't changed. For a
restore-job the project stays locked (see 5.3).

### 5.2 Flow of a project-backup

1. `POST /v1alpha/backup` `{ name, passphrase, stop_vms, include_images }` → check permission
   (project-admin or global admin), create a job and the backup-resource in ryokan
   (`POST backup/internal`, state `CREATING`), which selects the backup-target.
2. **Lock** the project in miko with the UUID of the job (`lock/internal`, reason `BACKUP`). If
   the project is already locked (other backup or restore running), the backup is rejected with
   `409`. From now on no resources can be created, changed or deleted in the project, so the
   set of VMs, images, secrets, networks, … can't change between the snapshots and the
   collection of the metadata. Power-actions of the users (start/stop/reboot) are blocked too.
   Only isekai itself (internal API-key) can stop/start the VMs for `stop_vms`.
3. Derive the keys from the passphrase (Argon2id, random salt). Keep the passphrase/keys only in
   memory of the job.
4. For every VM of the project: generate a secret in omamori (`owned_by = "backup"`), then
   `POST sakura …/backup/internal` → wait for the tasks (with a limit of parallel snapshots per
   sakura-host). The disks land directly in the bundle on the backup-target. `stop_vms` is
   passed to sakura as `stop_vm` for each VM:
   - `stop_vms=false`: the VMs keep running and are only paused shortly by the
     snapshot-function of sakura. The backup is crash-consistent (like a power-loss), data which
     an application still holds in memory is not part of the backup, but the VMs stay available.
   - `stop_vms=true`: each running VM is shut down before its snapshot and started again directly
     afterwards. The backup is application-consistent, but the VMs are unavailable during their
     snapshot and the project-lock is held longer. VMs which were already stopped are not
     started.
5. When `include_images`: copy all images/snapshots of the project into `images/` over ryokan.
6. Collect the metadata: miko (project, quota) → omamori (with `wrap_key`) → ryokan → hanami →
   sakura. Because of the lock, the metadata matches exactly the state of the disks. Images or
   secrets, which are referenced but missing, are still checked as safety-net → job fails.
7. Encrypt each component-file, write it over ryokan into `metadata/`.
8. Write `manifest.json` last (sha256 of all files over `GetFileInfo`, HMAC). A bundle without a
   valid manifest counts as incomplete and is ignored on scan/import.
9. **Finalize** the backup-resource in ryokan (`finalize/internal`, `CREATING` → `READY`), so the
   bundle is sealed and read only from now on.
10. **Unlock** the project (`lock/internal`), backup-state `READY`, audit-log in omikuji.
11. **Errors**: the backup-resource is deleted in ryokan as a whole (`DELETE backup/internal`,
    removes the partial bundle incl. the disks), the disk-secrets of the job are deleted in
    omamori, VMs which were stopped for `stop_vms` are started again and the project is unlocked.
    Unlike a failed restore, the project itself wasn't changed, so it doesn't have to stay locked.

Deleting a backup deletes the backup-resource in ryokan (whole bundle), the disk-secrets in
omamori and the entry in isekai.

The lock is held as short as possible: with `include_images=false` and without `stop_vms` it
lasts mainly for the copy of the root-disks in sakura. The VMs themselves keep running during
the backup (only paused shortly by the snapshot-function of sakura).

A VM-backup is the same with only one VM and the reduced scope from chapter 2. It locks the
whole project as well, because the backup also contains the network, filters and public-key of
the VM, which belong to the project. Because only one disk is copied, this lock is short.

### 5.3 Flow of a restore

`POST /v1alpha/backup/{backup_uuid}/restore`
`{ passphrase, force, prune, target_project_id?, start_vms }`

1. **Verify**: read the manifest, check the HMAC (wrong passphrase → `400` before any change),
   check the sha256 of all files, check `format_version`/`ainari_version` for compatibility.
2. **Plan** (dry-run, also available as `?dry_run=true`): get the existing resources of the target
   project over the export-endpoints and calculate the `id_map`:
   - `force=false`: UUID free → keep it; taken → new UUID, rewrite all references (VM → network,
     image, public-key; image → secret; filter → VM; floating-ip → VM). Internal IP free in
     the network → keep it, otherwise a new one.
   - `force=true`: every UUID is kept. Existing resources with the same UUID are overwritten.
   - `prune=true`: all resources of the target project, which are not in the backup, are added
     to the plan as "to delete". The dry-run shows them, so the user can check them before.
     `prune` is only allowed together with `force`, because without `force` the target can
     contain remapped resources and the result would not be the state of the backup.
   - Floating IPs: always newly allocated. Quotas are checked before starting.
3. **Lock**: create the project in miko if it doesn't exist yet and lock it with the UUID of the
   job (`lock/internal`, reason `RESTORE`). If the project is already locked (other backup or
   restore running), the restore is rejected with `409`.
   From now on users can only read the project.
4. **Prune** (only with `prune=true`): delete the resources from the plan in the reverse order of
   the dependencies (floating-ips → network-filters → VMs → vm-types → networks → images →
   public-keys/secrets). The internal secrets of images are deleted together with their images.
5. **Apply** in the order of the dependencies:
   1. miko: project, quota
   2. omamori: secrets (unwrap → re-encrypt), public-keys
   3. ryokan: copy images back to normal onsens and register them. The disk-files are copied out
      of the bundle (`copy/internal`) to a normal onsen and registered as snapshot-images of the
      project. The bundle itself stays sealed and untouched, so it can be restored again.
   4. hanami: networks, vm-types
   5. VMs:
      - new VM (or `force` and the existing VM doesn't fit anymore, e.g. other disk-size): delete
        the old one with `force`, then create it over the existing creation-path of hanami with
        the restored snapshot as boot-image and the original UUID/IP/MAC (new optional fields in
        the reserve-request, internal only).
      - `force` and the VM exists with the same config: reuse the existing
        `snapshot_restore_task` of sakura, which replaces the root-disk.
   6. hanami: network-filters, floating-ips
   7. Start the VMs according to their saved power-state or `start_vms`.
6. **Unlock**: remove the lock of the project (`lock/internal`) and store the `id_map` in the
   job, so the user can see which UUIDs changed.
7. **Users**: a restored project has no members, if it was newly created. An admin creates the
   users if required and assigns them over the existing miko-endpoints (`create_user_admin`,
   `set_project_role_admin`).
8. **Errors**: steps are idempotent. A failure leaves the job in `ERROR` with the last step and the
   already created resources (listed in the job). The project **stays locked**, so nobody works
   on a half restored project. It is unlocked again by a successful second restore of the same
   backup with `force` (which takes over the lock of the failed job) or manually by an admin
   (`lock/admin`). The same happens with an active restore-job after a restart of isekai.
   Rollback of a partial restore is optional (phase G1).

### 5.4 Import of a foreign bundle (disaster-recovery)

1. The operator copies the directory into `backups/` of a backup-target of the new deployment.
2. `GET /v1alpha/backup/scan/admin?onsen_host_uuid=…` → lists the bundles with a valid manifest,
   which are not known in isekai.
3. `POST /v1alpha/backup/import/admin` `{ onsen_host_uuid, backup_uuid, passphrase }` → verifies
   the bundle and registers it in the backup-table of isekai and as backup-resource in ryokan
   (`backup/register/internal`, directly `READY`). Afterwards it is a normal backup, which can be
   restored with 5.3.
4. Import is global admin only, because the restore can create projects. The users of the old
   deployment are not part of the bundle and have to be created and assigned by the admin.

### 5.5 Permissions

| Action                                     | Role                                                                         |
| ------------------------------------------ | ---------------------------------------------------------------------------- |
| VM-/project-backup, list, get              | project-member / project-admin of the project (observer: read only)          |
| delete backup                              | project-admin                                                                |
| restore into the existing project          | project-admin                                                                |
| restore, which creates a new project       | global admin                                                                 |
| `force`, `prune`                           | project-admin for its own project, global admin otherwise                    |
| manual unlock of a project                 | global admin                                                                 |
| scan / import of foreign bundles, backup-targets | global admin                                                           |

Isekai uses the token of the user for all calls, like sakura does for the snapshot. The internal
endpoints additionally require the internal API-key.

## 6. Deployment and other parts of the repo

- `Cargo.toml`: add `src/binaries/isekai` to `members` and `default-members`.
- `example_configs/ainari/isekai.toml` (tested by the CI like the other examples).
- `dockerfiles/{debian_based,nix_based}`: new target `isekai` in `Dockerfile_services` / nix-flake.
- `docker-compose.yml`: isekai-service + a second onsen as backup-target.
- `deploy/operator`: isekai-deployment, `Ainari`-CRD extended by isekai and a backup-onsen with its
  own volume (`backupStorage`). No wireguard-peer is needed for isekai. Zarf-package: add image.
- `scripts/`: OpenAPI-collector and docker-build-script extended.
- CLI `ainarictl`: `backup create|list|get|delete`, `backup restore`, `backup scan|import`,
  `onsen-host set-backup-target`. Passphrase only interactive or from a file/env, never as an
  argument (shell-history).
- Go- and Python-SDK: same functions.
- Dashboard: backup-tab in the project and on the VM, restore-dialog with dry-run-result.
- Docs: architecture-overview (new component), user-docs, sequence-diagrams for backup/restore
  in `docs/developer/sequence_diagrams/`, entry in `repo_structure.md`.

## 7. Tests

- Unit-tests: manifest (HMAC, wrong passphrase, manipulated file), key-derivation, id_map-
  calculation (conflicts, `force`), reference-rewriting of all structs.
- Unit-tests of the prune-plan (only resources missing in the backup, `prune` without `force`
  rejected) and of the lock in the auth-middleware (`423` for writes, internal API-key passes,
  reads allowed).
- Unit-tests in ryokan: backup-target can't be set with images or registered backups, normal
  images never land on a backup-target, backups are only created on backup-targets, the
  state-machine of the package (no write after `finalize`, no read before `finalize`, no
  `finalize` without manifest, delete only as a whole, failed delete stays `DELETING`).
- Omamori: wrap/unwrap round-trip, plain payload not in the export-response.
- E2E in `testing/local_stack` (like `vm_lifecycle_test.py`):
  1. Project with 2 networks, 2 VMs, filters, floating IP, secret, own image → backup.
  2. Write a file in the VM, then restore with `force` → file is gone, UUIDs are the same.
  3. Create an additional VM, then restore with `force` + `prune` → the additional VM is deleted.
  4. During a restore, a user tries to create a VM in the project → `423`, listing still works.
     The same during a backup; a second backup or a restore of the project during a running
     backup → `409`; after a failed backup the project is unlocked again.
  5. Restore into a new project without `force` → new UUIDs, VMs reachable.
  6. Delete the project completely, then restore → all resources are back, the admin assigns the
     users again.
  7. Copy the bundle into a second stack (DR) → scan, import, restore, create and assign a user,
     log in and access the VMs.
  8. Wrong passphrase / manipulated file → rejected before any change and without a lock.

## 8. Phases

Each phase is **exactly one commit**, which can be reviewed on its own. Rules for every phase:

- It touches as few components as possible (mostly one) and has one clear purpose.
- After the commit `cargo build`, `cargo clippy`, `cargo test` and the existing CI are green. New
  code, which isn't used yet, is fine; broken or half wired code isn't.
- The client-functions in `ainari_clients` and the structs in `ainari_api_structs` are added in
  the same commit as the endpoint they belong to, so each commit is complete in itself.
- New endpoints come with their unit-tests and their OpenAPI-description in the same commit.
- Diesel-migrations are only in the commit, which needs the column.
- The commit-message follows the conventional-commits of `commitlint.config.cjs`; the proposed
  message is listed with each phase.

### Block A – shared crypto

| #   | Commit                                                       | Content                                                                                                                                      |
| --- | ------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------- |
| A1  | `feat(ainari_common): add key-derivation from passphrases`   | `kdf.rs`: generic Argon2id with configurable and limited parameters + HKDF by label, random salt, zeroize. Unit-tests with test-vectors. |
| A2  | `feat(ainari_common): add encryption of in-memory data`      | `encryption.rs`: `encrypt_bytes` / `decrypt_bytes` (AES-256-GCM, random nonce, optional associated data, same layout like the encrypted files). Isekai binds each metadata-file to its file-name over the associated data. Unit-tests incl. manipulated ciphertext. |

### Block B – onsen and ryokan: backup-targets

| #   | Commit                                                        | Content                                                                                                                                                                       |
| --- | ------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| B1  | `feat(onsen): add rpcs for directory-listing and file-info`   | `ListDirectory`, `DeleteDirectory`, `GetFileInfo` (size + sha256) in `onsen_upload.proto`, server-side with `is_safe_subpath`, client-functions in `onsen_file_transfer.rs`. |
| B2  | `feat(ryokan): add backup-target flag to onsen-hosts`         | Column `is_backup_target` + migration, shown in `list_host_admin` / `get_host_admin`. No behaviour-change yet.                                                                 |
| B3  | `feat(ryokan): add admin-endpoint to mark backup-targets`     | `PUT /host/{uuid}/backup_target/admin`: `409` if images exist on the host (enable), or if `backups/` on the host isn't empty (disable, checked with `ListDirectory`).    |
| B4  | `feat(ryokan): exclude backup-targets from image-placement`   | Host-selection of `init_image_internal` / `create_image` skips backup-targets, a backup-target as target is a hard error. Tests.                                              |
| B5  | `feat(ryokan): add backups as own resource`                   | Table `backups` + migration (mysql + sqlite), `ryokan_db.md`. `POST backup/internal` (only on backup-targets), `GET`/list, `POST …/finalize/internal`, `DELETE …/internal` (whole bundle over `DeleteDirectory`), state-machine `CREATING`/`READY`/`DELETING`/`ERROR` in transactions. Disable of a backup-target additionally checks the table. Tests. |
| B6  | `feat(ryokan): add file-access to backups`                    | `PUT/GET backup/{uuid}/file/internal`, `POST …/disk/internal` (upload-target for sakura), `GET backup_target/{host_uuid}/scan/internal`, `POST backup/register/internal`; all checked against the state of the package. |
| B7  | `feat(ryokan): add streamed copy of files between onsens`     | `POST backup/{uuid}/copy/internal`: download-stream from the source-onsen directly into the upload-stream of the target-onsen, without temp-files; into the bundle only `CREATING`, out of it only `READY`. Test with two onsens in the test-setup. |

### Block C – project-lock

| #   | Commit                                                      | Content                                                                                                                                               |
| --- | ----------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1  | `feat(miko): add lock of projects`                          | Columns `locked_by_job`, `lock_reason`, `locked_at` + migration; `POST/DELETE lock/internal`, `DELETE lock/admin`; `409` on double lock.             |
| C2  | `feat(miko): return lock-state in token-validation`         | `project_locked` in the response of `validate_token` and in `UserContext`. Only informative, nothing is blocked yet.                                  |
| C3  | `feat(ainari_api): reject writes to locked projects`        | Auth-middleware: non-`GET` requests of a locked project → `423`, internal API-key passes. Unit-tests of the middleware. From now on the lock is active in all services. |

### Block D – export/import of the components

| #   | Commit                                                           | Content                                                                                                                        |
| --- | ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| D1  | `feat(miko): add internal export and import of projects`         | Project + quota, `force` overwrites name/quota. No users/mappings.                                                             |
| D2  | `feat(omamori): add owner-type backup for secrets`               | `owned_by = "backup"`, not deletable by users, like the internal secrets of images.                                           |
| D3  | `feat(omamori): add internal export of project-secrets`          | Payloads wrapped with `wrap_key`, public-keys; test that no plain payload is in the response and that `wrap_key` isn't logged. |
| D4  | `feat(omamori): add internal import of project-secrets`          | Unwrap → re-encrypt with `simple_crypto`; `force`; `id_map`. Round-trip-test with D3.                                          |
| D5  | `feat(ryokan): add internal export and import of image-metadata` | Images and snapshots of the project (backups are not part of it), `force`, `id_map` for the references to secrets.             |
| D6  | `feat(hanami): add internal export of projects`                  | Networks, vm-types, meta-VMs, addresses, network-filters, floating-ips with the attached VM.                                   |
| D7  | `feat(hanami): add internal import of networks and vm-types`     | `force`, `id_map`.                                                                                                             |
| D8  | `feat(hanami): add internal import of filters and floating-ips`  | Filters with rewritten VM-references; floating-ips newly allocated from the pool and attached.                                 |
| D9  | `feat(hanami): allow fixed uuid, ip and mac on vm-reservation`   | Optional fields in the reserve-request, only accepted with the internal API-key; `409` if taken.                               |
| D10 | `feat(sakura): add internal export of vm-config`                 | `GET virtual_machine/{uuid}/export/internal` with the content for `sakura.json`.                                               |
| D11 | `feat(sakura): add snapshot of root-disk as backup-disk`         | `SnapshotKind` in the existing snapshot-task, `POST …/backup/internal` with upload into the bundle over B6, `stop_vm`-option. Image-specific calls move out of `save_ch_virtual_machine` into the caller. |

### Block E – isekai

| #   | Commit                                                    | Content                                                                                                                                                       |
| --- | --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| E1  | `feat(isekai): add skeleton of the new service`           | Crate, workspace-entry, `config.rs`, `main.rs`, http-server with version/ready-endpoints, `example_configs/ainari/isekai.toml`.                               |
| E2  | `build(isekai): add isekai to images and local setup`     | Targets in `Dockerfile_services` (debian + nix), `docker-compose.yml` with isekai and a second onsen as backup-target, build-scripts, OpenAPI-collector.        |
| E3  | `feat(miko): add isekai to the endpoint-list`             | `EndpointsResp.isekai`, config of miko.                                                                                                                       |
| E4  | `feat(isekai): add database and job-queue`                | `backup_table`, `job_table`, `id_map_table`, worker + queue like in sakura, active jobs → `ERROR` on start (cleanup only as hook, filled in later phases).      |
| E5  | `feat(isekai): add manifest of the backup-bundle`         | `core/keys.rs` (backup-labels over `ainari_common::kdf`), format, versioning, HMAC, sha256-check; pure logic with unit-tests (wrong passphrase, manipulated file, missing file, unknown version). |
| E6  | `feat(isekai): add backup of single virtual machines`     | Create/get/list/delete VM-backup with lock, backup-resource in ryokan, snapshot over D11, metadata, manifest, finalize, error-cleanup + unlock.               |
| E7  | `feat(isekai): add restore of single virtual machines`    | Restore into an existing project, with `force` over the existing `snapshot_restore_task`, without `force` as new VM from the snapshot.                       |
| E8  | `feat(isekai): add backup of projects`                    | All components, `include_images` with copy over B7, lock for the whole duration.                                                                            |
| E9  | `feat(isekai): add restore-plan with dry-run`             | Calculation of the `id_map` (remap/`force`), quota-check; only reading, endpoint with `dry_run=true`. Unit-tests of the reference-rewriting.               |
| E10 | `feat(isekai): add restore of projects`                   | Lock, apply in dependency-order, unlock; failed restore keeps the lock; re-run with `force` takes over the lock.                                             |
| E11 | `feat(isekai): add prune-option to the restore`           | Delete-list in the plan/dry-run, deletion in reverse order, `prune` only with `force`.                                                                     |
| E12 | `feat(isekai): add scan and import of foreign bundles`    | `GET backup/scan/admin`, `POST backup/import/admin`, registration of the bundle as backup-resource in ryokan.                                                 |

### Block F – tests, deployment, clients, docs

| #   | Commit                                                    | Content                                                                                          |
| --- | --------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| F1  | `test: add e2e-test of backup and restore`                | Scenarios 1–3, 5 and 6 of chapter 7 in `testing/local_stack`.                                    |
| F2  | `test: add e2e-test of project-lock and invalid bundles`  | Scenarios 4 and 8 of chapter 7.                                                                  |
| F3  | `test: add e2e-test of the disaster-recovery`             | Scenario 7 with a second stack.                                                                  |
| F4  | `feat(operator): deploy isekai and backup-onsen`          | CRD-extension `backupStorage`, deployment of isekai, zarf-package.                               |
| F5  | `feat(sdk): add backup-functions to the go-sdk`           | Go-SDK only.                                                                                     |
| F6  | `feat(cli): add backup-commands to ainarictl`             | Commands of chapter 6, passphrase only interactive/file/env.                                     |
| F7  | `feat(sdk): add backup-functions to the python-sdk`       | Python-SDK only.                                                                                 |
| F8  | `feat(dashboard): add backup and restore`                 | Backup-tab of project and VM, restore-dialog with dry-run-result.                                |
| F9  | `docs: add documentation of isekai`                       | Architecture-overview, user-docs, sequence-diagrams, `repo_structure.md`, nav in `zensical.toml`. |

### Block G – hardening (optional, later)

| #   | Commit                                                  | Content                                                       |
| --- | ------------------------------------------------------- | ------------------------------------------------------------- |
| G1  | `feat(isekai): add rollback of failed restores`         | Created resources of the job are deleted again, then unlock.  |
| G2  | `feat(isekai): add retention of backups`                | Keep the last n backups per project/VM.                       |
| G3  | `feat(isekai): add scheduled backups`                   | Cron-like schedule per project/VM.                            |

Dependencies: A and B1–B4 can be done in any order; B5 → B6 → B7; C1 → C2 → C3; D3 → D4;
D6 → D7 → D8; D11 needs B6; E1 → E2 → E3 → E4 → E5 and then E6 needs A, B6, C3, D2, D10, D11;
E7 needs D9 and B7; E8 needs D1–D8 and B7; E9 → E10 → E11; E12 needs E10.

## 9. Open points

- **Data-disks**: at the moment only the root-disk exists. When additional volumes are added,
  they must be part of `disks/`.
