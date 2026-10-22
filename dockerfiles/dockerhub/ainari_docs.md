# Ainari - Documentation

The complete documentation of Ainari as self-hosted website.

> **IMPORTANT:** Ainari is still a prototype and NOT ready for any productive usage.

## About

This image contains the documentation of [Ainari](https://github.com/kitsudaiki/ainari), a secure
and easy to use, deploy and develop IaaS stack to manage virtual machines. It is built with
[zensical](https://zensical.org) and is the same content as <https://docs.ainari.cloud>, so it can
also be read offline or within an isolated network.

## Usage

```bash
docker run -d --name ainari-docs -p 8000:8000 kitsudaiki/ainari_docs:develop
```

## Image

The image is built `FROM scratch` with [nix](https://nixos.org): it contains only python, which
serves the static site with its built-in http-server, and the built documentation. It runs as the
non-root user `ainari` (uid `1000`). The list of all packages is in `/etc/nix-packages.txt` of the
image.

## Links

- Source: <https://github.com/kitsudaiki/ainari>
- Documentation: <https://docs.ainari.cloud>

## License

Apache License 2.0, see [LICENSE](https://github.com/kitsudaiki/ainari/blob/develop/LICENSE).
