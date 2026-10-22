# Ainari - Build Base (Debian)

Debian-based build-image with rustup and the development-packages for the Ainari local setups.

> **IMPORTANT:** Ainari is still a prototype and NOT ready for any productive usage.

## About

This image is not meant to be run, but is the base of the build-stage of the debian-based images
of the rust-components of [Ainari](https://github.com/kitsudaiki/ainari) (miko, hanami, sakura,
ryokan, onsen, omamori, izakaya and omikuji), which are built with
[`dockerfiles/debian_based/Dockerfile_services`](https://github.com/kitsudaiki/ainari/blob/develop/dockerfiles/debian_based/Dockerfile_services).

The debian-based images are only used by the local setups (`make up local` and `make up kind`) and
are never published. They contain the usual tools of the distribution, so they are easier to debug
and to test with. The published images of the components are built with the nix-based variant on
[`kitsudaiki/ainari_build_base_nix`](https://hub.docker.com/r/kitsudaiki/ainari_build_base_nix).

The image is based on `debian:13-slim` (pinned by its digest) and contains:

- the stable rust-toolchain, installed with [rustup](https://rustup.rs) into `/root/.cargo/bin`
- `gcc`, `pkg-config`, `protobuf-compiler`, `git`, `curl` and `ca-certificates`
- the development-packages of openssl, sqlite and MariaDB (`libssl-dev`, `libsqlite3-dev`,
  `libmariadb-dev`)

Debian 13 is used, because the MariaDB client-library of Debian 12 returns wrong results to
diesel. The runtime-images of the components use the same Debian version, so the client-library
at runtime matches the one of the build.

Unlike the nix-based image, the packages of apt and the version of rust are not pinned, but the
latest ones at the time, when the image was built.

## Build

The image is built and pushed manually together with the nix-based base-image with
[`scripts/build_ainari_base.sh`](https://github.com/kitsudaiki/ainari/blob/develop/scripts/build_ainari_base.sh)
from [`dockerfiles/debian_based/Dockerfile_build_base`](https://github.com/kitsudaiki/ainari/blob/develop/dockerfiles/debian_based/Dockerfile_build_base).
See [Build base-image](https://docs.ainari.cloud/developer/build_base_image/) and
[Packages of the docker-images](https://docs.ainari.cloud/developer/docker_images/).

## Links

- Source: <https://github.com/kitsudaiki/ainari>
- Documentation: <https://docs.ainari.cloud>
- Build-guide: <https://docs.ainari.cloud/developer/build_guide/>

## License

Apache License 2.0, see [LICENSE](https://github.com/kitsudaiki/ainari/blob/develop/LICENSE).
