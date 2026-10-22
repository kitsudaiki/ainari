# Ainari - Torii

eBPF-based network-gateway of Ainari for the networks, floating IPs and API-proxy of the VMs.

> **IMPORTANT:** Ainari is still a prototype and NOT ready for any productive usage.

## About

Torii is the gateway-component of the stack. It is configured by Hanami and handles two pathways:

1. **API-proxy:** a layer-3-proxy with one port per virtual machine, over which the user talks
   directly to the Sakura-host of the virtual machine. Torii doesn't terminate the
   HTTPS-connection, so it stays end-to-end encrypted between the user and the Sakura-host.
2. **Networks:** the networks between the virtual machines and from the virtual machines to the
   internet, including the floating IPs. The datapath runs as eBPF-programs.

The same image is used for every gateway, the role is only a question of the config: one public
Torii at the edge of the network with an uplink, and one Torii in front of every Sakura-host,
which owns the TAP-devices of the virtual machines of that host.

It is one component of [Ainari](https://github.com/kitsudaiki/ainari), a secure and easy to use,
deploy and develop IaaS stack to manage virtual machines.

## Tags

| Tag | Content |
| --- | --- |
| `develop` | latest state of the `develop`-branch |
| `<VERSION>` | a release, without the leading `v`, for example `0.21.0` |

All images are multi-arch for `linux/amd64` and `linux/arm64`.

## Usage

The component reads its config from `/etc/ainari/torii.toml`, which can be changed with the
environment-variable `CONFIG_FILE`. See the
[config-documentation](https://docs.ainari.cloud/deployer/config/torii_config/) and the example-configs
for the [public gateway](https://github.com/kitsudaiki/ainari/blob/develop/example_configs/ainari/torii_public.toml)
and the [gateway in front of a host](https://github.com/kitsudaiki/ainari/blob/develop/example_configs/ainari/torii.toml).

| Environment-variable | Description |
| --- | --- |
| `CONFIG_FILE` | path of the config-file (default `/etc/ainari/torii.toml`) |
| `INTERNAL_API_KEY` | shared key for the internal communication between the components |
| `WAIT_FOR_IFACE` | optional: interface of the uplink, on which the gateway waits before it starts |
| `UPLINK_NEXT_HOP` | optional: address of the next hop of the uplink, which is resolved before the start |
| `RUST_LOG` | log-level, for example `info` or `debug` |

| Port | Description |
| --- | --- |
| `11419` | public REST-API |
| `10419` | internal API of the components |
| `10042-10053` | proxy-ports towards the Sakura-hosts (range of the config) |

```bash
docker run -d --name torii --privileged \
    --device /dev/net/tun \
    -v $(pwd)/torii.toml:/etc/ainari/torii.toml:ro \
    -v /sys/kernel/debug:/sys/kernel/debug:ro \
    -e INTERNAL_API_KEY=... \
    kitsudaiki/torii:develop
```

The container has to be privileged, because it attaches eBPF-programs to its interfaces, and the
kernel of the host needs eBPF-support. Checksum-offloading is disabled on `eth0` at start.

The usual way to run the whole stack is the
[kubernetes-operator](https://hub.docker.com/r/kitsudaiki/ainari_operator), which deploys and
configures all components.

## Image

The image is built `FROM scratch` with [nix](https://nixos.org): it contains only the pinned
runtime-packages of the component and its binary, no shell-tools for debugging. The list of all
packages is in `/etc/nix-packages.txt` of the image:

```bash
docker run --rm --entrypoint cat kitsudaiki/torii:develop /etc/nix-packages.txt
```

## Links

- Source: <https://github.com/kitsudaiki/ainari>
- Documentation: <https://docs.ainari.cloud>
- Architecture overview: <https://docs.ainari.cloud/home/overview/>
- Installation guide: <https://docs.ainari.cloud/deployer/installation/kubernetes_installation/>

## License

Apache License 2.0, see [LICENSE](https://github.com/kitsudaiki/ainari/blob/develop/LICENSE).
