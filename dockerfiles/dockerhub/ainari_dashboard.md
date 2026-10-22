# Ainari - Dashboard

Web-dashboard of Ainari to manage virtual machines, images, networks and users in the browser.

> **IMPORTANT:** Ainari is still a prototype and NOT ready for any productive usage.

## About

The dashboard is the web-interface of [Ainari](https://github.com/kitsudaiki/ainari), a secure and
easy to use, deploy and develop IaaS stack to manage virtual machines. It is a Vue-app, which is
served by nginx. The browser talks directly to the APIs of the components.

## Tags

| Tag | Content |
| --- | --- |
| `develop` | latest state of the `develop`-branch |
| `<VERSION>` | a release, without the leading `v`, for example `0.21.0` |

All images are multi-arch for `linux/amd64` and `linux/arm64`.

## Usage

The dashboard loads its config at startup from `/config.json` of its own web-server, so the file
has to be mounted to `/usr/share/nginx/html/config.json`:

```json
{
    "apiUrl": "https://miko.example.com"
}
```

| Option | Description |
| --- | --- |
| `apiUrl` | public address of Miko. The browser talks to it directly, so it has to be reachable from the machine of the user. |

```bash
docker run -d --name ainari-dashboard -p 8080:80 \
    -v $(pwd)/dashboard_config.json:/usr/share/nginx/html/config.json:ro \
    kitsudaiki/ainari_dashboard:develop
```

The dashboard is then available on <http://localhost:8080>. The web-server listens on port `80`.

See the [dashboard-documentation](https://docs.ainari.cloud/user/dashboard/dashboard/) and the
[config-documentation](https://docs.ainari.cloud/deployer/config/dashboard_config/). In the
kubernetes-deployment the config is created by the
[kubernetes-operator](https://hub.docker.com/r/kitsudaiki/ainari_operator).

## Image

The image is built `FROM scratch` with [nix](https://nixos.org): it contains only nginx, its
pinned runtime-packages and the built app, no shell-tools for debugging. The list of all packages
is in `/etc/nix-packages.txt` of the image:

```bash
docker run --rm --entrypoint cat kitsudaiki/ainari_dashboard:develop /etc/nix-packages.txt
```

## Links

- Source: <https://github.com/kitsudaiki/ainari>
- Documentation: <https://docs.ainari.cloud>
- Installation guide: <https://docs.ainari.cloud/deployer/installation/kubernetes_installation/>

## License

Apache License 2.0, see [LICENSE](https://github.com/kitsudaiki/ainari/blob/develop/LICENSE).
