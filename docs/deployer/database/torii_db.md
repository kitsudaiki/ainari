# Torii

Torii stores its data in a SQLite-database, whose file is configured with `database.file_path`
(`/etc/ainari/torii_db` in the example config). The tables are created and updated by the migrations
in `src/binaries/torii/migrations`, which are applied automatically at the start.

Everything, which is configured over the endpoints of the gateway, is persisted, so it is
restored into the eBPF-datapath, when the torii starts again.

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

## network_interfaces

Configurations of existing interfaces of the gateway, which place them into a tenant. `ip_cidr`
is the address, which was added to the interface, `up` tells, if the interface was brought up,
and `fip_port` tells, if floating IPs are translated on the interface. An interface has only one
configuration at a time.

| field      | type         | is primary | constraints              |
| ---------- | ------------ | ---------- | ------------------------ |
| uuid       | VARCHAR(40)  | x          |                          |
| iface_name | VARCHAR(256) |            |                          |
| ip_cidr    | VARCHAR(64)  |            |                          |
| up         | BOOLEAN      |            | `NOT NULL DEFAULT FALSE` |
| vni        | INTEGER      |            | `NOT NULL DEFAULT 0`     |
| fip_port   | BOOLEAN      |            | `NOT NULL DEFAULT FALSE` |
| owner_id   | VARCHAR(256) |            |                          |
| project_id | VARCHAR(256) |            |                          |
| status     | VARCHAR(8)   |            |                          |
| created_at | VARCHAR(64)  |            |                          |
| created_by | VARCHAR(256) |            |                          |
| updated_at | VARCHAR(64)  |            |                          |
| updated_by | VARCHAR(256) |            |                          |
| deleted_at | VARCHAR(64)  |            |                          |
| deleted_by | VARCHAR(256) |            |                          |

## taps

TAP-devices of the virtual machines on the host of the gateway, together with the tenant (`vni`)
and the MAC- and IP-address of the virtual machine behind the device. A TAP-device is registered
only once at a time.

| field      | type         | is primary | constraints          |
| ---------- | ------------ | ---------- | -------------------- |
| uuid       | VARCHAR(40)  | x          |                      |
| tap_name   | VARCHAR(256) |            |                      |
| vni        | INTEGER      |            | `NOT NULL DEFAULT 0` |
| vm_mac     | VARCHAR(17)  |            |                      |
| vm_ip      | VARCHAR(16)  |            |                      |
| owner_id   | VARCHAR(256) |            |                      |
| project_id | VARCHAR(256) |            |                      |
| status     | VARCHAR(8)   |            |                      |
| created_at | VARCHAR(64)  |            |                      |
| created_by | VARCHAR(256) |            |                      |
| updated_at | VARCHAR(64)  |            |                      |
| updated_by | VARCHAR(256) |            |                      |
| deleted_at | VARCHAR(64)  |            |                      |
| deleted_by | VARCHAR(256) |            |                      |

## routes

Routes of the datapath. A route leads to an address (`dest_ip`) within a tenant (`vni`), either
locally onto an interface (`target_iface`), like the TAP-device of a virtual machine, or through
the overlay to the gateway of another host (`gateway_ip`). `next_hop_ip` and `next_hop_mac`
define the link layer next hop, and `encrypted` routes the traffic over IPsec. The routes, which
the gateway derives from its own config at startup, only get an entry with their first update.

| field        | type         | is primary | constraints              |
| ------------ | ------------ | ---------- | ------------------------ |
| uuid         | VARCHAR(40)  | x          |                          |
| vni          | INTEGER      |            | `NOT NULL DEFAULT 0`     |
| dest_ip      | VARCHAR(16)  |            |                          |
| target_iface | VARCHAR(256) |            |                          |
| gateway_ip   | VARCHAR(16)  |            |                          |
| next_hop_ip  | VARCHAR(16)  |            |                          |
| next_hop_mac | VARCHAR(17)  |            |                          |
| encrypted    | BOOLEAN      |            | `NOT NULL DEFAULT FALSE` |
| owner_id     | VARCHAR(256) |            |                          |
| project_id   | VARCHAR(256) |            |                          |
| status       | VARCHAR(8)   |            |                          |
| created_at   | VARCHAR(64)  |            |                          |
| created_by   | VARCHAR(256) |            |                          |
| updated_at   | VARCHAR(64)  |            |                          |
| updated_by   | VARCHAR(256) |            |                          |
| deleted_at   | VARCHAR(64)  |            |                          |
| deleted_by   | VARCHAR(256) |            |                          |

## network_filters

Rules of the packet filters. A filter belongs to the address of a virtual machine (`ip`) within
its tenant (`vni`) and to one direction of its traffic: `ingress` is applied to the route
towards the address and checks the source address of the packets, `egress` is applied to the
TAP-device of the virtual machine and checks their destination address. Each entry is one rule
of one of the two include-lists of the filter: `rule_type` is `IP_RANGE` or `PORT` and `spec` the
rule in its canonical notation, like `10.0.0.0/24`, `10.0.0.5-10.0.0.9`, `22` or `8000-8100`. A
filter without active rules is unrestricted. The rules are deleted together with the route
towards the address and move with it, when the route is updated to another address or tenant.

| field      | type         | is primary | constraints |
| ---------- | ------------ | ---------- | ----------- |
| uuid       | VARCHAR(40)  | x          |             |
| vni        | INTEGER      |            |             |
| ip         | VARCHAR(16)  |            |             |
| direction  | VARCHAR(8)   |            |             |
| rule_type  | VARCHAR(16)  |            |             |
| spec       | VARCHAR(64)  |            |             |
| owner_id   | VARCHAR(256) |            |             |
| project_id | VARCHAR(256) |            |             |
| status     | VARCHAR(8)   |            |             |
| created_at | VARCHAR(64)  |            |             |
| created_by | VARCHAR(256) |            |             |
| updated_at | VARCHAR(64)  |            |             |
| updated_by | VARCHAR(256) |            |             |
| deleted_at | VARCHAR(64)  |            |             |
| deleted_by | VARCHAR(256) |            |             |

## floating_ips

Floating IPs, which are translated by the gateway, and the internal address (`internal_ip`)
within the tenant (`vni`) of the network (`network_uuid`) they lead to.

| field        | type         | is primary | constraints          |
| ------------ | ------------ | ---------- | -------------------- |
| uuid         | VARCHAR(40)  | x          |                      |
| name         | VARCHAR(256) |            |                      |
| network_uuid | VARCHAR(40)  |            |                      |
| floating_ip  | VARCHAR(16)  |            |                      |
| internal_ip  | VARCHAR(16)  |            |                      |
| vni          | INTEGER      |            | `NOT NULL DEFAULT 0` |
| owner_id     | VARCHAR(256) |            |                      |
| project_id   | VARCHAR(256) |            |                      |
| status       | VARCHAR(8)   |            |                      |
| created_at   | VARCHAR(64)  |            |                      |
| created_by   | VARCHAR(256) |            |                      |
| updated_at   | VARCHAR(64)  |            |                      |
| updated_by   | VARCHAR(256) |            |                      |
| deleted_at   | VARCHAR(64)  |            |                      |
| deleted_by   | VARCHAR(256) |            |                      |
