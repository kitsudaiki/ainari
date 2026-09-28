# Miko

Miko stores its data either in a SQLite-database, whose file is configured with `sqlite.file_path`
(`/etc/ainari/miko_db` in the example config), or in a MySQL-database, which is configured with the
`mysql`-group. Which one is used is selected by `database_type`. The tables are created and
updated by the migrations in `src/binaries/miko/migrations/sqlite` and
`src/binaries/miko/migrations/mysql`, which are applied automatically at the start. The types
below are the ones of the SQLite-database; the MySQL-database uses `INT` for `INTEGER` and
`LONGTEXT` for `TEXT`.

## users

Users with their passphrase-hash and the projects they belong to.

| field      | type         | is primary | constraints |
| ---------- | ------------ | ---------- | ----------- |
| id         | VARCHAR(256) | x          |             |
| name       | VARCHAR(256) |            |             |
| is_admin   | VARCHAR(8)   |            |             |
| pw_hash    | VARCHAR(64)  |            |             |
| salt       | VARCHAR(64)  |            |             |
| projects   | TEXT         |            |             |
| status     | VARCHAR(8)   |            |             |
| created_at | VARCHAR(64)  |            |             |
| created_by | VARCHAR(256) |            |             |
| updated_at | VARCHAR(64)  |            |             |
| updated_by | VARCHAR(256) |            |             |
| deleted_at | VARCHAR(64)  |            |             |
| deleted_by | VARCHAR(256) |            |             |

!!! note

    `id` is only the primary key for the queries, the SQL-table itself has no primary key constraint.

## projects

Projects, which group the resources of users.

| field      | type         | is primary | constraints |
| ---------- | ------------ | ---------- | ----------- |
| id         | VARCHAR(256) | x          |             |
| name       | VARCHAR(256) |            |             |
| status     | VARCHAR(8)   |            |             |
| created_at | VARCHAR(64)  |            |             |
| created_by | VARCHAR(256) |            |             |
| updated_at | VARCHAR(64)  |            |             |
| updated_by | VARCHAR(256) |            |             |
| deleted_at | VARCHAR(64)  |            |             |
| deleted_by | VARCHAR(256) |            |             |

!!! note

    `id` is only the primary key for the queries, the SQL-table itself has no primary key constraint.

## quotas

Maximum number of resources per project. `id` is the ID of the project.

| field               | type         | is primary | constraints |
| ------------------- | ------------ | ---------- | ----------- |
| id                  | VARCHAR(256) | x          |             |
| max_virtual_machine | INTEGER      |            |             |
| max_image           | INTEGER      |            |             |
| max_secret          | INTEGER      |            |             |
| max_network         | INTEGER      |            |             |
| max_floating_ip     | INTEGER      |            |             |
| status              | VARCHAR(8)   |            |             |
| created_at          | VARCHAR(64)  |            |             |
| created_by          | VARCHAR(256) |            |             |
| updated_at          | VARCHAR(64)  |            |             |
| updated_by          | VARCHAR(256) |            |             |
| deleted_at          | VARCHAR(64)  |            |             |
| deleted_by          | VARCHAR(64)  |            |             |

!!! note

    `id` is only the primary key for the queries, the SQL-table itself has no primary key constraint.
