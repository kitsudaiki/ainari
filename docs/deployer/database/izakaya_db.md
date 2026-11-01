# Izakaya

Izakaya stores its data either in a SQLite-database, whose file is configured with `sqlite.file_path`
(`/etc/ainari/izakaya_db` in the example config), or in a MySQL-database, which is configured with the
`mysql`-group. Which one is used is selected by `database_type`. The tables are created and
updated by the migrations in `src/binaries/izakaya/migrations/sqlite` and
`src/binaries/izakaya/migrations/mysql`, which are applied automatically at the start. The types
below are the ones of the SQLite-database; the MySQL-database uses `LONGTEXT` for `TEXT`.

## key_packages

MLS key-packages of the MLS-clients. Each key-package can be claimed exactly once to invite its
client into a group, so it is deleted with the claim.

| field       | type         | is primary | constraints |
| ----------- | ------------ | ---------- | ----------- |
| uuid        | VARCHAR(40)  | x          |             |
| client_id   | VARCHAR(256) |            |             |
| key_package | TEXT         |            |             |
| owner_id    | VARCHAR(256) |            |             |
| project_id  | VARCHAR(256) |            |             |
| created_at  | VARCHAR(64)  |            |             |
| created_by  | VARCHAR(256) |            |             |
| updated_at  | VARCHAR(64)  |            |             |
| updated_by  | VARCHAR(256) |            |             |

Indexes:

- `key_packages_client_id`: `(client_id)`

## mls_messages

MLS-messages, which wait for their recipient. A message with more than one recipient is stored
once per recipient, so each of them can acknowledge its copy on its own. An acknowledged message is
removed.

- `message_type`: `welcome`, `commit`, `operation` or `round`.
- `grants`: membership-grants, which are delivered together with the message.

| field        | type         | is primary | constraints          |
| ------------ | ------------ | ---------- | -------------------- |
| uuid         | VARCHAR(40)  | x          |                      |
| group_id     | VARCHAR(256) |            |                      |
| epoch        | BIGINT       |            | `NOT NULL DEFAULT 0` |
| message_type | VARCHAR(16)  |            |                      |
| sender       | VARCHAR(256) |            |                      |
| recipient    | VARCHAR(256) |            |                      |
| payload      | TEXT         |            |                      |
| owner_id     | VARCHAR(256) |            |                      |
| project_id   | VARCHAR(256) |            |                      |
| created_at   | VARCHAR(64)  |            |                      |
| created_by   | VARCHAR(256) |            |                      |
| updated_at   | VARCHAR(64)  |            |                      |
| updated_by   | VARCHAR(256) |            |                      |
| grants       | TEXT         |            |                      |

Indexes:

- `mls_messages_recipient`: `(recipient)`

## mls_grants

Membership-grants of hanami, which allow a gateway (`client_id`) to be member of the group of a
network (`vni`). A gateway has at most one grant per network, a new grant replaces the old one.

| field         | type         | is primary | constraints |
| ------------- | ------------ | ---------- | ----------- |
| uuid          | VARCHAR(40)  | x          |             |
| vni           | INTEGER      |            | `NOT NULL`  |
| client_id     | VARCHAR(256) |            | `NOT NULL`  |
| signature_key | VARCHAR(256) |            | `NOT NULL`  |
| payload       | TEXT         |            | `NOT NULL`  |
| signature     | VARCHAR(256) |            | `NOT NULL`  |
| expires_at    | BIGINT       |            | `NOT NULL`  |
| created_at    | VARCHAR(64)  |            | `NOT NULL`  |

Unique indexes:

- `mls_grants_vni_client`: `(vni, client_id)`

## mls_groups

State of the coordination of the group of every network: its members, its committer and its
key-rotation. `members` and `round_acks` are json-lists of identities, the timestamps are
unix-times in seconds. The `round_*`-columns are only set, while a key-rotation is running, and
`round_phase` is then `install`, `switch` or `grace`.

| field            | type         | is primary | constraints |
| ---------------- | ------------ | ---------- | ----------- |
| vni              | INTEGER      | x          | `NOT NULL`  |
| committer        | VARCHAR(256) |            | `NOT NULL`  |
| epoch            | BIGINT       |            | `NOT NULL`  |
| members          | TEXT         |            | `NOT NULL`  |
| round_epoch      | BIGINT       |            |             |
| round_phase      | VARCHAR(16)  |            |             |
| round_acks       | TEXT         |            |             |
| round_updated_at | BIGINT       |            |             |
| rotated_at       | BIGINT       |            | `NOT NULL`  |
| created_at       | BIGINT       |            | `NOT NULL`  |

## mls_operations

Queue of the changes of the groups, which their committers make one after another. The timestamps
are unix-times in milliseconds, so the operations of one second keep their order.

- `kind`: `add`, `remove` or `update`.
- `status`: `QUEUED`, while the operation waits for its turn, `INFLIGHT`, after it was handed to
  the committer, and `DONE`, when it is finished, successful or not.

| field      | type         | is primary | constraints          |
| ---------- | ------------ | ---------- | -------------------- |
| uuid       | VARCHAR(40)  | x          |                      |
| vni        | INTEGER      |            | `NOT NULL`           |
| kind       | VARCHAR(16)  |            | `NOT NULL`           |
| client_id  | VARCHAR(256) |            |                      |
| grant_json | TEXT         |            |                      |
| status     | VARCHAR(16)  |            | `NOT NULL`           |
| retries    | INTEGER      |            | `NOT NULL DEFAULT 0` |
| created_at | BIGINT       |            | `NOT NULL`           |
| started_at | BIGINT       |            |                      |

Indexes:

- `mls_operations_vni_status`: `(vni, status)`

## mls_clients

Last contact of every gateway as unix-time in seconds, which tells izakaya, if the gateway is still
alive.

| field     | type         | is primary | constraints |
| --------- | ------------ | ---------- | ----------- |
| client_id | VARCHAR(256) | x          | `NOT NULL`  |
| last_seen | BIGINT       |            | `NOT NULL`  |
