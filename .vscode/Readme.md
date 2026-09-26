# Local setup in VSCode

The compounds `Ainari Debug` and `Ainari Release` in `launch.json` start the whole stack locally
without any container. Miko, omamori, hanami, ryokan, sakura and onsen run as debug-sessions
(CodeLLDB). Torii requires root, so it can not be debugged and runs instead as single-node gateway
in the task `Run Torii (single-node)`, which is started first and asks for the sudo-password in its
terminal. It keeps running after the debug-sessions were stopped, so stop it by terminating this
task. The uplink stays until the task `remove single-node uplink` is run.

## Required binaries

- rust stable toolchain and the extension CodeLLDB
- rust nightly toolchain with `rust-src` and `bpf-linker` for the eBPF-programs of torii
    ```bash
    rustup toolchain install nightly --component rust-src
    cargo install bpf-linker
    ```
- `ip` and `ethtool` (packages `iproute2`, `ethtool`), used by torii and the uplink-script
- `qemu-img` and `cloud-localds` (packages `qemu-utils`, `cloud-image-utils`), used by sakura
- `cloud-hypervisor` and its firmware `CLOUDHV.fd`
  ([releases](https://github.com/cloud-hypervisor/cloud-hypervisor/releases),
  [firmware](https://github.com/cloud-hypervisor/edk2/releases)). Their paths are set in
  `[hypervisor]` of `sakura.toml`, the defaults are `/usr/local/bin/cloud-hypervisor` and
  `/usr/local/share/CLOUDHV.fd`.
- the user needs access to `/dev/kvm` (group `kvm`)

## Config directory

All services except torii read their config from `/etc/ainari/<service>.toml`, which has to be
writable by the user, because the databases are stored there as well. Templates for all files are
in `example_configs/ainari`:

```bash
sudo mkdir -p /etc/ainari && sudo chown $USER: /etc/ainari
cp example_configs/ainari/{miko,omamori,hanami,ryokan,sakura,onsen}.toml \
   example_configs/ainari/token_key /etc/ainari/
```

Torii reads `example_configs/ainari/torii_single_node.toml` directly from the repository. The
secrets (`INTERNAL_API_KEY`, `SAKURA_REGISTRATION_KEY`, `ONSEN_REGISTRATION_KEY` and the
admin-user `asdf` / `asdfasdf`) are set as env-variables in `launch.json` and `tasks.json`.

## Access to virtual machines

1. cloud-hypervisor is started by sakura as normal user, but has to attach the virtual machine to
   the TAP-device, which torii created as root. For this it needs `CAP_NET_ADMIN` (this has to be
   repeated after every update of the binary):
    ```bash
    sudo setcap cap_net_admin+ep /usr/local/bin/cloud-hypervisor
    ```
2. create a key, an image, a network, a virtual machine and a floating ip-address with
   `ainarictl` like in steps 2-4 of `testing/local_stack/Readme.md`. Hanami has to hand out the
   floating ip-addresses from the subnet of the uplink, so `floating_ip_cidr` in `hanami.toml`
   has to be `10.0.0.0/24`.
3. the floating ip-addresses are served on `uplink0` and are only reachable from the network
   namespace `torii-outside` on the other side of it, not from the host itself
   (see `scripts/setup_single_node_uplink.sh`):
    ```bash
    sudo ip netns exec torii-outside ssh -i ~/.ssh/ainari_local ubuntu@FLOATING_IP
    ```
