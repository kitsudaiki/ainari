# Hanami

Hanami stores its data in a SQLite-database, whose file is configured with `database.file_path`
(`/etc/ainari/hanami_db` in the example config). The tables are created at the first start, missing
columns are added automatically at the start after an update.

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
