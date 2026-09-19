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

| Parameter        | Type   | Default     | Description                                                              |
| ---------------- | ------ | ----------- | ------------------------------------------------------------------------ |
| `overlay_iface`  | string | `"veth-gw"` | Interface of the overlay network, `overlay_ingress` is attached to it.   |
| `underlay_iface` | string | `"eth0"`    | Interface of the underlay network, `underlay_ingress` is attached to it. |

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

## Example

!!! info

    example config-file can be found in the repository under `example_configs/ainari/`

```toml
--8<-- "example_configs/ainari/torii.toml"
```
