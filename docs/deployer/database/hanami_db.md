# Hanami

Hanami stores its data either in a SQLite-database, whose file is configured with `sqlite.file_path`
(`/etc/ainari/hanami_db` in the example config), or in a MySQL-database, which is configured with the
`mysql`-group. Which one is used is selected by `database_type`. The tables are created and
updated by the migrations in `src/binaries/hanami/migrations/sqlite` and
`src/binaries/hanami/migrations/mysql`, which are applied automatically at the start. The types
below are the ones of the SQLite-database; the MySQL-database uses `INT` for `INTEGER` and
`LONGTEXT` for `TEXT`.

## hosts

Sakura-hosts, which registered themselves at hanami, together with their resources and how much of
them is already used by virtual machines.

| field                     | type         | is primary | constraints                                                 |
| ------------------------- | ------------ | ---------- | ----------------------------------------------------------- |
| uuid                      | VARCHAR(40)  | x          |                                                             |
| name                      | VARCHAR(256) |            |                                                             |
| address                   | VARCHAR(256) |            |                                                             |
| number_of_cores           | BIGINT       |            | `NOT NULL DEFAULT 0 CHECK (number_of_cores >= 0)`           |
| used_number_of_cores      | BIGINT       |            | `NOT NULL DEFAULT 0 CHECK (used_number_of_cores >= 0)`      |
| memory_size               | BIGINT       |            | `NOT NULL DEFAULT 0 CHECK (memory_size >= 0)`               |
| amount_of_used_memory     | BIGINT       |            | `NOT NULL DEFAULT 0 CHECK (amount_of_used_memory >= 0)`     |
| disk_space                | BIGINT       |            | `NOT NULL DEFAULT 0 CHECK (disk_space >= 0)`                |
| amount_of_used_disk_space | BIGINT       |            | `NOT NULL DEFAULT 0 CHECK (amount_of_used_disk_space >= 0)` |
| status                    | VARCHAR(8)   |            |                                                             |
| created_at                | VARCHAR(64)  |            |                                                             |
| created_by                | VARCHAR(256) |            |                                                             |
| updated_at                | VARCHAR(64)  |            |                                                             |
| updated_by                | VARCHAR(256) |            |                                                             |
| deleted_at                | VARCHAR(64)  |            |                                                             |
| deleted_by                | VARCHAR(256) |            |                                                             |
| external_address          | VARCHAR(256) |            |                                                             |

`external_address` is the address of the external api of the sakura-host, which the proxies of
its virtual machines forward to. It is NULL for hosts without one, then `address` is used.

## meta_virtual_machines

Virtual machines with the sakura-host they were scheduled on and the resources they reserve there.

| field            | type         | is primary | constraints                                       |
| ---------------- | ------------ | ---------- | ------------------------------------------------- |
| uuid             | VARCHAR(40)  | x          |                                                   |
| name             | VARCHAR(256) |            |                                                   |
| sakura_host_uuid | VARCHAR(40)  |            |                                                   |
| proxy_uuid       | VARCHAR(40)  |            |                                                   |
| owner_id         | VARCHAR(256) |            |                                                   |
| project_id       | VARCHAR(256) |            |                                                   |
| status           | VARCHAR(8)   |            |                                                   |
| created_at       | VARCHAR(64)  |            |                                                   |
| created_by       | VARCHAR(256) |            |                                                   |
| updated_at       | VARCHAR(64)  |            |                                                   |
| updated_by       | VARCHAR(256) |            |                                                   |
| deleted_at       | VARCHAR(64)  |            |                                                   |
| deleted_by       | VARCHAR(256) |            |                                                   |
| number_of_cores  | BIGINT       |            | `NOT NULL DEFAULT 0 CHECK (number_of_cores >= 0)` |
| memory_size      | BIGINT       |            | `NOT NULL DEFAULT 0 CHECK (memory_size >= 0)`     |
| disk_size        | BIGINT       |            | `NOT NULL DEFAULT 0 CHECK (disk_size >= 0)`       |

## networks

Virtual networks with their subnet.

| field      | type         | is primary | constraints |
| ---------- | ------------ | ---------- | ----------- |
| uuid       | VARCHAR(40)  | x          |             |
| name       | VARCHAR(256) |            |             |
| subnet     | VARCHAR(40)  |            |             |
| owner_id   | VARCHAR(256) |            |             |
| project_id | VARCHAR(256) |            |             |
| status     | VARCHAR(8)   |            |             |
| created_at | VARCHAR(64)  |            |             |
| created_by | VARCHAR(256) |            |             |
| updated_at | VARCHAR(64)  |            |             |
| updated_by | VARCHAR(256) |            |             |
| deleted_at | VARCHAR(64)  |            |             |
| deleted_by | VARCHAR(256) |            |             |

## addresses

Addresses reserved in a network for the port of a virtual machine: internal IP, MAC-address,
TAP-device and the tenant (`vni`) of the network.

| field                | type         | is primary | constraints          |
| -------------------- | ------------ | ---------- | -------------------- |
| uuid                 | VARCHAR(40)  | x          |                      |
| mac_address          | VARCHAR(40)  |            |                      |
| tap_name             | VARCHAR(16)  |            |                      |
| internal_ip          | VARCHAR(40)  |            |                      |
| network_uuid         | VARCHAR(40)  |            |                      |
| vni                  | INTEGER      |            | `NOT NULL DEFAULT 0` |
| host_address         | VARCHAR(256) |            |                      |
| virtual_machine_uuid | VARCHAR(40)  |            |                      |
| owner_id             | VARCHAR(256) |            |                      |
| project_id           | VARCHAR(256) |            |                      |
| status               | VARCHAR(8)   |            |                      |
| created_at           | VARCHAR(64)  |            |                      |
| created_by           | VARCHAR(256) |            |                      |
| updated_at           | VARCHAR(64)  |            |                      |
| updated_by           | VARCHAR(256) |            |                      |
| deleted_at           | VARCHAR(64)  |            |                      |
| deleted_by           | VARCHAR(256) |            |                      |

Unique indexes:

- `addresses_active_mac_address`: `(mac_address)` where `status = 'ACTIVE'`
- `addresses_active_tap_name`: `(tap_name)` where `status = 'ACTIVE'`
- `addresses_active_network_internal_ip`: `(network_uuid, internal_ip)` where `status = 'ACTIVE'`

MySQL has no partial indexes, so there the unique indexes are built on generated columns
`active_<column>`, which contain the value of the column, if `status = 'ACTIVE'`, and NULL
otherwise.

## floating_ips

Floating IPs and the internal address they are attached to. `network_uuid` and `internal_ip_addr`
are NULL, while the floating IP is detached.

| field            | type         | is primary | constraints           |
| ---------------- | ------------ | ---------- | --------------------- |
| uuid             | VARCHAR(40)  | x          |                       |
| name             | VARCHAR(256) |            | `NOT NULL DEFAULT ''` |
| network_uuid     | VARCHAR(40)  |            |                       |
| internal_ip_addr | VARCHAR(40)  |            |                       |
| floating_ip_addr | VARCHAR(40)  |            |                       |
| owner_id         | VARCHAR(256) |            |                       |
| project_id       | VARCHAR(256) |            |                       |
| status           | VARCHAR(8)   |            |                       |
| created_at       | VARCHAR(64)  |            |                       |
| created_by       | VARCHAR(256) |            |                       |
| updated_at       | VARCHAR(64)  |            |                       |
| updated_by       | VARCHAR(256) |            |                       |
| deleted_at       | VARCHAR(64)  |            |                       |
| deleted_by       | VARCHAR(256) |            |                       |

Unique indexes:

- `floating_ips_active_floating_ip_addr`: `(floating_ip_addr)` where `status = 'ACTIVE'`
- `floating_ips_active_network_internal_ip_addr`: `(network_uuid, internal_ip_addr)` where `status =
  'ACTIVE'`

MySQL has no partial indexes, so there the unique indexes are built on generated columns
`active_<column>`, which contain the value of the column, if `status = 'ACTIVE'`, and NULL
otherwise.
