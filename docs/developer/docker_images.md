# Packages of the docker-images

The content of the docker-images of `dockerfiles/` is deterministic: every build of an image
installs exactly the same packages in exactly the same versions. The packages don't come from the
package-manager of a distribution, which always installs the latest version of its repository, but
from [nix](https://nixos.org). `dockerfiles/Dockerfile_local_test_tools` is the only exception, because it
is only a toolbox for the local test-environments.

## How it works

- `dockerfiles/nix/flake.nix` and `dockerfiles/nix/packages.nix` define the packages of the images.
  Every image has a runtime-environment `runtime-<IMAGE>` with all packages, which are installed
  in the image, and the images, which compile something, have a development-shell with their
  toolchain (`services`, `torii`, `dashboard` and `docs`).
- `dockerfiles/nix/flake.lock` pins the revisions of [nixpkgs](https://github.com/NixOS/nixpkgs)
  and [rust-overlay](https://github.com/oxalica/rust-overlay) with their hashes. So the versions of
  all packages are fixed, until this file is changed.
- The Dockerfiles build within the image `nixos/nix`, which is pinned by its digest. The binaries
  are compiled with the toolchain of the development-shell and linked against the libraries of the
  nix-store.
- `dockerfiles/nix/make_rootfs.sh` creates the root-filesystem of the image out of the closure of
  its runtime-environment, which is copied into an image `FROM scratch`. So the image contains
  nothing else than these packages and the files of the component. The script fails, if a binary
  references a library, which is not part of the runtime-environment.
- The list of all packages of an image is in `/etc/nix-packages.txt` of the image:

    ```bash
    docker run --rm --entrypoint cat kitsudaiki/miko:develop /etc/nix-packages.txt
    ```

- The dependencies of the rust-components are pinned by `Cargo.lock` and the ones of the dashboard
  by `src/dashboard/app/package-lock.json`, which is installed with `npm ci`.

Packages, which are not taken as they are from nixpkgs, are defined at the beginning of
`dockerfiles/nix/packages.nix` together with the reason, for example the rust-toolchains, the
client-library of MariaDB 3.4 or the version of cloud-hypervisor.

## SBOM of the images

`scripts/generate_sbom.sh` generates an SBOM of every image with
[sbomnix](https://github.com/tiiuae/sbomnix), which is pinned by the flake as well. It lists all
packages of the runtime-environment of the image, which are exactly the packages within the image,
with their versions, licenses, patches, CPEs and purls. The images don't have to be built before
and nix doesn't have to be installed on the host:

```bash
./scripts/generate_sbom.sh                          # all images for the platform of the host
./scripts/generate_sbom.sh --platform linux/arm64   # needs QEMU on an amd64-host
./scripts/generate_sbom.sh miko sakura              # only some images
```

The result is written to `temporary_files/sbom/<PLATFORM>/`: `<IMAGE>.cdx.json` (CycloneDX),
`<IMAGE>.spdx.json` (SPDX), `<IMAGE>.csv` and `versions.csv` with the packages and versions of all
images. The compiled rust-crates and the npm-packages of the dashboard are not part of these
SBOMs, they are pinned by `Cargo.lock` and `package-lock.json`.

## Update the packages

Nix doesn't have to be installed on the host, the commands run within the image of nix:

```bash
docker run --rm -v "$PWD/dockerfiles/nix:/flake" -w /flake nixos/nix:2.35.2 \
    nix --extra-experimental-features "nix-command flakes" flake update
```

This updates nixpkgs to the latest revision of its branch (`nixos-26.05`) and rust-overlay to its
latest revision. To change the branch of nixpkgs, the url of the input in `flake.nix` has to be
changed before. The versions of the rust-toolchains are set within `packages.nix`: the nightly of
the torii has to use the same major version of LLVM as the bpf-linker.

After a change of the flake, the base-image `dockerfiles/Dockerfile_build_base` should get a new
version and has to be pushed, because the CI builds the images of the components on it:

```bash
docker buildx build \
    --platform linux/amd64,linux/arm64 \
    -f dockerfiles/Dockerfile_build_base \
    -t kitsudaiki/ainari_build_base:<NEW_VERSION> \
    --push .
```

The images always use the flake of the repository. If the base-image was built with another one,
the missing packages are downloaded while building the image, so an outdated base-image only makes
the build slower.
