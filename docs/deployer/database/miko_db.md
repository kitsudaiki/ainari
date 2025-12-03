# Miko

## users

| field      | type         | is primary |
| ---------- | ------------ | ---------- |
| id         | VARCHAR(256) | x          |
| name       | VARCHAR(256) |            |
| projects   | TEXT         |            |
| is_admin   | VARCHAR(8)   |            |
| pw_hash    | VARCHAR(64)  |            |
| salt       | VARCHAR(64)  |            |
| status     | VARCHAR(8)   |            |
| created_at | VARCHAR(64)  |            |
| created_by | VARCHAR(256) |            |
| updated_at | VARCHAR(64)  |            |
| updated_by | VARCHAR(256) |            |
| deleted_at | VARCHAR(64)  |            |
| deleted_by | VARCHAR(256) |            |

## projects

| field      | type         | is primary |
| ---------- | ------------ | ---------- |
| id         | VARCHAR(256) | x          |
| name       | VARCHAR(256) |            |
| status     | VARCHAR(8)   |            |
| created_at | VARCHAR(64)  |            |
| created_by | VARCHAR(256) |            |
| updated_at | VARCHAR(64)  |            |
| updated_by | VARCHAR(256) |            |
| deleted_at | VARCHAR(64)  |            |
| deleted_by | VARCHAR(256) |            |

## quotas

| field          | type         | is primary |
| -------------- | ------------ | ---------- |
| id             | VARCHAR(256) | x          |
| max_cluster    | INTEGER      |            |
| max_dataset    | INTEGER      |            |
| max_checkpoint | INTEGER      |            |
| max_secret     | INTEGER      |            |
| max_taskqueue  | INTEGER      |            |
| status         | VARCHAR(8)   |            |
| created_at     | VARCHAR(64)  |            |
| created_by     | VARCHAR(256) |            |
| updated_at     | VARCHAR(64)  |            |
| updated_by     | VARCHAR(256) |            |
| deleted_at     | VARCHAR(64)  |            |
| deleted_by     | VARCHAR(256) |            |
