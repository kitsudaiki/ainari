# Ainari - Sakura

Compute-host of Ainari, which runs the virtual machines with cloud-hypervisor.

> **IMPORTANT:** Ainari is still a prototype and NOT ready for any productive usage.

## About

Sakura is the compute-host of the stack. It manages the virtual machines, which run with
[cloud-hypervisor](https://www.cloudhypervisor.org), and is itself managed by Hanami. The users
access the control-endpoints of their virtual machines end-to-end encrypted over the proxy-ports
of the Torii-gateway.

It is one component of [Ainari](https://github.com/kitsudaiki/ainari), a secure and easy to use,
deploy and develop IaaS stack to manage virtual machines.

## Tags

| Tag | Content |
| --- | --- |
| `develop` | latest state of the `develop`-branch |
| `<VERSION>` | a release, without the leading `v`, for example `0.21.0` |

All images are multi-arch for `linux/amd64` and `linux/arm64`.

## Usage

The component reads its config from `/etc/ainari/sakura.toml`, which can be changed with the
environment-variable `CONFIG_FILE`. See the [config-documentation](https://docs.ainari.cloud/deployer/config/sakura_config/) and the
[example-config](https://github.com/kitsudaiki/ainari/blob/develop/example_configs/ainari/sakura.toml).

| Environment-variable | Description |
| --- | --- |
| `CONFIG_FILE` | path of the config-file (default `/etc/ainari/sakura.toml`) |
| `INTERNAL_API_KEY` | shared key for the internal communication between the components |
| `SAKURA_REGISTRATION_KEY` | key, with which the host registers itself at Hanami |
| `RUST_LOG` | log-level, for example `info` or `debug` |

| Port | Description |
| --- | --- |
| `11420` | public API, reachable over the proxy-ports of the Torii |
| `10420` | internal API, which Hanami uses |

```bash
docker run -d --name sakura \
    --network container:torii-vmm \
    --device /dev/kvm --device /dev/net/tun \
    --group-add $(stat -c '%g' /dev/kvm) \
    --cap-add NET_ADMIN \
    -v $(pwd)/sakura.toml:/etc/ainari/sakura.toml:ro \
    -e INTERNAL_API_KEY=... \
    -e SAKURA_REGISTRATION_KEY=... \
    kitsudaiki/sakura:develop
```

Requirements of the container:

- `/dev/kvm` and `/dev/net/tun` as devices, and the group of `/dev/kvm` of the host
  (`--group-add`)
- the capability `NET_ADMIN` (`--cap-add NET_ADMIN`). It is only used by the hypervisor-binary
  itself, which carries it as file-capability, so Sakura doesn't run as root
- the network-namespace of the Torii in front of the host, which owns the TAP-devices of the
  virtual machines (`--network container:<torii>`)
- if `/etc/wireguard/wg0.conf` exists, a wireguard-tunnel is started before Sakura

The usual way to run the whole stack is the
[kubernetes-operator](https://hub.docker.com/r/kitsudaiki/ainari_operator), which deploys and
configures all components.

## Image

The image is built `FROM scratch` with [nix](https://nixos.org): it contains only the pinned
runtime-packages of the component and its binary, no shell-tools for debugging. The list of all
packages is in `/etc/nix-packages.txt` of the image:

```bash
docker run --rm --entrypoint cat kitsudaiki/sakura:develop /etc/nix-packages.txt
```

The container runs as the non-root user `ainari` (uid `1000`).

## Links

- Source: <https://github.com/kitsudaiki/ainari>
- Documentation: <https://docs.ainari.cloud>
- Architecture overview: <https://docs.ainari.cloud/home/overview/>
- Installation guide: <https://docs.ainari.cloud/deployer/installation/kubernetes_installation/>

## License

Apache License 2.0, see [LICENSE](https://github.com/kitsudaiki/ainari/blob/develop/LICENSE).
