# Readme

## setup_local_stack.sh

### Purpose

Starts the local docker-compose setup of `docker-compose.yml` with all components and two
sakura-hosts and connects the host to it. The images are built from the debian-based Dockerfiles
of `dockerfiles/debian_based` and are always rebuilt before, so the setup never runs an older
version than the one of the working tree. The script injects a veth-pair into the
gateway at the edge (`torii-public`), so the host reaches the floating ip-addresses of the virtual
machines, and lets the host forward and masquerade the traffic of the virtual machines towards the
internet. See [Docker-compose setup](../../docs/developer/local_testing/docker_compose_setup.md)
for the details of the setup.

### Usage

```bash
make up local                             # or sudo ./testing/local_stack/setup_local_stack.sh
make down local                           # or sudo ./testing/local_stack/setup_local_stack.sh --down
```

### Limitations

- The whole script needs root.
- The host needs docker with compose, `/dev/kvm` and a kernel with eBPF/XDP-support.
- Every start removes the previous containers, so no state is kept between two runs.
- The floating ip-addresses `10.0.0.0/24` must not be used by any other interface of the host, so
  the setup can't run together with the kind-setup, the vagrant-setup or the uplink of
  [setup_single_node_uplink.sh](setup_single_node_uplink.sh).
- `net.ipv4.ip_forward` stays enabled on the host after `--down`, because docker needs it for its
  own networks as well.
- The virtual machines only reach the internet over the first default-route of the host.
- The api is only reachable over plain http.

## setup_single_node_uplink.sh

### Purpose

Creates the uplink for a torii, which runs directly on the host for development (see
`example_configs/ainari/torii_single_node.toml` and
[Development](../../docs/developer/development.md)). The "outside" is a network-namespace
`torii-outside`, which is connected to the host by a veth-pair:

```text
[ netns torii-outside ]  outside0 10.0.0.1/24    (uplink_next_hop)
           |
[ host, torii ]          uplink0  10.0.0.254/24  (uplink_iface)
```

The floating ip-addresses (`10.0.0.2` - `10.0.0.253`) are reachable from within the namespace.
Checksum-offloading is switched off on both ends, because the gateway rewrites the addresses with
incremental checksum-updates.

### Usage

```bash
sudo ./testing/local_stack/setup_single_node_uplink.sh         # create it (replaces an existing one)
sudo ./testing/local_stack/setup_single_node_uplink.sh down    # remove it again
sudo ip netns exec torii-outside ssh ubuntu@10.0.0.2
```

### Limitations

- Needs root and `ethtool`.
- The addresses are fixed within the script and collide with the floating ip-addresses of the
  local setups, so it can't be used together with them. The setups refuse to start, while
  `uplink0` exists.
- The virtual machines are only reachable from within the namespace and have no internet.
- Only for local development: on a real host the uplink is the physical network-interface and the
  next hop the router behind it.