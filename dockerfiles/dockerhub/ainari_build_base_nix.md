# Ainari - Build Base (nix)

Build-image with nix and the pinned rust-toolchain, on which the Ainari component-images are built.

> **IMPORTANT:** Ainari is still a prototype and NOT ready for any productive usage.

## About

This image is not meant to be run, but is the base of the build-stage of the images of the
rust-components of [Ainari](https://github.com/kitsudaiki/ainari) (miko, hanami, sakura, ryokan,
onsen, omamori, izakaya and omikuji), which are built with
[`dockerfiles/nix_based/Dockerfile_services`](https://github.com/kitsudaiki/ainari/blob/develop/dockerfiles/nix_based/Dockerfile_services).

It contains [nix](https://nixos.org) with the flake of
[`dockerfiles/nix_based/nix`](https://github.com/kitsudaiki/ainari/tree/develop/dockerfiles/nix_based/nix)
and its development-shell `services` already downloaded: rust, protobuf and the libraries of
openssl, sqlite and MariaDB. All of them are pinned by the `flake.lock` of the flake, so every build
on this image uses exactly the same toolchain. The images of the components don't have to download
the toolchain again, which makes their builds much faster.

## Tags

| Tag | Content |
| --- | --- |
| `<VERSION>` | version of the base-image, for example `0.5.0`, which is increased with every change of its Dockerfile or of the flake |

There is no `develop`-tag. The version, which is used, is set by the `BUILD_BASE_IMAGE` build-arg
of `Dockerfile_services`. All images are multi-arch for `linux/amd64` and `linux/arm64`.

## Usage

```dockerfile
FROM kitsudaiki/ainari_build_base_nix:0.5.0 AS builder

COPY dockerfiles/nix_based/nix /nix-flake
RUN nix develop path:/nix-flake#services --command cargo build --release
```

The toolchain can also be checked directly:

```bash
docker run --rm kitsudaiki/ainari_build_base_nix:0.5.0 \
    nix develop path:/nix-flake#services --command rustc -vV
```

If the flake of the repository differs from the one within the image, the missing packages are
downloaded while building, so an outdated base-image only makes the build slower.

## Build

The image is built and pushed manually with
[`scripts/build_ainari_base.sh`](https://github.com/kitsudaiki/ainari/blob/develop/scripts/build_ainari_base.sh)
from [`dockerfiles/nix_based/Dockerfile_build_base`](https://github.com/kitsudaiki/ainari/blob/develop/dockerfiles/nix_based/Dockerfile_build_base).
See [Build base-image](https://docs.ainari.cloud/developer/build_base_image/) and
[Packages of the docker-images](https://docs.ainari.cloud/developer/docker_images/).

## Links

- Source: <https://github.com/kitsudaiki/ainari>
- Documentation: <https://docs.ainari.cloud>
- Build-guide: <https://docs.ainari.cloud/developer/build_guide/>

## License

Apache License 2.0, see [LICENSE](https://github.com/kitsudaiki/ainari/blob/develop/LICENSE).
