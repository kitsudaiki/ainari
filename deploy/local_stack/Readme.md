# Local stack

Runs every component of ainari as a docker-container on one machine, with two gateways: one at
the edge of the network, which serves the floating ip-addresses towards the host, and one in
front of the sakura-host, which owns the TAP-devices of the virtual machines. It is meant for
development and testing only.

```
     [ host ]  10.0.0.1 on veth-host
                       |
                       v  veth-gw (uplink)
  =========================================================
                [ torii-public ]  172.30.0.10
    floating ip-NAT 10.0.0.x <--> 192.168.100.x
    proxy-ports 10042-10053 --> sakura
  =========================================================
                       |  underlay (eth0, 172.30.0.0/16)
                       v
  =========================================================
                 [ torii-vmm ]  172.30.0.20
    routes 192.168.100.x --> tap-device of the virtual machine
    [ sakura ] shares this network-namespace
  =========================================================
```

Beside the two gateways the setup runs `miko` (auth), `omamori` (secrets and public-keys),
`ryokan` (images) with `onsen` (storage) and `hanami` (the api for the virtual machines).

## Requirements

- docker with the compose-plugin
- `/dev/kvm` and `/dev/net/tun` on the host, because the virtual machines are really booted
- a kernel with eBPF/XDP support, because the gateways attach their datapath to the interfaces
- `python3` with the dependencies of the sdk for the test:

```bash
python3 -m venv .venv
.venv/bin/pip install -r src/sdk/python/ainari_sdk/requirements.txt
```

## Usage

```bash
# starts the containers and connects the host to the gateway (needs root)
sudo ./scripts/setup_local_stack.sh

# walks through the whole life-cycle of a virtual machine (as a normal user)
.venv/bin/python testing/local_stack/vm_lifecycle_test.py

# stops everything and removes the veth-pair again
sudo ./scripts/setup_local_stack.sh --down
```

The api is reachable on the host at `miko http://127.0.0.1:11417`, `hanami :11418`,
`ryokan :11416`, `omamori :11421` and `torii :11419`. The admin-user is `asdf` with the
passphrase `asdfasdf`, like in the other tests; both can be overwritten with `AINARI_USER` and
`AINARI_PASSPHRASE` before the setup-script is started.

## What the setup-script does as root

- starts the containers with `docker compose`
- creates the veth-pair `veth-host` / `veth-gw`, gives the host side the address `10.0.0.1/24`
  and the gateway side `10.0.0.254/24`, and moves the gateway side into the network-namespace of
  `torii-public`
- enables `net.ipv4.ip_forward` and adds a MASQUERADE-rule for `10.0.0.0/24`, so the virtual
  machines reach the internet through the host

The host only ever sees the floating ip-addresses. The internal addresses of the virtual machines
(`192.168.100.0/24`) stay behind the gateway, which translates them.

## Privileges

The two gateways run privileged, because they attach their eBPF-programs to the interfaces and
create the TAP-devices of the virtual machines.

Sakura does not: it runs as the user `ainari`. Attaching a virtual machine to an existing
TAP-device is its only privileged operation, so the hypervisor-binary carries
`cap_net_admin+ep` as a file-capability and the container gets `CAP_NET_ADMIN` in its bounding
set over `cap_add`. `/dev/kvm` is opened through its group, whose id the setup-script reads from
the device and hands to the container with `group_add`.

## State

The setup keeps no state: every run starts with empty databases, and the databases and boot-disks
live in the containers. `docker compose down` throws everything away.

## Configs

The configs of the components are in `deploy/local_stack/configs`. They differ from
`example_configs/ainari` only in the addresses: the components talk to each other over the
container-names, while the public addresses are the ports published on the host.
