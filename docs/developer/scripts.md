# Scripts

The directory `scripts/` contains the helper-scripts for building, testing and releasing ainari.
The setup-scripts of the local setups are located next to their setup in `testing/`.
All scripts are started from the root of the repository.

| Script | Purpose |
| --- | --- |
| [`build_ainari_base.sh`](#build_ainari_basesh) | builds and pushes the nix- and debian-based base-images for amd64 and arm64 |
| [`build_docker_images.sh`](#build_docker_imagessh) | builds all images with the tag `local_test` and saves them in a tar-file |
| [`collect-api-specs.sh`](#collect-api-specssh) | downloads the openapi-specs of the running components into the docs |
| [`generate_sbom.sh`](#generate_sbomsh) | generates the SBOMs of all docker-images |
| [`update_version.sh`](#update_versionsh) | sets the version of all components |

## build_ainari_base.sh

### Purpose

Builds the base-images with the toolchain of the rust-components, which the images of the
components are built on, and pushes them to Docker Hub (see [Build base-image](build_base_image.md)):

- `kitsudaiki/ainari_build_base_nix` of `dockerfiles/nix_based/Dockerfile_build_base`
- `kitsudaiki/ainari_build_base_debian` of `dockerfiles/debian_based/Dockerfile_build_base`

Both images are built for `linux/amd64` and `linux/arm64` and get the given version as tag. The
platform, which is not the one of the host, is built with the emulation of QEMU. The script runs
all steps of the manual build:

1. registers QEMU for the other platform with `tonistiigi/binfmt` and checks, that QEMU is at least
   version 10.1, which the nix-based build needs
2. creates the buildx-builder `multi-builder` or restarts it, if it exists already, and checks,
   that it supports both platforms
3. logs in to Docker Hub with `docker login`, which asks for the credentials, if they are not
   stored already
4. builds both base-images for both platforms and pushes them

### Usage

```bash
./scripts/build_ainari_base.sh <VERSION>
./scripts/build_ainari_base.sh 0.5.0
```

### Limitations

- The images are always pushed. An image for multiple platforms can't be loaded into the local
  docker, as long as docker doesn't use the containerd image store. See
  [Build base-image](build_base_image.md) to test the images locally without pushing them.
- The tag of the base-images in `Dockerfile_services` of both variants and in the scripts of
  `scripts/` is not changed and has to be updated manually to use the new version.
- An existing tag on Docker Hub is overwritten without asking.
- The registration of QEMU needs a privileged container and is lost with a reboot of the host.
- The nix-based build of the other platform takes some minutes, because the packages, which are not
  in the binary cache of nix, are compiled with the emulation.

## build_docker_images.sh

### Purpose

Builds the images of all components (`hanami`, `miko`, `omamori`, `onsen`, `ryokan`, `sakura`,
`torii` and the dashboard) with the `release`-profile of cargo and the tag `local_test`, for
example `kitsudaiki/miko:local_test`. Afterwards all images are saved together in
`temporary_files/ainari_docker_files.tar`, so they can be copied to another machine and loaded
there with `docker load`. The argument selects, if the images are built from the nix-based
Dockerfiles of `dockerfiles/nix_based`, like the images of the CI, or from the debian-based ones of
`dockerfiles/debian_based` (see [Packages of the docker-images](docker_images.md)). The
base-image of the variant (`kitsudaiki/ainari_build_base_nix` or
`kitsudaiki/ainari_build_base_debian`) is built locally before, so it doesn't have to be pulled
from Docker Hub.

### Usage

```bash
./scripts/build_docker_images.sh nix        # nix-based images
./scripts/build_docker_images.sh debian     # debian-based images
```

On the other machine:

```bash
docker load -i ainari_docker_files.tar
```

### Limitations

- The images are only built for the platform of the host.
- Sakura is built with the default id `993` of the group `kvm`, see
  `testing/kind/build_local_images.sh` for images with the id of the host.
- The tar-file contains all images uncompressed and is correspondingly big.

## collect-api-specs.sh

### Purpose

Downloads the openapi-specs of all components out of a running local docker-compose setup and
writes them to `docs/user/rest_api/open_api_docu_<COMPONENT>.json`, which the rest-api-docu of the
documentation renders. All `operationId`-fields are removed with `jq`, because the secret-scanner
of GitHub reports them as false-positives. A spec is only overwritten, if its download was
successful.

### Usage

Start the docker-compose setup first (see
[Docker-compose setup](local_testing/docker_compose_setup.md)) and then:

```bash
./scripts/collect-api-specs.sh
```

Sakura has no published port and is reached over the address of its gateway on the docker-network.
Another address can be given with `SAKURA_ADDRESS`:

```bash
SAKURA_ADDRESS=172.30.0.20:11420 ./scripts/collect-api-specs.sh
```

### Limitations

- Only works with the docker-compose setup, because it uses plain http on `127.0.0.1` with the
  ports of `docker-compose.yml`. The kind- and vagrant-setup use https on other addresses.
- `curl` and `jq` have to be installed on the host.
- A component, which is not reachable, doesn't stop the script, but its spec is not updated. The
  script lists these components at the end and fails.
- The specs describe the running images, so the images have to be built from the current state
  of the repository, which `setup_local_stack.sh` always does.

## generate_sbom.sh

### Purpose

Generates an SBOM of every docker-image, which is built with nix, with
[sbomnix](https://github.com/tiiuae/sbomnix). It lists all packages of the runtime-environment of
the image, which are exactly the packages within the image, with their versions, licenses,
patches, CPEs and purls. See [Packages of the docker-images](docker_images.md) for how the packages
of the images are pinned.

The SBOMs are generated out of the flake of `dockerfiles/nix_based/nix`, so the images don't have
to be built before. Nix doesn't have to be installed on the host, the script runs within the same
image of nix like the Dockerfiles, and sbomnix itself is pinned by the flake as well.

### Usage

```bash
./scripts/generate_sbom.sh                          # all images for the platform of the host
./scripts/generate_sbom.sh --platform linux/arm64   # another platform
./scripts/generate_sbom.sh miko sakura              # only some images
```

The result is written to `temporary_files/sbom/<PLATFORM>/`:

| File | Content |
| --- | --- |
| `<IMAGE>.cdx.json` | SBOM in the format CycloneDX |
| `<IMAGE>.spdx.json` | SBOM in the format SPDX |
| `<IMAGE>.csv` | all fields of sbomnix as table |
| `versions.csv` | overview of all images with the columns `image`, `package` and `version` |

The nix-store of the container is kept in the docker-volume `ainari-sbom-nix`, so the packages are
only downloaded and built once. It is removed with:

```bash
docker volume rm ainari-sbom-nix
```

### Limitations

- Only the packages of nix are listed. The rust-crates, which are compiled into the binaries of
  the components, and the npm-packages of the dashboard are not part of the SBOMs. They are pinned
  by `Cargo.lock` and `src/dashboard/app/package-lock.json`.
- Only the nix-based images of `dockerfiles/nix_based` have SBOMs. The debian-based images of
  `dockerfiles/debian_based` and `dockerfiles/Dockerfile_local_test_tools` are not built with nix.
  The SBOMs are only valid for the images of the CI and of the vagrant-setup.
- The first run takes some minutes, because the packages, which are not in the binary cache of nix
  (like cloud-hypervisor and the client-library of MariaDB), are built from source.
- Another platform than the one of the host needs QEMU (see
  [Build base-image](build_base_image.md)) and is much slower.
- Some packages have their version only within their name, like `sudo-1.9.17p2`, and an empty
  version-column, because of the way their recipe in nixpkgs sets the name.
- Every run gets a random serial-number, so two SBOMs of the same packages are not identical
  byte by byte.

## update_version.sh

### Purpose

Sets the version of all components in the repository to the same value:

- the rust-workspace (`Cargo.toml`, which all crates inherit, and `Cargo.lock`)
- the python-sdk (`setup.py` and `__init__.py`)
- the cli (`ainarictl`)
- the dashboard (`package.json`)

The script fails, if the version of a file can't be found, so a changed layout of a file doesn't
get lost silently.

### Usage

```bash
./scripts/update_version.sh 0.21.0
./scripts/update_version.sh 0.21.0-rc1
```

A leading `v` is removed and the version has to be a semantic version.

### Limitations

- `Cargo.lock` is only updated, if `cargo` is installed. Otherwise it is updated with the next
  build.
- `src/dashboard/app/package-lock.json`, the `CHANGELOG.md` and the tag of the base-image
  `kitsudaiki/ainari_build_base_nix` and `kitsudaiki/ainari_build_base_debian` are not updated.
- The version in `setup.py` is only the default, the CI overwrites it with the tag.
- The script doesn't commit or tag anything.
