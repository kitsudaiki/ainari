# Miko

Miko stores its data either in a SQLite-database, whose file is configured with `sqlite.file_path`
(`/etc/ainari/miko_db` in the example config), or in a MySQL-database, which is configured with the
`mysql`-group. Which one is used is selected by `database_type`. The tables are created and
updated by the migrations in `src/binaries/miko/migrations/sqlite` and
`src/binaries/miko/migrations/mysql`, which are applied automatically at the start. The types
below are the ones of the SQLite-database; the MySQL-database uses `INT` for `INTEGER` and
`LONGTEXT` for `TEXT`.

## users

Users with their passphrase-hash. The projects they belong to are stored in
[user_project_mapping](#user_project_mapping).

| field      | type         | is primary | constraints |
| ---------- | ------------ | ---------- | ----------- |
| id         | VARCHAR(256) | x          |             |
| name       | VARCHAR(256) |            |             |
| is_admin   | VARCHAR(8)   |            |             |
| pw_hash    | VARCHAR(64)  |            |             |
| salt       | VARCHAR(64)  |            |             |
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

When a user is created, a project with the ID `default-<USER_ID>` is created together with it. The
prefix `default-` is reserved for these default-projects, so projects with such an ID can not be
created over the API.

`locked_by_job`, `lock_reason` and `locked_at` are the lock of the project during a backup or
restore. The project is locked, while `locked_by_job` is set, and it is unlocked, when all three are
NULL. `lock_reason` is a free text, which describes, why the project is locked, like the running
backup or restore, or the description, which an admin has given for a manual lock. While a project
is locked, its users can only read its resources, and only the job in
`locked_by_job` or an admin can remove the lock. A lock of an admin has the nil-UUID in
`locked_by_job`, which the jobs are not allowed to use, so only an admin can remove it. A locked project
can not be deleted. In the SQLite-database the columns are at the end of the table, because SQLite
can only append new columns.

| field         | type         | is primary | constraints |
| ------------- | ------------ | ---------- | ----------- |
| id            | VARCHAR(256) | x          |             |
| name          | VARCHAR(256) |            |             |
| status        | VARCHAR(8)   |            |             |
| locked_by_job | VARCHAR(40)  |            |             |
| lock_reason   | TEXT         |            |             |
| locked_at     | VARCHAR(64)  |            |             |
| created_at    | VARCHAR(64)  |            |             |
| created_by    | VARCHAR(256) |            |             |
| updated_at    | VARCHAR(64)  |            |             |
| updated_by    | VARCHAR(256) |            |             |
| deleted_at    | VARCHAR(64)  |            |             |
| deleted_by    | VARCHAR(256) |            |             |

!!! note

    `id` is only the primary key for the queries, the SQL-table itself has no primary key constraint.

## user_project_mapping

Assignment of users to projects with the role of the user within the project. A user can be
assigned to multiple projects and a project can have multiple users. When a user is created, it is
assigned to its default-project with the role `admin`.

`role` is one of `admin`, `member` or `observer`. Any other value within the column is rejected,
when the row is read.

| field      | type         | is primary | constraints |
| ---------- | ------------ | ---------- | ----------- |
| project_id | VARCHAR(256) | x          |             |
| user_id    | VARCHAR(256) | x          |             |
| role       | VARCHAR(64)  |            |             |
| status     | VARCHAR(8)   |            |             |
| created_at | VARCHAR(64)  |            |             |
| created_by | VARCHAR(256) |            |             |
| updated_at | VARCHAR(64)  |            |             |
| updated_by | VARCHAR(256) |            |             |
| deleted_at | VARCHAR(64)  |            |             |
| deleted_by | VARCHAR(256) |            |             |

Unique indexes:

- `user_project_mapping_active_project_user`: `(project_id, user_id)` where `status = 'ACTIVE'`

MySQL has no partial indexes, so there the unique index is built on the generated column
`active_project_id`, which contains the value of `project_id`, if `status = 'ACTIVE'`, and NULL
otherwise.

!!! note

    `project_id` and `user_id` are only the primary key for the queries, the SQL-table itself has no
    primary key constraint. Each of the two values alone can exist multiple times.

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
