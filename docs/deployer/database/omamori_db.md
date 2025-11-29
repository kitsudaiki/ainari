# Omamori

## secrets

| field      | type         | is primary |
| ---------- | ------------ | ---------- |
| uuid       | VARCHAR(40)  | x          |
| name       | VARCHAR(256) |            |
| owner_id   | VARCHAR(256) |            |
| project_id | VARCHAR(256) |            |
| status     | VARCHAR(8)   |            |
| created_at | VARCHAR(64)  |            |
| created_by | VARCHAR(256) |            |
| updated_at | VARCHAR(64)  |            |
| updated_by | VARCHAR(256) |            |
| deleted_at | VARCHAR(64)  |            |
| deleted_by | VARCHAR(256) |            |

## simple_crypto

| field            | type        | is primary |
| ---------------- | ----------- | ---------- |
| secret_uuid      | VARCHAR(40) | x          |
| encrypted_secret | TEXT        |            |
