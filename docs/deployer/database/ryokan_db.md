# Ryokan

Ryokan stores its data either in a SQLite-database, whose file is configured with `sqlite.file_path`
(`/etc/ainari/ryokan_db` in the example config), or in a MySQL-database, which is configured with the
`mysql`-group. Which one is used is selected by `database_type`. The tables are created and
updated by the migrations in `src/binaries/ryokan/migrations/sqlite` and
`src/binaries/ryokan/migrations/mysql`, which are applied automatically at the start. The types
below are the ones of the SQLite-database; the MySQL-database uses `INT` for `INTEGER` and
`LONGTEXT` for `TEXT`.

## hosts

Onsen-hosts, which registered themselves at ryokan and store the image-files.

`is_backup_target` marks a host, which an admin reserved for backups. Such a host never stores
normal images or snapshots: it is skipped by the host-selection for new images and adding an image
on it is rejected. A host can only be marked, while no image is stored on it, and only be unmarked,
while no backup is registered on it in `backups` and its directory `backups/` is empty. In the
SQLite-database the column is at the end of the table, because SQLite can only append new columns.

| field            | type         | is primary | constraints                    |
| ---------------- | ------------ | ---------- | ------------------------------ |
| uuid             | VARCHAR(40)  | x          |                                |
| name             | VARCHAR(256) |            |                                |
| address          | VARCHAR(256) |            |                                |
| status           | VARCHAR(8)   |            |                                |
| is_backup_target | BOOLEAN      |            | `NOT NULL DEFAULT FALSE`       |
| created_at       | VARCHAR(64)  |            |                                |
| created_by       | VARCHAR(256) |            |                                |
| updated_at       | VARCHAR(64)  |            |                                |
| updated_by       | VARCHAR(256) |            |                                |
| deleted_at       | VARCHAR(64)  |            |                                |
| deleted_by       | VARCHAR(256) |            |                                |

## images

Images of virtual machines and the onsen-host, where the file is stored. Snapshots of virtual
machines are images with `is_snapshot` set.

`file_type` and `file_size` are measured on the unencrypted file, before it is encrypted. The type
is identified by the magic-bytes of the file and named like the formats of `qemu-img` (`qcow2`,
`qcow`, `qed`, `vhdx`, `vmdk`, `vdi`, `vpc`, `iso`), a file without known format is `raw`. A
snapshot is registered before its file exists, so it keeps the defaults `unknown` and `0`, until
sakura has copied the root-disk. In the SQLite-database both columns are at the end of the table,
because SQLite can only append new columns.

`image_status` is the state of the file of the image. Only a `READY` image can be used for a
virtual machine. Existing images got `READY` with the migration, because they are complete.

| image_status | meaning                                                                                   |
| ------------ | ----------------------------------------------------------------------------------------- |
| `UPLOADING`  | the encrypted file is stored in the onsen, by ryokan or by sakura for a snapshot          |
| `ENCRYPTING` | ryokan encrypts the uploaded file and stores it, or sakura copies and encrypts a snapshot |
| `READY`      | the image is complete and can be used                                                     |
| `IN_USE`     | reserved, not set at the moment                                                           |

An uploaded image is only registered, after its file was completely uploaded to ryokan. Like a
snapshot, which is created by sakura, it goes through `ENCRYPTING` → `UPLOADING` → `READY`. If the
encryption or the storing of an uploaded image fails, the image is deleted again together with its
secret.

| field         | type         | is primary | constraints                    |
| ------------- | ------------ | ---------- | ------------------------------ |
| uuid          | VARCHAR(40)  | x          |                                |
| name          | VARCHAR(256) |            |                                |
| onsen_address | VARCHAR(256) |            |                                |
| file_path     | TEXT         |            |                                |
| file_type     | VARCHAR(32)  |            | `NOT NULL DEFAULT 'unknown'`   |
| file_size     | BIGINT       |            | `NOT NULL DEFAULT 0`           |
| image_status  | VARCHAR(16)  |            | `NOT NULL DEFAULT 'READY'`     |
| secret_uuid   | VARCHAR(40)  |            |                                |
| is_snapshot   | BOOLEAN      |            | `NOT NULL DEFAULT FALSE`       |
| owner_id      | VARCHAR(256) |            |                                |
| project_id    | VARCHAR(256) |            |                                |
| status        | VARCHAR(8)   |            |                                |
| created_at    | VARCHAR(64)  |            |                                |
| created_by    | VARCHAR(256) |            |                                |
| updated_at    | VARCHAR(64)  |            |                                |
| updated_by    | VARCHAR(256) |            |                                |
| deleted_at    | VARCHAR(64)  |            |                                |
| deleted_by    | VARCHAR(256) |            |                                |

## backups

Backups, whose package is stored as one directory `backups/<uuid>` on a backup-target
(`host_uuid`). A package consists of multiple files (manifest, metadata, disks and images), but
ryokan only tracks the package as a whole and never its single files, so it stays consistent.
Name, kind and the other metadata of a backup are stored by isekai, which uses the same UUID.

`backup_status` is the state of the package. It defines, which operations are allowed on its files.
Every change of the state is a conditional update, so of two concurrent changes, like sealing and
deleting the same package, only one succeeds.

| backup_status | meaning                                                                                     |
| ------------- | ------------------------------------------------------------------------------------------- |
| `CREATING`    | the backup is created, files can only be written into the package in this state             |
| `READY`       | the package is complete and sealed, so it is read only. Its files can be read and copied.   |
| `DELETING`    | the package is deleted as a whole. A failed deletion stays in this state and is repeatable. |
| `ERROR`       | the package is incomplete, for example because it has no manifest, and can only be deleted |

`file_count`, `total_size` and `manifest_sha256` are read from the backup-target, when the package
is sealed, so a later change of the files can be detected. A package, which was copied onto a
backup-target from outside, is registered directly as `READY` with these values. Also a deleted
backup keeps its UUID, because the UUID names the directory of the package.

| field           | type         | is primary | constraints          |
| --------------- | ------------ | ---------- | -------------------- |
| uuid            | VARCHAR(40)  | x          |                      |
| host_uuid       | VARCHAR(40)  |            | `NOT NULL`           |
| directory_path  | TEXT         |            | `NOT NULL`           |
| backup_status   | VARCHAR(16)  |            | `NOT NULL`           |
| file_count      | INTEGER      |            | `NOT NULL DEFAULT 0` |
| total_size      | BIGINT       |            | `NOT NULL DEFAULT 0` |
| manifest_sha256 | VARCHAR(64)  |            |                      |
| owner_id        | VARCHAR(256) |            |                      |
| project_id      | VARCHAR(256) |            |                      |
| status          | VARCHAR(8)   |            |                      |
| created_at      | VARCHAR(64)  |            |                      |
| created_by      | VARCHAR(256) |            |                      |
| updated_at      | VARCHAR(64)  |            |                      |
| updated_by      | VARCHAR(256) |            |                      |
| deleted_at      | VARCHAR(64)  |            |                      |
| deleted_by      | VARCHAR(256) |            |                      |
