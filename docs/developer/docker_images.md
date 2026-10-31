# Packages of the docker-images

There are two variants of the Dockerfiles of the components, which build the same images:

| Variant | Directory | Used by |
| --- | --- | --- |
| nix-based | `dockerfiles/nix_based` | the CI (all published images), `make up vagrant` |
| debian-based | `dockerfiles/debian_based` | `make up local` (`docker-compose.yml`), `make up kind` |

- **nix-based**
    - deterministic: every build installs exactly the same packages in exactly the same versions,
      taken from [nix](https://nixos.org) instead of the package-manager of a distribution
    - contains nothing else than these packages and the files of the component, so there are no
      shell-tools for debugging
- **debian-based**
    - built on Debian and Ubuntu with the unpinned packages of apt
    - contains the usual tools of the distribution, so it is easier to debug and to test with
    - only meant for the local setups and never published

Notes:

- Both variants share the start-scripts of `dockerfiles/files`.
- Changes of the build or of the runtime-dependencies of a component have to be made in both
  variants.
- `dockerfiles/Dockerfile_local_test_tools` belongs to none of them, it is only a toolbox for the
  local test-environments.

The rest of this page describes the nix-based variant.

## How it works

- **Package-definitions**: `dockerfiles/nix_based/nix/flake.nix` and `packages.nix` define the
  packages of the images.
    - Every image has a runtime-environment `runtime-<IMAGE>` with all packages of the image.
    - The images, which compile something (`services`, `torii`, `dashboard`, `docs` and
      `operator`), additionally have a development-shell with their toolchain.
    - Packages, which are not taken as they are from nixpkgs, are defined at the beginning of
      `packages.nix` together with the reason, for example the rust-toolchains, the client-library
      of MariaDB 3.4 or the version of cloud-hypervisor.
- **Pinning**: `dockerfiles/nix_based/nix/flake.lock` pins the revisions of
  [nixpkgs](https://github.com/NixOS/nixpkgs) and
  [rust-overlay](https://github.com/oxalica/rust-overlay) with their hashes, so all package-versions
  are fixed until this file changes.
- **Build**: the Dockerfiles build within the image `nixos/nix`, pinned by its digest. The binaries
  are compiled with the toolchain of the development-shell and linked against the nix-store.
- **Root-filesystem**: `dockerfiles/nix_based/nix/make_rootfs.sh` creates it out of the closure of
  the runtime-environment and copies it into an image `FROM scratch`. It fails, if a binary
  references a library, which is not part of the runtime-environment.
- **Package-list**: all packages of an image are listed in `/etc/nix-packages.txt` of the image:

    ```bash
    docker run --rm --entrypoint cat kitsudaiki/miko:develop /etc/nix-packages.txt
    ```

- **Language-dependencies** are pinned separately:
    - rust-components: `Cargo.lock`
    - dashboard: `src/dashboard/app/package-lock.json` (installed with `npm ci`)
    - kubernetes-operator: `deploy/operator/go.sum`
- **Operator**: `dockerfiles/nix_based/Dockerfile_operator` only exists as nix-based image. It is
  also used by `make up kind`, which builds the other images with the debian-based Dockerfiles.

## SBOM of the images

`scripts/generate_sbom.sh` generates an SBOM of every image with
[sbomnix](https://github.com/tiiuae/sbomnix), which is pinned by the flake as well.

- It lists all packages of the runtime-environment, which are exactly the packages within the
  image, with their versions, licenses, patches, CPEs and purls.
- The images don't have to be built before and nix doesn't have to be installed on the host.

```bash
./scripts/generate_sbom.sh                          # all images for the platform of the host
./scripts/generate_sbom.sh --platform linux/arm64   # needs QEMU on an amd64-host
./scripts/generate_sbom.sh miko sakura              # only some images
```

The result is written to `temporary_files/sbom/<PLATFORM>/`:

- `<IMAGE>.cdx.json` (CycloneDX)
- `<IMAGE>.spdx.json` (SPDX)
- `<IMAGE>.csv`
- `versions.csv` with the packages and versions of all images

The compiled rust-crates, the npm-packages and the go-modules are not part of these SBOMs, they are
pinned by `Cargo.lock`, `package-lock.json` and `deploy/operator/go.sum`.

## Update the packages

Nix doesn't have to be installed on the host, the commands run within the image of nix:

```bash
docker run --rm -v "$PWD/dockerfiles/nix_based/nix:/flake" -w /flake nixos/nix:2.35.2 \
    nix --extra-experimental-features "nix-command flakes" flake update
```

- This updates nixpkgs to the latest revision of its branch (`nixos-26.05`) and rust-overlay to its
  latest revision.
- To change the branch of nixpkgs, change the url of the input in `flake.nix` before.
- The versions of the rust-toolchains are set within `packages.nix`. The nightly of torii has to use
  the same major version of LLVM as the bpf-linker.

After a change of the flake:

- The base-image `dockerfiles/nix_based/Dockerfile_build_base` should get a new version and has to
  be pushed, because the CI builds the images of the components on it.
- The debian-based base-image gets the same version, see [Build base-image](build_base_image.md).
- The images always use the flake of the repository. If the base-image was built with another one,
  the missing packages are downloaded while building, so an outdated base-image only makes the
  build slower.
