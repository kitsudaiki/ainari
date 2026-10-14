# Docker-compose setup

## Overview

Runs every component of ainari as a docker-container on the host, defined in `docker-compose.yml`.
There is one gateway at the edge of the network (`torii-public`), which serves the floating
ip-addresses towards the host, and one gateway in front of every sakura-host (`torii-vmm`,
`torii-vmm-2`), which owns the TAP-devices of the virtual machines of that host. There are two
sakura-hosts, so a virtual machine can land on either of them and the traffic between the hosts
really crosses the edge-gateway. The components talk plain http to each other.

The only exception are the proxies of torii, over which the clients reach the api of the
sakura-hosts: every virtual machine is reachable at `https://<vm-uuid>.ainari.localhost:10042`.
All proxies share this port and `torii-public` picks the sakura-host by the server name of the
TLS-connection, without terminating it. So the external api of every sakura-host is served over
https by an nginx-container (`sakura-tls`, `sakura-2-tls`) with a certificate for
`*.ainari.localhost`, which is signed by the CA of the setup, see
[Certificates](#certificates).

## Architecture

```mermaid
flowchart TB
    host["host<br/>10.0.0.1 on veth-host<br/>api on 127.0.0.1:11416-11424<br/>proxies on 127.0.0.1:10042"]

    subgraph docker["docker-network ainari-net 172.30.0.0/16, mtu 1600"]
        public["torii-public 172.30.0.10<br/>floating ip-NAT 10.0.0.x to 192.168.100.x<br/>proxies on 10042"]
        control["miko, omamori, izakaya, ryokan, onsen, hanami"]
        subgraph host1["network-namespace of torii-vmm 172.30.0.20"]
            vmm1["torii-vmm<br/>TAP-devices"]
            sakura1["sakura<br/>cloud-hypervisor"]
            tls1["sakura-tls<br/>https on 9443"]
        end
        subgraph host2["network-namespace of torii-vmm-2 172.30.0.21"]
            vmm2["torii-vmm-2<br/>TAP-devices"]
            sakura2["sakura-2<br/>cloud-hypervisor"]
            tls2["sakura-2-tls<br/>https on 9443"]
        end
    end

    host -- "veth-host / veth-gw (uplink)" --- public
    public -- "underlay (eth0)" --- vmm1
    public -- "underlay (eth0)" --- vmm2
    vmm1 --- vm1["virtual machines"]
    vmm2 --- vm2["virtual machines"]
    control -. "http" .- public
    control -. "http" .- sakura1
    control -. "http" .- sakura2
    public -. "proxies, tls passed through" .- tls1
    public -. "proxies, tls passed through" .- tls2
    tls1 -. "http" .- sakura1
    tls2 -. "http" .- sakura2
```

Beside the gateways and the sakura-hosts the setup runs `miko` (auth), `omamori` (secrets and
public-keys), `ryokan` (images) with `onsen` (storage) and `hanami` (the api for the virtual
machines). There is no dashboard.

A sakura has no network of its own, because it shares the network-namespace of its gateway. Its
address `http://sakura:10420` (and `http://sakura-2:10420`) is an alias of its gateway in the dns
of docker: the host is listed under its own name, while the address still leads to the
network-namespace, which it shares, and hanami derives the torii of a host from it again. Its
external api, which the proxies forward to, is `https://sakura:9443` (and
`https://sakura-2:9443`), served by its nginx-container in the same network-namespace.

Hanami picks one of the sakura-hosts for every new virtual machine. Both hosts serve addresses
out of the same subnet of a network: the edge-gateway gets a route towards the host, which really
runs a virtual machine, and the gateway of a host sends everything else to the edge-gateway, so
two virtual machines on different hosts reach each other over it.

## Requirements

- docker with the compose-plugin (`docker compose`)
- `/dev/kvm` and `/dev/net/tun` on the host, because the virtual machines are really booted
- a kernel with eBPF/XDP support, because the gateways attach their datapath to the interfaces
- `sudo`, because the setup-script connects the host to the edge-gateway and adds NAT-rules
- `openssl`, because the setup-script creates the CA and the certificate of the proxies
- `make`
- `go` in the version of `src/cli/ainarictl/go.mod`, to build the cli
- `python3` with the dependencies of the sdk for the end-to-end test:

    ```bash
    python3 -m venv .venv
    .venv/bin/pip install -r src/sdk/python/ainari_sdk/requirements.txt
    ```

## Usage

1. start the stack. It builds the images first, from the debian-based Dockerfiles of
   `dockerfiles/debian_based` (see [Packages of the docker-images](../docker_images.md)). It needs
   root, because it creates the veth-pair towards the edge-gateway and the NAT-rules for the
   floating ip-addresses. `docker compose up` alone is not enough: `torii-public` waits for that
   uplink and the host has no route to a virtual machine without it.

    ```bash
    make up local    # runs: sudo ./testing/local_stack/setup_local_stack.sh
    ```

2. trust the CA of the setup `temporary_files/local/ainari-local-ca.crt` once, see
   [Certificates](#certificates). It stays the same over all runs. Without it, the cli has to be
   called with `--insecure` for everything, which is done on a sakura-host, and the dashboard
   fails there with *API did not respond*.

3. build the cli. The binary is placed beside its sources.

    ```bash
    cd src/cli/ainarictl && go build .
    ```

4. point the cli at the stack and check, that both sakura-hosts are registered. They are listed
   as `http://sakura:10420` and `http://sakura-2:10420`.

    ```bash
    source src/cli/ainarictl/test_auth.sh    # AINARI_ADDRESS, AINARI_USER, AINARI_PASSPHRASE
    ainarictl host list
    ```

5. create a virtual machine. `-j` prints json, so the uuids can be read with `jq`. `vm create`
   installs the image over the proxy of the virtual machine, so it needs the trusted CA or
   `--insecure`.

    ```bash
    ssh-keygen -t ed25519 -N '' -f ~/.ssh/ainari_local
    ainarictl public_key upload -k ~/.ssh/ainari_local.pub local-key

    wget https://cloud-images.ubuntu.com/noble/current/noble-server-cloudimg-amd64.img
    ainarictl image create disk -i noble-server-cloudimg-amd64.img ubuntu-noble

    ainarictl network create -s 192.168.100.1/24 local-net
    ainarictl vm create -c 2 -m 2048 -d 10 -u NETWORK_UUID -i IMAGE_UUID -k KEY_UUID vm1
    ainarictl vm get VM_UUID                 # repeat, until 'vm_state' is RUNNING
    ainarictl floating_ip add -n vm1-fip -v VM_UUID   # or: floating_ip add + floating_ip attach FIP_UUID VM_UUID
    ```

6. connect to the virtual machine over its floating ip-address, which the host reaches over
   `veth-host`.

    ```bash
    ssh -i ~/.ssh/ainari_local ubuntu@FLOATING_IP
    ```

   The sakura-host, which got the virtual machine, is reachable over the proxy
   `https://<vm-uuid>.ainari.localhost:10042`, the `TORII ADDRESS` in the output of `vm create`.
   `*.localhost` resolves to `127.0.0.1` without any DNS-entry. Torii picks the sakura-host by the
   server name of the TLS-connection, so the proxy is only reachable over https and its name, not
   over `127.0.0.1` directly. Its api requires a token, so `ainarictl` is the easier way to ask
   it.

7. stop the stack again. This also removes the veth-pair and throws all data away.

    ```bash
    make down local    # runs: sudo ./testing/local_stack/setup_local_stack.sh --down
    ```

The api is reachable on the host at:

| Component | Address                  |
| --------- | ------------------------ |
| miko      | `http://127.0.0.1:11417` |
| hanami    | `http://127.0.0.1:11418` |
| ryokan    | `http://127.0.0.1:11416` |
| omamori   | `http://127.0.0.1:11421` |
| izakaya   | `http://127.0.0.1:11423` |
| omikuji   | `http://127.0.0.1:11424` |
| torii     | `http://127.0.0.1:11419` |

The sakura-hosts are only reachable over the proxies of torii at
`https://<vm-uuid>.ainari.localhost:10042`, the `torii_address` of a virtual machine.

The admin-user is `asdf` with the passphrase `asdfasdf`; both can be overwritten with
`AINARI_USER` and `AINARI_PASSPHRASE` before the setup-script is started.

## Certificates

`make up local` creates the CA `temporary_files/local/ainari-local-ca.crt` with its first start
and keeps it over all runs. It signs the certificate of the proxies for `*.ainari.localhost`
(`temporary_files/local/certs`), which the nginx-containers of the sakura-hosts serve. Its
name-constraints limit it to the names below `localhost`, so it can't be misused for any other
host, even if its key leaks.

The CA has to be trusted once, for the browser of the dashboard and for the cli. The commands for
the trust-store of the system, Firefox and Chrome are described in
[The CAs of the local setups](https_ca.md) with `SETUP=local`, for example:

```bash
SETUP=local
# system: curl and the go-cli
sudo cp temporary_files/$SETUP/ainari-$SETUP-ca.crt /usr/local/share/ca-certificates/ainari-$SETUP-ca.crt && sudo update-ca-certificates
# Chrome and Chromium
certutil -d sql:$HOME/.pki/nssdb -A -t "C,," -n ainari-$SETUP-ca -i temporary_files/$SETUP/ainari-$SETUP-ca.crt
```

The browser has to be restarted completely afterwards. Without the trusted CA:

- the dashboard fails with *API did not respond* for everything, which is done on a sakura-host:
  *Show tasks*, creating, starting, stopping and snapshotting a virtual machine. The other pages
  work, because they only talk to the api over plain http.
- the cli has to be called with `--insecure` for the same commands.
- the python-sdk needs `REQUESTS_CA_BUNDLE` or `verify_connection=False`. The end-to-end test and
  `prepare_resources.py` skip the verification anyway.

Deleting `temporary_files/local` creates a new CA with the next start, which has to be trusted
again.

## End-to-end test

`testing/ainari_test/vm_lifecycle_test.py` walks through the whole life-cycle with the python-sdk,
from the ssh-key-pair up to the login into the virtual machines over ssh. It creates two virtual
machines out of the same image, with the same public key and within the same network, and gives
every one of them its own floating ip-address.

```bash
make up local
.venv/bin/python testing/ainari_test/vm_lifecycle_test.py
make down local
```

The test waits at its start until both sakura-hosts are registered in hanami.
`AINARI_SAKURA_HOSTS` sets how many hosts it waits for and `AINARI_VIRTUAL_MACHINES` how many
virtual machines it creates. Every one of them gets 2 cores and 2 GiB memory, so the two of the
default need 4 GiB on the host.

## Running from the tools-container

The image `dockerfiles/Dockerfile_local_test_tools` contains all tools of the requirements above,
which can be installed in an image, so they don't have to be installed on the host. The container
runs in the network- and pid-namespace of the host and uses the docker of the host, so the setup
is deployed on the host, exactly like without the container. The repository is mounted with the
same path as on the host, because the docker of the host resolves the paths of the setup.
`HOST_UID` and `HOST_GID` let the container run as the user of the host, so the files, which the
setup creates in the repository, belong to this user; the user has sudo within the container.

Build the image once in the root of the repository:

```bash
docker build -f dockerfiles/Dockerfile_local_test_tools -t ainari/local-test-tools .
```

Start the container in the root of the repository:

```bash
docker run --rm -it --privileged --network host --pid host \
    -e HOST_UID=$(id -u) -e HOST_GID=$(id -g) \
    -v /var/run/docker.sock:/var/run/docker.sock \
    -v "$PWD:$PWD" -w "$PWD" \
    ainari/local-test-tools
```

Within the container, the setup is started, tested and stopped with the same commands as on the
host. The python of the container has the dependencies of the sdk already. The trust-store of the
container doesn't know the CA of the setup and is thrown away with the container, so the cli
within the container needs `--insecure` for the commands, which are done on a sakura-host. The
browser on the host still needs the trusted CA, see [Certificates](#certificates):

```bash
make up local
python3 testing/ainari_test/vm_lifecycle_test.py
cd src/cli/ainarictl && go build . && cd -    # the cli, if needed
make down local
```

What still has to be on the host: docker, `/dev/kvm`, `/dev/net/tun` and a kernel with
eBPF/XDP-support. The setup is reachable from the host like without the container: the api at
`http://127.0.0.1:11417`, the proxies at `https://<vm-uuid>.ainari.localhost:10042` and the
floating ip-addresses over `veth-host`, which the setup-script creates on the host. The setup
keeps running, when the container is left; it is stopped with `make down local` out of a new
container.

## Hints

- **What the setup-script does as root:** it creates the CA and the certificate of the proxies in
  `temporary_files/local`, starts the containers with `docker compose`, creates
  the veth-pair `veth-host` / `veth-gw`, gives the host side the address `10.0.0.1/24` and the
  gateway side `10.0.0.254/24`, moves the gateway side into the network-namespace of
  `torii-public`, enables `net.ipv4.ip_forward` and adds a MASQUERADE-rule for `10.0.0.0/24`, so
  the virtual machines reach the internet through the host. The host only ever sees the floating
  ip-addresses; the internal addresses of the virtual machines (`192.168.100.0/24`) stay behind
  the gateway.
- **Privileges:** the three gateways run privileged, because they attach their eBPF-programs to
  the interfaces and create the TAP-devices. The sakura-hosts run as the user `ainari`: the
  hypervisor-binary carries `cap_net_admin+ep` as a file-capability and the containers get
  `CAP_NET_ADMIN` over `cap_add`. `/dev/kvm` is opened through its group, whose id the
  setup-script reads from the device and hands to the containers with `group_add`.
- **State:** every run starts with empty databases; the databases and boot-disks live in the
  containers.
- **Configs:** the configs of the components are in `testing/local_stack/configs`. The two gateways
  in front of the sakura-hosts share `torii_vmm.toml`; the sakura-hosts need one config each
  (`sakura.toml`, `sakura_2.toml`), because every host registers itself with the address out of
  its own config.
- **Conflicts:** the setup-script refuses to start, if another interface of the host has an
  address in `10.0.0.0/24`, for example the one of the kind-setup. Stop the other setup first.
