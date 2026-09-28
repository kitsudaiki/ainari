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

| field      | type         | is primary | constraints |
| ---------- | ------------ | ---------- | ----------- |
| uuid       | VARCHAR(40)  | x          |             |
| name       | VARCHAR(256) |            |             |
| address    | VARCHAR(256) |            |             |
| status     | VARCHAR(8)   |            |             |
| created_at | VARCHAR(64)  |            |             |
| created_by | VARCHAR(256) |            |             |
| updated_at | VARCHAR(64)  |            |             |
| updated_by | VARCHAR(256) |            |             |
| deleted_at | VARCHAR(64)  |            |             |
| deleted_by | VARCHAR(256) |            |             |

## images

Images of virtual machines and the onsen-host, where the file is stored. Snapshots of virtual
machines are images with `is_snapshot` set.

| field         | type         | is primary | constraints              |
| ------------- | ------------ | ---------- | ------------------------ |
| uuid          | VARCHAR(40)  | x          |                          |
| name          | VARCHAR(256) |            |                          |
| onsen_address | VARCHAR(256) |            |                          |
| file_path     | TEXT         |            |                          |
| secret_uuid   | VARCHAR(40)  |            |                          |
| is_snapshot   | BOOLEAN      |            | `NOT NULL DEFAULT FALSE` |
| owner_id      | VARCHAR(256) |            |                          |
| project_id    | VARCHAR(256) |            |                          |
| status        | VARCHAR(8)   |            |                          |
| created_at    | VARCHAR(64)  |            |                          |
| created_by    | VARCHAR(256) |            |                          |
| updated_at    | VARCHAR(64)  |            |                          |
| updated_by    | VARCHAR(256) |            |                          |
| deleted_at    | VARCHAR(64)  |            |                          |
| deleted_by    | VARCHAR(256) |            |                          |
