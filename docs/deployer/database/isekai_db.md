# Isekai

Isekai stores its data either in a SQLite-database, whose file is configured with `sqlite.file_path`
(`/etc/ainari/isekai_db` in the example config), or in a MySQL-database, which is configured with the
`mysql`-group. Which one is used is selected by `database_type`. The tables are created and
updated by the migrations in `src/binaries/isekai/migrations/sqlite` and
`src/binaries/isekai/migrations/mysql`, which are applied automatically at the start. The types
below are the ones of the SQLite-database; the MySQL-database uses `LONGTEXT` for `TEXT`.

Isekai doesn't store the files of the backups itself. They are stored as one package on a
backup-target, which is tracked by the backup-resource with the same UUID in ryokan (see
[ryokan](ryokan_db.md)).

## backups

Backups, which were created by isekai.

| field                | type         | is primary | constraints |
| -------------------- | ------------ | ---------- | ----------- |
| uuid                 | VARCHAR(40)  | x          |             |
| name                 | VARCHAR(256) |            | `NOT NULL`  |
| backup_kind          | VARCHAR(32)  |            | `NOT NULL`  |
| project_id           | VARCHAR(256) |            | `NOT NULL`  |
| virtual_machine_uuid | VARCHAR(40)  |            |             |
| backup_state         | VARCHAR(16)  |            | `NOT NULL`  |
| ainari_version       | VARCHAR(64)  |            | `NOT NULL`  |
| secret_uuids         | TEXT         |            | `NOT NULL`  |
| owner_id             | VARCHAR(256) |            |             |
| status               | VARCHAR(8)   |            |             |
| created_at           | VARCHAR(64)  |            |             |
| created_by           | VARCHAR(256) |            |             |
| updated_at           | VARCHAR(64)  |            |             |
| updated_by           | VARCHAR(256) |            |             |
| deleted_at           | VARCHAR(64)  |            |             |
| deleted_by           | VARCHAR(256) |            |             |

- `backup_kind`: `PROJECT` or `VIRTUAL_MACHINE`, which also sets `virtual_machine_uuid`.
- `backup_state`: `CREATING`, `READY`, `DELETING` or `ERROR`.
- `secret_uuids`: json-list of the secrets in omamori, which encrypt the disks of the backup. They
  are only needed, while the backup is created, and deleted afterwards. Secrets, which couldn't be
  deleted, stay in the list and are deleted together with the backup.

## jobs

Backup- and restore-jobs, which are processed in the background. The passphrases of the backups
are never stored, so a job, which was queued or active during a restart of isekai, is marked as
`ERROR` at the next start.

| field       | type         | is primary | constraints |
| ----------- | ------------ | ---------- | ----------- |
| uuid        | VARCHAR(40)  | x          |             |
| job_type    | VARCHAR(16)  |            | `NOT NULL`  |
| backup_uuid | VARCHAR(40)  |            | `NOT NULL`  |
| project_id  | VARCHAR(256) |            | `NOT NULL`  |
| job_state   | VARCHAR(16)  |            | `NOT NULL`  |
| step        | VARCHAR(256) |            | `NOT NULL`  |
| messages    | TEXT         |            | `NOT NULL`  |
| options     | TEXT         |            | `NOT NULL`  |
| owner_id    | VARCHAR(256) |            | `NOT NULL`  |
| created_at  | VARCHAR(64)  |            | `NOT NULL`  |
| created_by  | VARCHAR(256) |            | `NOT NULL`  |
| started_at  | VARCHAR(64)  |            |             |
| finished_at | VARCHAR(64)  |            |             |

- `job_type`: `BACKUP` or `RESTORE`.
- `job_state`: `QUEUED`, `ACTIVE`, `FINISHED` or `ERROR`.
- `messages`: json-list of the messages of the job, like the reason of a failure.
- `options`: json-object with the options of the job, without the passphrase.

## id_maps

Resources, which were restored by a job under another UUID, because their original UUID was
already used.

| field         | type        | is primary | constraints |
| ------------- | ----------- | ---------- | ----------- |
| job_uuid      | VARCHAR(40) | x          | `NOT NULL`  |
| resource_type | VARCHAR(32) |            | `NOT NULL`  |
| old_uuid      | VARCHAR(40) | x          | `NOT NULL`  |
| new_uuid      | VARCHAR(40) |            | `NOT NULL`  |
