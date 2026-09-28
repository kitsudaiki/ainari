# Sakura

Sakura stores its data in a SQLite-database, whose file is configured with `database.file_path`
(`/etc/ainari/sakura_db` in the example config). The tables are created and updated by the migrations in
`src/binaries/sakura/migrations`, which are applied automatically at the start.

## virtual_machines

Virtual machines running on this host, with their resources, their files and their network-port.

| field           | type          | is primary | constraints                   |
| --------------- | ------------- | ---------- | ----------------------------- |
| uuid            | VARCHAR(40)   | x          |                               |
| name            | VARCHAR(256)  |            |                               |
| vm_state        | VARCHAR(16)   |            | `NOT NULL DEFAULT 'RESERVED'` |
| number_of_cores | INTEGER       |            |                               |
| memory_size     | INTEGER       |            |                               |
| image_uuid      | VARCHAR(40)   |            |                               |
| public_key_uuid | VARCHAR(40)   |            |                               |
| network_uuid    | VARCHAR(40)   |            |                               |
| internal_ip     | VARCHAR(40)   |            |                               |
| root_disk_path  | VARCHAR(1024) |            |                               |
| seed_path       | VARCHAR(1024) |            |                               |
| tap_name        | VARCHAR(256)  |            |                               |
| mac_address     | VARCHAR(32)   |            |                               |
| owner_id        | VARCHAR(256)  |            |                               |
| project_id      | VARCHAR(256)  |            |                               |
| status          | VARCHAR(8)    |            |                               |
| created_at      | VARCHAR(64)   |            |                               |
| created_by      | VARCHAR(256)  |            |                               |
| updated_at      | VARCHAR(64)   |            |                               |
| updated_by      | VARCHAR(256)  |            |                               |
| deleted_at      | VARCHAR(64)   |            |                               |
| deleted_by      | VARCHAR(256)  |            |                               |
| disk_size       | BIGINT        |            | `NOT NULL DEFAULT 5`          |

## tasks

Asynchronous tasks on resources of this host, for example the creation of a virtual machine, with
their state and messages.

| field         | type         | is primary | constraints |
| ------------- | ------------ | ---------- | ----------- |
| uuid          | VARCHAR(40)  | x          |             |
| description   | VARCHAR(256) |            |             |
| resource_uuid | VARCHAR(40)  |            |             |
| resource_type | VARCHAR(32)  |            |             |
| task_type     | VARCHAR(32)  |            |             |
| task_state    | VARCHAR(32)  |            |             |
| queued_at     | VARCHAR(64)  |            |             |
| started_at    | VARCHAR(64)  |            |             |
| aborted_at    | VARCHAR(64)  |            |             |
| finished_at   | VARCHAR(64)  |            |             |
| messages      | TEXT         |            |             |
| owner_id      | VARCHAR(256) |            |             |
| project_id    | VARCHAR(256) |            |             |
| created_by    | VARCHAR(256) |            |             |
