# Local stack

Runs every component of ainari as a docker-container on one machine, with one gateway at the
edge of the network, which serves the floating ip-addresses towards the host, and one gateway in
front of every sakura-host, which owns the TAP-devices of the virtual machines of that host.
There are two sakura-hosts, so a virtual machine can land on either of them and the traffic
between the hosts really crosses the edge-gateway. It is meant for development and testing only.

```
     [ host ]  10.0.0.1 on veth-host
                       |
                       v  veth-gw (uplink)
  ============================================================
                [ torii-public ]  172.30.0.10
    floating ip-NAT 10.0.0.x <--> 192.168.100.x
    proxy-ports 10042-10053 --> the sakura-hosts
    routes 192.168.100.x --> the torii of the host of that address
  ============================================================
                       |  underlay (eth0, 172.30.0.0/16)
          +------------+------------+
          v                         v
  =========================   =========================
    [ torii-vmm ] .0.20         [ torii-vmm-2 ] .0.21
    routes 192.168.100.x        the same for the
    --> tap-device of the       virtual machines of
    virtual machine             the second host
    [ sakura ] shares this      [ sakura-2 ] shares
    network-namespace           this network-namespace
  =========================   =========================
```

Beside the gateways and the sakura-hosts the setup runs `miko` (auth), `omamori` (secrets and
public-keys), `ryokan` (images) with `onsen` (storage) and `hanami` (the api for the virtual
machines).

Hanami picks one of the sakura-hosts for every new virtual machine, so which host runs it differs
from run to run. The hosts are told apart by their address, which `ainarictl host list` shows as
`http://sakura:11420` and `http://sakura-2:11420`. A sakura has no network of its own, because it
shares the network-namespace of its gateway, so these names are aliases of the gateways in the
dns of docker: the host is listed under its own name, while the address still leads to the
network-namespace, which it shares, and hanami derives the torii of a host from it again.

Both hosts serve addresses out of the same subnet of a network: the edge-gateway gets a route
towards the host, which really runs a virtual machine, and the gateway of a host sends everything
else to the edge-gateway, so two virtual machines on different hosts reach each other over it.

## Requirements

- docker with the compose-plugin
- `/dev/kvm` and `/dev/net/tun` on the host, because the virtual machines are really booted
- a kernel with eBPF/XDP support, because the gateways attach their datapath to the interfaces
- `go` in the version of `src/cli/ainarictl/go.mod`, to build the cli
- `python3` with the dependencies of the sdk for the test:

```bash
python3 -m venv .venv
.venv/bin/pip install -r src/sdk/python/ainari_sdk/requirements.txt
```

## Getting started

1. start the stack. It needs root, because it creates the veth-pair towards the edge-gateway and
   the NAT-rules for the floating ip-addresses. `docker compose up` alone is not enough:
   `torii-public` waits for that uplink and the host has no route to a virtual machine without it.

    ```bash
    sudo ./scripts/setup_local_stack.sh
    ```

2. build the cli. The binary is placed beside its sources.

    ```bash
    cd src/cli/ainarictl && go build .
    ```

3. point the cli at the stack and check, that both sakura-hosts are registered.

    ```bash
    source src/cli/ainarictl/test_auth.sh    # AINARI_ADDRESS, AINARI_USER, AINARI_PASSPHRASE
    ainarictl host list
    ```

4. create a virtual machine. `-j` prints json, so the uuids can be read with `jq`.

    ```bash
    ssh-keygen -t ed25519 -N '' -f ~/.ssh/ainari_local
    ainarictl public_key upload -k ~/.ssh/ainari_local.pub local-key

    wget https://cloud-images.ubuntu.com/noble/current/noble-server-cloudimg-amd64.img
    ainarictl image create disk -i noble-server-cloudimg-amd64.img ubuntu-noble

    ainarictl network create -s 192.168.100.1/24 local-net
    ainarictl vm create -c 2 -m 2147483648 -u NETWORK_UUID -i IMAGE_UUID -k KEY_UUID vm1
    ainarictl vm get VM_UUID                 # repeat, until 'is_created' is true
    ainarictl floating_ip add -n vm1-fip -u NETWORK_UUID -i INTERNAL_IP
    ```

5. connect to the virtual machine over its floating ip-address, which the host reaches over
   `veth-host`.

    ```bash
    ssh -i ~/.ssh/ainari_local ubuntu@FLOATING_IP
    ```

   The sakura-host, which got the virtual machine, is reachable at `http://127.0.0.1:TORII_PORT`
   with the proxy-port from the output of `vm create`. Its api requires a token, so `ainarictl`
   is the easier way to ask it.

## End-to-end test

`testing/local_stack/vm_lifecycle_test.py` walks through the same steps with the python-sdk, from
the ssh-key-pair up to the login into the virtual machine.

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

The test waits at its start until both sakura-hosts are registered in hanami and prints them.
`AINARI_SAKURA_HOSTS` sets how many hosts it waits for, if the setup is run with another number
of them.

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

The three gateways run privileged, because they attach their eBPF-programs to the interfaces and
create the TAP-devices of the virtual machines.

The sakura-hosts do not: they run as the user `ainari`. Attaching a virtual machine to an
existing TAP-device is their only privileged operation, so the hypervisor-binary carries
`cap_net_admin+ep` as a file-capability and the containers get `CAP_NET_ADMIN` in their bounding
set over `cap_add`. `/dev/kvm` is opened through its group, whose id the setup-script reads from
the device and hands to the containers with `group_add`.

## State

The setup keeps no state: every run starts with empty databases, and the databases and boot-disks
live in the containers. `docker compose down` throws everything away.

## Configs

The configs of the components are in `deploy/local_stack/configs`. They differ from
`example_configs/ainari` only in the addresses: the components talk to each other over the
container-names, while the public addresses are the ports published on the host.

The two gateways in front of the sakura-hosts share `torii_vmm.toml`, because they only differ in
their address on the underlay, which they get from docker. The sakura-hosts need one config each
(`sakura.toml` and `sakura_2.toml`), because every host registers itself in hanami with the
address out of its own config.
