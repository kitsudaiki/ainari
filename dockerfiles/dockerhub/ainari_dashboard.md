# Ainari - Dashboard

Web-dashboard of Ainari to manage virtual machines, images, networks and users in the browser.

> **IMPORTANT:** Ainari is still a prototype and NOT ready for any productive usage.

## About

The dashboard is the web-interface of [Ainari](https://github.com/kitsudaiki/ainari), a secure and
easy to use, deploy and develop IaaS stack to manage virtual machines. It is a Vue-app, which is
served by nginx. The browser talks directly to the APIs of the components.

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
