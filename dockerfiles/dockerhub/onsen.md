# Ainari - Onsen

Storage-pool of Ainari for the images and snapshots of the virtual machines.

> **IMPORTANT:** Ainari is still a prototype and NOT ready for any productive usage.

## About

Onsen is the storage-pool, where all images and snapshots of Ryokan and Sakura are stored. It has
no REST-API and must not be reachable from the internet. Ryokan and Sakura talk to it over gRPC,
which is secured by a wireguard-tunnel in the kubernetes-setup.

It is one component of [Ainari](https://github.com/kitsudaiki/ainari), a secure and easy to use,
deploy and develop IaaS stack to manage virtual machines.

## Tags

| Tag | Content |
| --- | --- |
| `develop` | latest state of the `develop`-branch |
| `<VERSION>` | a release, without the leading `v`, for example `0.21.0` |

All images are multi-arch for `linux/amd64` and `linux/arm64`.

## Usage

The component reads its config from `/etc/ainari/onsen.toml`, which can be changed with the
environment-variable `CONFIG_FILE`. See the [config-documentation](https://docs.ainari.cloud/deployer/config/onsen_config/) and the
[example-config](https://github.com/kitsudaiki/ainari/blob/develop/example_configs/ainari/onsen.toml).

| Environment-variable | Description |
| --- | --- |
| `CONFIG_FILE` | path of the config-file (default `/etc/ainari/onsen.toml`) |
| `INTERNAL_API_KEY` | shared key for the internal communication between the components |
| `ONSEN_REGISTRATION_KEY` | key, with which the host registers itself at Ryokan |
| `RUST_LOG` | log-level, for example `info` or `debug` |

| Port | Description |
| --- | --- |
| `50051` | gRPC-API for Ryokan and Sakura (address of the config) |

```bash
docker run -d --name onsen \
    -v $(pwd)/onsen.toml:/etc/ainari/onsen.toml:ro \
    -e INTERNAL_API_KEY=... \
    kitsudaiki/onsen:develop
```

Mount a volume to the `[storage].location` of the config, so the stored files survive a restart.
If `/etc/wireguard/wg0.conf` exists, a wireguard-tunnel is started before Onsen.

The usual way to run the whole stack is the
[kubernetes-operator](https://hub.docker.com/r/kitsudaiki/ainari_operator), which deploys and
configures all components.

## Image

The image is built `FROM scratch` with [nix](https://nixos.org): it contains only the pinned
runtime-packages of the component and its binary, no shell-tools for debugging. The list of all
packages is in `/etc/nix-packages.txt` of the image:

```bash
docker run --rm --entrypoint cat kitsudaiki/onsen:develop /etc/nix-packages.txt
```

The container runs as the non-root user `ainari` (uid `1000`).

## Links

- Source: <https://github.com/kitsudaiki/ainari>
- Documentation: <https://docs.ainari.cloud>
- Architecture overview: <https://docs.ainari.cloud/home/overview/>
- Installation guide: <https://docs.ainari.cloud/deployer/installation/kubernetes_installation/>

## License

Apache License 2.0, see [LICENSE](https://github.com/kitsudaiki/ainari/blob/develop/LICENSE).
