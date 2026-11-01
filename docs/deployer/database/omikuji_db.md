# Omikuji

Omikuji stores its data either in a SQLite-database, whose file is configured with `sqlite.file_path`
(`/etc/ainari/omikuji_db` in the example config), or in a MySQL-database, which is configured with the
`mysql`-group. Which one is used is selected by `database_type`. The tables are created and
updated by the migrations in `src/binaries/omikuji/migrations/sqlite` and
`src/binaries/omikuji/migrations/mysql`, which are applied automatically at the start. The types
below are the ones of the SQLite-database; the MySQL-database uses `LONGTEXT` for `TEXT`.

The entries of both tables are only added and never changed or deleted, so there are no update-
columns. `created_at` is an RFC-3339 timestamp in UTC, whose string-order is the chronological
order. `user_id` and `project_id` are empty, if the entry doesn't belong to a user or project.

## error_logs

Error-messages, which were reported by the components.

| field          | type         | is primary | constraints |
| -------------- | ------------ | ---------- | ----------- |
| uuid           | VARCHAR(40)  | x          |             |
| message        | TEXT         |            | `NOT NULL`  |
| component_name | VARCHAR(64)  |            | `NOT NULL`  |
| host_name      | VARCHAR(256) |            | `NOT NULL`  |
| user_id        | VARCHAR(256) |            | `NOT NULL`  |
| project_id     | VARCHAR(256) |            | `NOT NULL`  |
| created_at     | VARCHAR(64)  |            | `NOT NULL`  |

Indexes:

- `error_logs_created_at_idx`: `(created_at)`

## audit_logs

Requests, which were reported by the components for the audit, with their type, like `POST`, and
the called endpoint.

| field          | type          | is primary | constraints |
| -------------- | ------------- | ---------- | ----------- |
| uuid           | VARCHAR(40)   | x          |             |
| request_type   | VARCHAR(64)   |            | `NOT NULL`  |
| endpoint       | VARCHAR(1024) |            | `NOT NULL`  |
| component_name | VARCHAR(64)   |            | `NOT NULL`  |
| host_name      | VARCHAR(256)  |            | `NOT NULL`  |
| user_id        | VARCHAR(256)  |            | `NOT NULL`  |
| project_id     | VARCHAR(256)  |            | `NOT NULL`  |
| created_at     | VARCHAR(64)   |            | `NOT NULL`  |

Indexes:

- `audit_logs_created_at_idx`: `(created_at)`
