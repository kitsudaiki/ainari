# Ryokan

## checkpoints

| field         | type         | is primary |
| ------------- | ------------ | ---------- |
| uuid          | VARCHAR(40)  | x          |
| name          | VARCHAR(256) |            |
| onsen_address | VARCHAR(256) |            |
| file_path     | TEXT         |            |
| secret_uuid   | VARCHAR(40)  |            |
| owner_id      | VARCHAR(256) |            |
| project_id    | VARCHAR(256) |            |
| status        | VARCHAR(8)   |            |
| created_at    | VARCHAR(64)  |            |
| created_by    | VARCHAR(256) |            |
| updated_at    | VARCHAR(64)  |            |
| updated_by    | VARCHAR(256) |            |
| deleted_at    | VARCHAR(64)  |            |
| deleted_by    | VARCHAR(256) |            |

## datasets

| field             | type         | is primary |
| ----------------- | ------------ | ---------- |
| uuid              | VARCHAR(40)  | x          |
| name              | VARCHAR(256) |            |
| onsen_address     | VARCHAR(256) |            |
| file_path         | TEXT         |            |
| secret_uuid       | VARCHAR(40)  |            |
| number_of_rows    | BIGINT       |            |
| number_of_columns | BIGINT       |            |
| owner_id          | VARCHAR(256) |            |
| project_id        | VARCHAR(256) |            |
| status            | VARCHAR(8)   |            |
| created_at        | VARCHAR(64)  |            |
| created_by        | VARCHAR(256) |            |
| updated_at        | VARCHAR(64)  |            |
| updated_by        | VARCHAR(256) |            |
| deleted_at        | VARCHAR(64)  |            |
| deleted_by        | VARCHAR(256) |            |

## hosts

| field      | type         | is primary |
| ---------- | ------------ | ---------- |
| uuid       | VARCHAR(40)  | x          |
| name       | VARCHAR(256) |            |
| address    | VARCHAR(256) |            |
| status     | VARCHAR(8)   |            |
| created_at | VARCHAR(64)  |            |
| created_by | VARCHAR(256) |            |
| updated_at | VARCHAR(64)  |            |
| updated_by | VARCHAR(256) |            |
| deleted_at | VARCHAR(64)  |            |
| deleted_by | VARCHAR(256) |            |
