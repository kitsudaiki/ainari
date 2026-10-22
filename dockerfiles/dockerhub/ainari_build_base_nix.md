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
