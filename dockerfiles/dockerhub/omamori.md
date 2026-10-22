# Ainari - Omamori

Key-manager of Ainari: upload, generation and download of keys.

> **IMPORTANT:** Ainari is still a prototype and NOT ready for any productive usage.

## About

Omamori is the protection-component of the stack and basically a key-manager. It provides the
functionality to upload, generate and download keys, for example the keys, with which Ryokan
encrypts the images of the virtual machines. Currently the keys are encrypted with a key of the
config and stored in its database.

It is one component of [Ainari](https://github.com/kitsudaiki/ainari), a secure and easy to use,
deploy and develop IaaS stack to manage virtual machines.

## Tags

| Tag | Content |
| --- | --- |
| `develop` | latest state of the `develop`-branch |
| `<VERSION>` | a release, without the leading `v`, for example `0.21.0` |

All images are multi-arch for `linux/amd64` and `linux/arm64`.

## Usage

The component reads its config from `/etc/ainari/omamori.toml`, which can be changed with the
environment-variable `CONFIG_FILE`. See the [config-documentation](https://docs.ainari.cloud/deployer/config/omamori_config/) and the
[example-config](https://github.com/kitsudaiki/ainari/blob/develop/example_configs/ainari/omamori.toml).

| Environment-variable | Description |
| --- | --- |
| `CONFIG_FILE` | path of the config-file (default `/etc/ainari/omamori.toml`) |
| `INTERNAL_API_KEY` | shared key for the internal communication between the components |
| `AINARI_MYSQL_PASSWORD` | password of the database-user, if `database_type` is `"mysql"` |
| `RUST_LOG` | log-level, for example `info` or `debug` |

| Port | Description |
| --- | --- |
| `11421` | public REST-API |
| `10421` | internal API of the components |

```bash
docker run -d --name omamori \
    -v $(pwd)/omamori.toml:/etc/ainari/omamori.toml:ro \
    -e INTERNAL_API_KEY=... \
    kitsudaiki/omamori:develop
```

The usual way to run the whole stack is the
[kubernetes-operator](https://hub.docker.com/r/kitsudaiki/ainari_operator), which deploys and
configures all components.

## Image

The image is built `FROM scratch` with [nix](https://nixos.org): it contains only the pinned
runtime-packages of the component and its binary, no shell-tools for debugging. The list of all
packages is in `/etc/nix-packages.txt` of the image:

```bash
docker run --rm --entrypoint cat kitsudaiki/omamori:develop /etc/nix-packages.txt
```

The container runs as the non-root user `ainari` (uid `1000`).

## Links

- Source: <https://github.com/kitsudaiki/ainari>
- Documentation: <https://docs.ainari.cloud>
- Architecture overview: <https://docs.ainari.cloud/home/overview/>
- Installation guide: <https://docs.ainari.cloud/deployer/installation/kubernetes_installation/>

## License

Apache License 2.0, see [LICENSE](https://github.com/kitsudaiki/ainari/blob/develop/LICENSE).
