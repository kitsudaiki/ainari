# Torii

Torii stores its data in a SQLite-database, whose file is configured with `database.file_path`
(`/etc/ainari/torii_db` in the example config). The tables are created and updated by the migrations
in `src/binaries/torii/migrations`, which are applied automatically at the start.

## proxys

Proxies, which forward a port of the gateway to a virtual machine.

| field                | type         | is primary | constraints |
| -------------------- | ------------ | ---------- | ----------- |
| uuid                 | VARCHAR(40)  | x          |             |
| port                 | INTEGER      |            |             |
| target_address       | VARCHAR(256) |            |             |
| virtual_machine_uuid | VARCHAR(40)  |            |             |
| owner_id             | VARCHAR(256) |            |             |
| project_id           | VARCHAR(256) |            |             |
| status               | VARCHAR(8)   |            |             |
| created_at           | VARCHAR(64)  |            |             |
| created_by           | VARCHAR(256) |            |             |
| updated_at           | VARCHAR(64)  |            |             |
| updated_by           | VARCHAR(256) |            |             |
| deleted_at           | VARCHAR(64)  |            |             |
| deleted_by           | VARCHAR(256) |            |             |
