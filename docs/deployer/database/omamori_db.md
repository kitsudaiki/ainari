# Omamori

Omamori stores its data in a SQLite-database, whose file is configured with `database.file_path`
(`/etc/ainari/omamori_db` in the example config). The tables are created at the first start, missing
columns are added automatically at the start after an update.

## secrets

Metadata of secrets. The secret itself is stored encrypted in `simple_crypto`.

| field      | type         | is primary | constraints |
| ---------- | ------------ | ---------- | ----------- |
| uuid       | VARCHAR(40)  | x          |             |
| name       | VARCHAR(256) |            |             |
| owner_id   | VARCHAR(256) |            |             |
| project_id | VARCHAR(256) |            |             |
| status     | VARCHAR(8)   |            |             |
| created_at | VARCHAR(64)  |            |             |
| created_by | VARCHAR(256) |            |             |
| updated_at | VARCHAR(64)  |            |             |
| updated_by | VARCHAR(256) |            |             |
| deleted_at | VARCHAR(64)  |            |             |
| deleted_by | VARCHAR(256) |            |             |

## simple_crypto

Encrypted content of the secrets, encrypted with `simple_crypto.key_b64` of the config.

| field            | type        | is primary | constraints |
| ---------------- | ----------- | ---------- | ----------- |
| secret_uuid      | VARCHAR(40) | x          |             |
| encrypted_secret | TEXT        |            |             |

## public_keys

Public SSH-keys, which are injected into virtual machines.

| field       | type         | is primary | constraints |
| ----------- | ------------ | ---------- | ----------- |
| uuid        | VARCHAR(40)  | x          |             |
| name        | VARCHAR(256) |            |             |
| public_key  | TEXT         |            |             |
| fingerprint | VARCHAR(256) |            |             |
| owner_id    | VARCHAR(256) |            |             |
| project_id  | VARCHAR(256) |            |             |
| status      | VARCHAR(8)   |            |             |
| created_at  | VARCHAR(64)  |            |             |
| created_by  | VARCHAR(256) |            |             |
| updated_at  | VARCHAR(64)  |            |             |
| updated_by  | VARCHAR(256) |            |             |
| deleted_at  | VARCHAR(64)  |            |             |
| deleted_by  | VARCHAR(256) |            |             |
