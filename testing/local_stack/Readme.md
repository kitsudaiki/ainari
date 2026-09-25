# Local stack

Runs every component of ainari on one machine, with one gateway at the edge of the network, which
serves the floating ip-addresses towards the host, and one gateway in front of every sakura-host,
which owns the TAP-devices of the virtual machines of that host. There are two sakura-hosts, so a
virtual machine can land on either of them and the traffic between the hosts really crosses the
edge-gateway. It is meant for development and testing only.

There are two ways to run it, with the same topology, the same ports on the host and the same
end-to-end test:

| Setup | Start / stop                        | Runs on                                               |
| ----- | ----------------------------------- | ----------------------------------------------------- |
| local | `make up local` / `make down local` | docker compose, plain http                            |
| kind  | `make up kind` / `make down kind`   | a kind-cluster with the helm-chart of `deploy/k8s`, https |

Both use the same floating ip-addresses, so only one of them can run at a time.

## Topology

```
     [ host ]  10.0.0.1 on veth-host (local) / veth-kind (kind)
                       |
                       v  veth-gw (uplink)
  ============================================================
                [ torii-public ]
    floating ip-NAT 10.0.0.x <--> 192.168.100.x
    proxy-ports 10042-10053 --> the sakura-hosts
    routes 192.168.100.x --> the torii of the host of that address
  ============================================================
                       |  underlay (eth0)
          +------------+------------+
          v                         v
  =========================   =========================
    [ torii-vmm ]               [ torii-vmm-2 ]
    routes 192.168.100.x        the same for the
    --> tap-device of the       virtual machines of
    virtual machine             the second host
    [ sakura ] shares this      [ sakura-2 ] shares
    network-namespace           this network-namespace
  =========================   =========================
```

Beside the gateways and the sakura-hosts the setup runs `miko` (auth), `omamori` (secrets and
public-keys), `ryokan` (images) with `onsen` (storage) and `hanami` (the api for the virtual
machines). The kind-setup additionally runs the dashboard.

Hanami picks one of the sakura-hosts for every new virtual machine, so which host runs it differs
from run to run. The hosts are told apart by their address, which `ainarictl host list` shows. A
sakura has no network of its own, because it shares the network-namespace of its gateway, so its
address leads to that network-namespace, and hanami derives the torii of a host from it again.

Both hosts serve addresses out of the same subnet of a network: the edge-gateway gets a route
towards the host, which really runs a virtual machine, and the gateway of a host sends everything
else to the edge-gateway, so two virtual machines on different hosts reach each other over it.

## Requirements

### Both setups

- docker (the engine), because the local setup runs its containers in it and kind runs its node
  in it. Both build their images with it.
- `/dev/kvm` and `/dev/net/tun` on the host, because the virtual machines are really booted
- a kernel with eBPF/XDP support, because the gateways attach their datapath to the interfaces
- `sudo`, because the setup-scripts connect the host to the edge-gateway and add NAT-rules
- `make`
- `go` in the version of `src/cli/ainarictl/go.mod`, to build the cli
- `python3` with the dependencies of the sdk for the end-to-end test:

```bash
python3 -m venv .venv
.venv/bin/pip install -r src/sdk/python/ainari_sdk/requirements.txt
```

### Local setup (`make up local`)

- the compose-plugin of docker (`docker compose`)

### Kind setup (`make up kind`)

- `kind`. If it is not installed, `make` downloads the version of `KIND_VERSION` in the
  `Makefile` with `curl` into `temporary_files/bin`.
- `kubectl` and `helm`
- `openssl`, to create the CA of the setup
- access to github, because cert-manager is installed into the cluster from there

The kind-setup doesn't need docker compose.

## Local setup

### Getting started

1. start the stack. It needs root, because it creates the veth-pair towards the edge-gateway and
   the NAT-rules for the floating ip-addresses. `docker compose up` alone is not enough:
   `torii-public` waits for that uplink and the host has no route to a virtual machine without it.

    ```bash
    make up local    # runs: sudo ./scripts/setup_local_stack.sh
    ```

2. build the cli. The binary is placed beside its sources.

    ```bash
    cd src/cli/ainarictl && go build .
    ```

3. point the cli at the stack and check, that both sakura-hosts are registered. They are listed
   as `http://sakura:11420` and `http://sakura-2:11420`, which are aliases of their gateways in
   the dns of docker.

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
    ainarictl vm get VM_UUID                 # repeat, until 'vm_state' is RUNNING
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

6. stop the stack again. This also removes the veth-pair.

    ```bash
    make down local    # runs: sudo ./scripts/setup_local_stack.sh --down
    ```

### End-to-end test

`testing/local_stack/vm_lifecycle_test.py` walks through the same steps with the python-sdk, from
the ssh-key-pair up to the login into the virtual machines. It creates two virtual machines out
of the same image, with the same public key and within the same network, and gives every one of
them its own floating ip-address. Which sakura-host runs which of them is decided by hanami, so
they can end up on the same host or on different ones.

```bash
make up local

# walks through the whole life-cycle of two virtual machines (as a normal user)
.venv/bin/python testing/local_stack/vm_lifecycle_test.py

make down local
```

The api is reachable on the host at `miko http://127.0.0.1:11417`, `hanami :11418`,
`ryokan :11416`, `omamori :11421` and `torii :11419`. The admin-user is `asdf` with the
passphrase `asdfasdf`, like in the other tests; both can be overwritten with `AINARI_USER` and
`AINARI_PASSPHRASE` before the setup-script is started.

The test waits at its start until both sakura-hosts are registered in hanami and prints them.
`AINARI_SAKURA_HOSTS` sets how many hosts it waits for, if the setup is run with another number
of them, and `AINARI_VIRTUAL_MACHINES` how many virtual machines it creates. Every one of them
gets 2 cores and 2 GiB memory, so the two of the default need 4 GiB on the host.

### What the setup-script does as root

- starts the containers with `docker compose`
- creates the veth-pair `veth-host` / `veth-gw`, gives the host side the address `10.0.0.1/24`
  and the gateway side `10.0.0.254/24`, and moves the gateway side into the network-namespace of
  `torii-public`
- enables `net.ipv4.ip_forward` and adds a MASQUERADE-rule for `10.0.0.0/24`, so the virtual
  machines reach the internet through the host

The host only ever sees the floating ip-addresses. The internal addresses of the virtual machines
(`192.168.100.0/24`) stay behind the gateway, which translates them.

### Privileges

The three gateways run privileged, because they attach their eBPF-programs to the interfaces and
create the TAP-devices of the virtual machines.

The sakura-hosts do not: they run as the user `ainari`. Attaching a virtual machine to an
existing TAP-device is their only privileged operation, so the hypervisor-binary carries
`cap_net_admin+ep` as a file-capability and the containers get `CAP_NET_ADMIN` in their bounding
set over `cap_add`. `/dev/kvm` is opened through its group, whose id the setup-script reads from
the device and hands to the containers with `group_add`.

### State

The setup keeps no state: every run starts with empty databases, and the databases and boot-disks
live in the containers. `docker compose down` throws everything away.

### Configs

The configs of the components are in `deploy/local_stack/configs`. They differ from
`example_configs/ainari` only in the addresses: the components talk to each other over the
container-names, while the public addresses are the ports published on the host.

The two gateways in front of the sakura-hosts share `torii_vmm.toml`, because they only differ in
their address on the underlay, which they get from docker. The sakura-hosts need one config each
(`sakura.toml` and `sakura_2.toml`), because every host registers itself in hanami with the
address out of its own config.

## Kind setup

### Getting started

```bash
make up kind      # runs scripts/setup_kind_stack.sh, asks for sudo for the network-setup
make down kind    # deletes the cluster and removes the veth-pair again
```

`make up kind` builds the images, creates the cluster, installs cert-manager and the helm-chart of
`deploy/k8s/ainari` and connects the host to the edge-gateway. Like the local setup, every run
starts with empty databases. The cluster (`deploy/k8s/kind/cluster.yaml`) has one node, the values
(`deploy/k8s/kind/values.yaml`) turn off the ingresses and wireguard and use the locally built
images.

Like on every kubernetes-cluster, the components only talk https, over a nginx-sidecar with a
certificate of cert-manager, and skip the verification of the certificates. The api is mapped to
the same ports on the host as in the local setup, but with https:

- miko `https://127.0.0.1:11417`, hanami `:11418`, ryokan `:11416`, omamori `:11421`,
  torii `:11419`
- the dashboard `https://127.0.0.1:11422`

The steps 2 to 5 of the local setup work the same way, with `AINARI_ADDRESS` pointing to miko
over https. Without the CA of the setup in the trust-store of the system (see below), the cli
has to skip the verification of the certificates:

```bash
AINARI_ADDRESS=https://127.0.0.1:11417 ainarictl --insecure host list
```

The sakura-hosts are listed as `https://sakura-0.sakura.ainari.svc.cluster.local:8443` and
`https://sakura-1…`, the names of their pods. The cluster can be inspected with:

```bash
kubectl --context kind-ainari --namespace ainari get pods
```

### End-to-end test

The same test as for the local setup, only with the address of miko over https. The test skips
the verification of the certificates anyway.

```bash
make up kind
AINARI_MIKO_ADDRESS=https://127.0.0.1:11417 .venv/bin/python testing/local_stack/vm_lifecycle_test.py
make down kind
```

### The CA of the kind-setup

All certificates are signed by one CA, which `make up kind` creates once in
`temporary_files/kind/ainari-kind-ca.crt` and keeps over all runs. Trusted once, the browser and
the clients accept all endpoints of the setup, also the proxy-ports of torii, without any
exceptions. The CA is limited by name-constraints to `127.0.0.1`, `localhost` and names below
`cluster.local`, so it can't be misused for any other host, even if its key leaks.

- The trust-store of the system (debian/ubuntu), which is used by curl and the go-cli, so
  `ainarictl` works without `--insecure`. The file has to end with `.crt` and has to be in
  `/usr/local/share/ca-certificates`, otherwise it is skipped. `update-ca-certificates` reports
  `1 added`:

    ```bash
    sudo cp temporary_files/kind/ainari-kind-ca.crt /usr/local/share/ca-certificates/ainari-kind-ca.crt && sudo update-ca-certificates
    ```

    While the kind-setup is running, the api then answers without skipping the verification:

    ```bash
    curl https://127.0.0.1:11417/v1alpha/is_ready
    ```

    To remove the CA from the trust-store of the system again:

    ```bash
    sudo rm /usr/local/share/ca-certificates/ainari-kind-ca.crt && sudo update-ca-certificates --fresh
    ```

Firefox and Chromium don't use the trust-store of the system on linux, but their own
nss-databases, so the CA has to be added to them separately, even if it is already in the one of
the system:

- Firefox: an enterprise-policy imports the CA at every start into every profile. It uses the file
  of the trust-store of the system from above, so that step has to be done first. The command
  overwrites an existing `/etc/firefox/policies/policies.json`, so check before, that there is
  none yet:

    ```bash
    sudo mkdir -p /etc/firefox/policies && echo '{"policies":{"Certificates":{"Install":["/usr/local/share/ca-certificates/ainari-kind-ca.crt"]}}}' | sudo tee /etc/firefox/policies/policies.json
    ```

    Firefox has to be closed completely and started again afterwards. `about:policies` shows the
    policy as active. Without the policy, the CA can also be imported by hand: *Settings → Privacy
    & Security → Certificates → View Certificates → Authorities → Import*, then choose *Trust this
    CA to identify websites*. To remove the policy again:
    `sudo rm /etc/firefox/policies/policies.json`. The CA, which Firefox imported already, stays
    in its store, until it is deleted under *Authorities*.

- Chrome and Chromium use the nss-database of the user in `~/.pki/nssdb` (the snap of Chromium
  on ubuntu uses `~/snap/chromium/current/.pki/nssdb` instead). `certutil` is part of
  `libnss3-tools`:

    ```bash
    sudo apt install libnss3-tools
    certutil -d sql:$HOME/.pki/nssdb -A -t "C,," -n ainari-kind-ca -i temporary_files/kind/ainari-kind-ca.crt
    ```

    The browser has to be restarted completely afterwards. `certutil -d sql:$HOME/.pki/nssdb -L`
    lists the CA as `ainari-kind-ca` with the trust `C,,`. Without `certutil`, the CA can also be
    imported by hand in *Settings → Privacy and security → Security → Manage certificates* under
    the authorities, with *Trust this certificate for identifying websites*. To remove it again:

    ```bash
    certutil -d sql:$HOME/.pki/nssdb -D -n ainari-kind-ca
    ```

- The python-sdk uses the certificates of `certifi` and not the ones of the system, so it needs
  the CA explicitly: `REQUESTS_CA_BUNDLE=temporary_files/kind/ainari-kind-ca.crt`.

Deleting `temporary_files/kind` creates a new CA with the next `make up kind`, which then has to
be trusted again.

### Differences to the local setup

- every sakura-host is a pod of the statefulset `sakura` (`sakura-0`, `sakura-1`), which also
  contains the torii in front of that host, because both have to share one network-namespace
- the veth-pair of the uplink is named `veth-kind` on the host and is moved into the pod of
  `torii-public` over the network-namespace of the node
- the pod-network of kind is routed, so the gateways don't share a link. Torii addresses the
  encapsulated packets to the router of the node, which it takes from the route of the kernel
- the node creates its own `/dev/kvm`, so sakura gets the kvm-group of the node, not the one of
  the host
- the components talk https to each other, only the connection between ryokan, sakura and onsen
  is still plain grpc, because the grpc-client has no tls-support yet
- the dashboard is always part of the setup
