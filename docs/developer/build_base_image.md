# Build base-image

The images of the rust-components are built on a base-image with their toolchain. There is one
base-image for each variant of the Dockerfiles (see
[Packages of the docker-images](docker_images.md)):

| Image | Dockerfile | Content |
| --- | --- | --- |
| `kitsudaiki/ainari_build_base_nix` | `dockerfiles/nix_based/Dockerfile_build_base` | nix and the toolchain of the flake of `dockerfiles/nix_based/nix` |
| `kitsudaiki/ainari_build_base_debian` | `dockerfiles/debian_based/Dockerfile_build_base` | Debian 13 with rustup and the development-packages of apt |

The CI only builds the nix-based images, but doesn't build the base-image itself. It pulls
`kitsudaiki/ainari_build_base_nix` from Docker Hub. Both base-images have to be built and pushed
manually for `linux/amd64` and `linux/arm64`, whenever one of their Dockerfiles or the flake of
`dockerfiles/nix_based/nix` changes. Both images always get the same tag, which is used by the
`Dockerfile_services` of both variants and the scripts of `scripts/`, and which is increased with
every new version.

The script `scripts/build_ainari_base.sh` runs all of the following steps and builds and pushes
both base-images for both platforms with the given version (see
[Scripts](scripts.md#build_ainari_basesh)):

```bash
./scripts/build_ainari_base.sh 0.5.0
```

## Manual steps

The platform, which is not the one of the host, is built with the emulation of QEMU.

- Enable QEMU for arm64 (or amd64 on an arm64-host)

    ```bash
    docker pull tonistiigi/binfmt:latest
    docker run --privileged --rm tonistiigi/binfmt --uninstall qemu-aarch64
    docker run --privileged --rm tonistiigi/binfmt --install arm64
    docker run --privileged --rm tonistiigi/binfmt --version
    ```

    QEMU has to be at least version 10.1. Older versions don't support the ioctl `TCGETS2`, which
    the glibc of nixpkgs uses, and the build fails with `getting pseudoterminal attributes:
    Inappropriate ioctl for device`. The old registration is removed before, because it keeps
    using the binary of the old QEMU. The registration is lost with a reboot of the host.

- Create a buildx-builder, which can build for multiple platforms (only once)

    ```bash
    docker buildx create --name multi-builder --use
    ```

    If it exists already, it has to be restarted after a change of QEMU, because it detects the
    platforms only at its start:

    ```bash
    docker buildx stop multi-builder
    ```

- Verify, that `linux/arm64` and `linux/amd64` are listed

    ```bash
    docker buildx inspect multi-builder --bootstrap | grep Platforms
    ```

- Login to Docker Hub

    ```bash
    docker login
    ```

- Build both new base-images for both platforms and push them

    ```bash
    for base in nix debian; do
        docker buildx build \
            --builder multi-builder \
            --platform linux/amd64,linux/arm64 \
            -f "dockerfiles/${base}_based/Dockerfile_build_base" \
            -t "kitsudaiki/ainari_build_base_${base}:0.5.0" \
            --push .
    done
    ```

!!! info

    An image for multiple platforms can only be pushed into a registry, but not be loaded into the
    local docker, as long as docker doesn't use the containerd image store. To test the images
    locally without pushing them, every platform is built and loaded on its own:

    ```bash
    for base in nix debian; do
        for arch in amd64 arm64; do
            docker buildx build --builder multi-builder --platform "linux/$arch" --load \
                -f "dockerfiles/${base}_based/Dockerfile_build_base" \
                -t "kitsudaiki/ainari_build_base_${base}:0.5.0-$arch" .
        done
    done
    docker run --rm --platform linux/arm64 kitsudaiki/ainari_build_base_nix:0.5.0-arm64 \
        nix develop path:/nix-flake#services --command rustc -vV
    docker run --rm --platform linux/arm64 kitsudaiki/ainari_build_base_debian:0.5.0-arm64 \
        /root/.cargo/bin/rustc -vV
    ```

The nix-based build of the other platform takes some minutes, because the packages, which are not
in the binary cache of nix, like the client-library of MariaDB, are compiled with the emulation.

!!! info

    The local setups don't need the base-images from Docker Hub: `make up local`, `make up kind`
    and `make up vagrant` build the base-image of their variant locally with the tag, which its
    `Dockerfile_services` uses by default.
