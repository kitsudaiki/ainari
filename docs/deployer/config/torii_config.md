# Torii

## Options

### Root Configuration

| Parameter               | Type    | Default      | Description                                                                                        |
| ----------------------- | ------- | ------------ | -------------------------------------------------------------------------------------------------- |
| `debug`                 | boolean | `true`       | Enables debug mode for detailed logging and troubleshooting.                                       |
| `log_path`              | string  | `"/var/log"` | Path to the directory where log files will be stored.                                              |
| `skip_tls_verification` | boolean | `true`       | Set true to skip validation of https-connections, for example in case of self-singed certificates. |

### `api` Configuration

| Parameter       | Type    | Default     | Description                         |
| --------------- | ------- | ----------- | ----------------------------------- |
| `public_ip`     | string  | `"0.0.0.0"` | IP address for public API access.   |
| `public_port`   | integer | `11419`     | Port for public API access.         |
| `internal_ip`   | string  | `"0.0.0.0"` | IP address for internal API access. |
| `internal_port` | integer | `10419`     | Port for internal API access.       |

### `database` Configuration

| Parameter   | Type   | Default                  | Description                |
| ----------- | ------ | ------------------------ | -------------------------- |
| `file_path` | string | `"/etc/ainari/torii_db"` | Path to the database file. |

### `miko` Configuration

| Parameter | Type   | Default                    | Description                  |
| --------- | ------ | -------------------------- | ---------------------------- |
| `address` | string | `"http://127.0.0.1:11417"` | Address of the Miko service. |

### `ports` Configuration

| Parameter  | Type    | Default | Description                                      |
| ---------- | ------- | ------- | ------------------------------------------------ |
| `min_port` | integer | `10042` | Minimum port number for dynamic port allocation. |
| `max_port` | integer | `10043` | Maximum port number for dynamic port allocation. |

### `network` Configuration

| Parameter            | Type    | Default     | Description                                                                          |
| -------------------- | ------- | ----------- | ------------------------------------------------------------------------------------ |
| `overlay_iface`      | string  | `"veth-gw"` | Interface of the overlay network, `overlay_ingress` is attached to it.               |
| `underlay_iface`     | string  | `"eth0"`    | Interface of the underlay network, `underlay_ingress` is attached to it.             |
| `tenant_table_base`  | integer | `100`       | First kernel routing-table a tenant is given. The table of a tenant is `tenant_table_base + vni`. |

### `development` Configuration

!!! warning

    These settings are only for local development and testing. Never use them in a production deployment.

| Parameter         | Type    | Default | Description                                                                                                                          |
| ----------------- | ------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `single_node`     | boolean | `false` | Runs the single node setup: the uplink, the floating IPs and all VMs sit behind this one gateway, without underlay, tunnel or IPsec. |
| `uplink_iface`    | string  | -       | Interface facing the outside, on which the floating IPs are served. Required if `single_node` is enabled.                            |
| `uplink_next_hop` | string  | -       | Next hop behind the uplink, which all traffic leaving the virtual network is sent to. Required if `single_node` is enabled.          |

In the single node setup the floating IPs are translated only where the traffic crosses the uplink: packets entering through it are
translated to the VM (DNAT), packets of a VM leaving through it get the floating IP of the VM as source (SNAT). ARP requests for the
floating IPs on the uplink are answered by the gateway itself with the MAC of the uplink. At startup the routes towards `uplink_next_hop`
and the default route (`0.0.0.0`) are pointed at the uplink.

The uplink can be any interface, also a physical NIC: set `uplink_iface` to the NIC and `uplink_next_hop` to the router behind
it. The floating IPs then have to be unused addresses of that network. For local development
`scripts/setup_single_node_uplink.sh` creates a veth pair `uplink0` (10.0.0.254/24) towards a network namespace `torii-outside`
(10.0.0.1/24), from which the floating IPs are reachable (`sudo ip netns exec torii-outside ssh ubuntu@10.0.0.2`).

An example config-file for the single node setup is `example_configs/ainari/torii_single_node.toml`.

## Tenants

Unnumbered TAP-devices already get several VMs of the *same subnet* onto one host. They do not get
two VMs of the *same address* onto one host: a returning packet for `192.168.100.2` has to end up
on exactly one TAP, and nothing in the packet says which. That is what the **VNI** is for.

Every route, every packet-filter and every floating IP is stored under a `(vni, address)`-pair
instead of a bare address. `vni 0` is the shared tenant: it is what every payload defaults to,
what an interface that was never registered belongs to and what uses the `main` kernel
routing-table, so a setup that never mentions a VNI behaves exactly as it did before tenants
existed.

A VM never names its own tenant. The VNI is attached to a packet in one of three places, and
nowhere else:

| the packet                                    | where its VNI comes from                                       |
| --------------------------------------------- | -------------------------------------------------------------- |
| leaving a VM                                  | the port it arrived on, registered by `POST /interfaces/tap`   |
| arriving from another gateway                 | the VXLAN header it was carried in                             |
| arriving on the uplink for a floating IP      | the floating IP, which is unique across all tenants            |

The third one is the only way into a tenant from outside, and it is restricted to the interfaces
marked `fip_port` - the uplink. Without that restriction a VM could step into another tenant
simply by addressing its floating IP.

The overlay itself is VXLAN (RFC 7348) on UDP port 5555, which is what carries the VNI between two
gateways:

```
[ Ethernet | IPv4 | UDP 5555 | VXLAN (vni) | inner Ethernet frame ... ]
  14         20     8          8
```

That is **50 bytes** of overhead, so the underlay MTU has to be at least 1550 for 1500 byte VM
frames. `tcpdump -T vxlan 'udp port 5555'` decodes it and shows the VNI of every frame.

Traffic marked `encrypted: true` leaves the eBPF-datapath and is routed by the kernel, which has
no VNI. Each tenant therefore gets a routing-table of its own (`tenant_table_base + vni`),
selected by two `ip rule`s per TAP-device:

```
1000:  from all iif tap-00000001 lookup 101
1001:  from all iif tap-00000001 unreachable
```

The second rule matters: a plain table-rule falls through to the next one when its table has no
matching route, and the next one is eventually `main` - the table of tenant 0. Without the guard a
tenant that does not know a destination would quietly borrow the route of tenant 0.

One thing the kernel genuinely cannot express: xfrm selectors are plain address-pairs with no room
for a tenant. Two tenants that both want to protect the same address-pair with IPsec would
overwrite each other's policies, so the second one is refused with a `409` instead. Everything
that stays in the eBPF overlay has no such limit.

Hanami hands out one tenant per network and stores it with the addresses it reserves, so nothing
has to be configured by hand for the virtual machines it creates.

## Example

!!! info

    example config-file can be found in the repository under `example_configs/ainari/`

```toml
--8<-- "example_configs/ainari/torii.toml"
```
