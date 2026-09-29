# Scripts

The directory `scripts/` contains the helper-scripts for building, testing and releasing ainari.
All scripts are started from the root of the repository.

| Script | Purpose |
| --- | --- |
| [`build_docker_images.sh`](#build_docker_imagessh) | builds all images with the tag `local_test` and saves them in a tar-file |
| [`build_local_images.sh`](#build_local_imagessh) | builds all images with the tag `local` for the kind- and vagrant-setup |
| [`collect-api-specs.sh`](#collect-api-specssh) | downloads the openapi-specs of the running components into the docs |
| [`create_local_ca.sh`](#create_local_cash) | creates the CA of a local setup |
| [`generate_sbom.sh`](#generate_sbomsh) | generates the SBOMs of all docker-images |
| [`setup_kind_stack.sh`](#setup_kind_stacksh) | starts the whole stack on a kind-cluster |
| [`setup_local_stack.sh`](#setup_local_stacksh) | starts the whole stack with docker compose |
| [`setup_single_node_uplink.sh`](#setup_single_node_uplinksh) | creates the uplink of a torii, which runs directly on the host |
| [`setup_vagrant_stack.sh`](#setup_vagrant_stacksh) | starts the whole stack on eight virtual machines |
| [`update_version.sh`](#update_versionsh) | sets the version of all components |

## build_docker_images.sh

### Purpose

Builds the images of all components (`hanami`, `miko`, `omamori`, `onsen`, `ryokan`, `sakura`,
`torii` and the dashboard) with the `release`-profile of cargo and the tag `local_test`, for
example `kitsudaiki/miko:local_test`. Afterwards all images are saved together in
`temporary_files/ainari_docker_files.tar`, so they can be copied to another machine and loaded
there with `docker load`. The base-image `kitsudaiki/ainari_build_base` is built locally before, so
it doesn't have to be pulled from Docker Hub.

### Usage

```bash
./scripts/build_docker_images.sh
```

On the other machine:

```bash
docker load -i ainari_docker_files.tar
```

### Limitations

- The script doesn't stop at a failed build: the following images are still built and the
  tar-file may contain an older image with the same tag or `docker save` fails.
- The images are only built for the platform of the host.
- Sakura is built with the default id `993` of the group `kvm`, see
  [build_local_images.sh](#build_local_imagessh) for images with the id of the host.
- The tar-file contains all images uncompressed and is correspondingly big.

## build_local_images.sh

### Purpose

Builds the images of all components for the kubernetes-based local setups (kind and vagrant) with
the tag `local`, for example `ainari/miko:local`. They are the same images as the ones of the
docker-compose setup, but built without docker compose and with the faster `local`-profile of
cargo. [setup_kind_stack.sh](#setup_kind_stacksh) and
[setup_vagrant_stack.sh](#setup_vagrant_stacksh) call this script before every start, so it only
has to be called directly to rebuild the images without restarting the setup.

### Usage

```bash
./scripts/build_local_images.sh
```

The id of the group of `/dev/kvm`, which sakura is built with, can be given with `KVM_GID`. It
defaults to the one of the host:

```bash
KVM_GID=108 ./scripts/build_local_images.sh
```

### Limitations

- The `local`-profile is optimized for the build-time and not for the runtime, so the images are
  only meant for testing and never for a real deployment.
- The images are only built for the platform of the host.
- The images are not pushed anywhere, the setups load them into their cluster or virtual machines
  themselves.

## collect-api-specs.sh

### Purpose

Downloads the openapi-specs of all components out of a running local docker-compose setup and
writes them to `docs/user/rest_api/open_api_docu_<COMPONENT>.json`, which the rest-api-docu of the
documentation renders. All `operationId`-fields are removed with `jq`, because the secret-scanner
of GitHub reports them as false-positives. A spec is only overwritten, if its download was
successful.

### Usage

Start the docker-compose setup first (see [setup_local_stack.sh](#setup_local_stacksh)) and then:

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
- The script stops at the first component, which is not reachable, so the specs of the following
  components are not updated.
- The specs describe the running images, so the images have to be built from the current state
  of the repository, which `setup_local_stack.sh` always does.

## create_local_ca.sh

### Purpose

Creates the CA of a local setup, which signs the certificates of all components of the setup, if
it doesn't exist yet. The CA stays the same over all runs, so it only has to be added to the
trust-store of the host once (see
[The CA of the kind- and the vagrant-setup](local_testing/https_ca.md)).
Its name-constraints limit it to the names of the setup, so it can't be misused for any other
host, even if its key leaks. [setup_kind_stack.sh](#setup_kind_stacksh) and
[setup_vagrant_stack.sh](#setup_vagrant_stacksh) call this script, so it is normally not called
directly.

### Usage

```bash
./scripts/create_local_ca.sh <CERT_FILE> <KEY_FILE> <COMMON_NAME> <PERMITTED_NAMES>
```

!!! example

    ```bash
    ./scripts/create_local_ca.sh temporary_files/kind/ainari-kind-ca.crt \
        temporary_files/kind/ainari-kind-ca.key \
        "ainari kind-setup CA" \
        "permitted;IP:127.0.0.1/255.255.255.255,permitted;DNS:localhost,permitted;DNS:cluster.local"
    ```

### Limitations

- An existing CA is never changed: if the certificate and the key already exist, the script does
  nothing, even if the common-name or the name-constraints are different. To create a new CA, the
  two files have to be deleted before.
- The key is stored unencrypted. It is only readable by its owner, but should never be used
  outside of a local setup.
- The CA is valid for 10 years and has no revocation.
- `openssl` has to be installed on the host.

## generate_sbom.sh

### Purpose

Generates an SBOM of every docker-image, which is built with nix, with
[sbomnix](https://github.com/tiiuae/sbomnix). It lists all packages of the runtime-environment of
the image, which are exactly the packages within the image, with their versions, licenses,
patches, CPEs and purls. See [Packages of the docker-images](docker_images.md) for how the packages
of the images are pinned.

The SBOMs are generated out of the flake of `dockerfiles/nix`, so the images don't have to be built
before. Nix doesn't have to be installed on the host, the script runs within the same image of nix
like the Dockerfiles, and sbomnix itself is pinned by the flake as well.

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
- `dockerfiles/Dockerfile_local_test_tools` is not built with nix and has no SBOM.
- The first run takes some minutes, because the packages, which are not in the binary cache of nix
  (like cloud-hypervisor and the client-library of MariaDB), are built from source.
- Another platform than the one of the host needs QEMU (see
  [Build base-image](build_base_image.md)) and is much slower.
- Some packages have their version only within their name, like `sudo-1.9.17p2`, and an empty
  version-column, because of the way their recipe in nixpkgs sets the name.
- Every run gets a random serial-number, so two SBOMs of the same packages are not identical
  byte by byte.

## setup_kind_stack.sh

### Purpose

Starts the same setup as [setup_local_stack.sh](#setup_local_stacksh), but on a kind-cluster
(kubernetes in docker) with the helm-chart of `deploy/k8s/ainari`, and connects the host to it. The
images are built with [build_local_images.sh](#build_local_imagessh) and loaded into the cluster.
The components talk https to each other with certificates of cert-manager, which are signed by the
CA of [create_local_ca.sh](#create_local_cash). See [Kind setup](local_testing/kind_setup.md) for
the details of the setup.

### Usage

```bash
make up kind      # or ./scripts/setup_kind_stack.sh
make down kind    # or ./scripts/setup_kind_stack.sh --down
```

The binary of kind can be given with `KIND`. `make up kind` downloads kind into
`temporary_files/bin`, if it is not installed.

### Limitations

- `docker`, `kind`, `kubectl`, `helm` and `openssl` have to be installed and the host needs
  `/dev/kvm` and a kernel with eBPF/XDP-support.
- The network-setup needs root, so the script asks for the password of sudo.
- Every start deletes the previous deployment with its databases, so no state is kept between two
  runs.
- The floating ip-addresses `10.0.0.0/24` must not be used by any other interface of the host, so
  the setup can't run together with the docker-compose setup, the vagrant-setup or the uplink of
  [setup_single_node_uplink.sh](#setup_single_node_uplinksh).
- The manifest of cert-manager is downloaded from GitHub, so the host needs access to the internet.
- `--down` removes the NAT-rules, but `net.ipv4.ip_forward` stays enabled on the host.
- The virtual machines only reach the internet over the first default-route of the host.

## setup_local_stack.sh

### Purpose

Starts the local docker-compose setup of `docker-compose.yml` with all components and two
sakura-hosts and connects the host to it. The images are always rebuilt before, so the setup never
runs an older version than the one of the working tree. The script injects a veth-pair into the
gateway at the edge (`torii-public`), so the host reaches the floating ip-addresses of the virtual
machines, and lets the host forward and masquerade the traffic of the virtual machines towards the
internet. See [Docker-compose setup](local_testing/docker_compose_setup.md) for the details of the
setup.

### Usage

```bash
make up local                             # or sudo ./scripts/setup_local_stack.sh
make down local                           # or sudo ./scripts/setup_local_stack.sh --down
python3 testing/ainari_test/vm_lifecycle_test.py
```

### Limitations

- The whole script needs root.
- The host needs docker with compose, `/dev/kvm` and a kernel with eBPF/XDP-support.
- Every start removes the previous containers, so no state is kept between two runs.
- The floating ip-addresses `10.0.0.0/24` must not be used by any other interface of the host, so
  the setup can't run together with the kind-setup, the vagrant-setup or the uplink of
  [setup_single_node_uplink.sh](#setup_single_node_uplinksh).
- `--down` doesn't remove the NAT- and forward-rules of iptables and `net.ipv4.ip_forward` stays
  enabled. The rules are replaced with the next start.
- The virtual machines only reach the internet over the first default-route of the host.
- The api is only reachable over plain http.

## setup_single_node_uplink.sh

### Purpose

Creates the uplink for a torii, which runs directly on the host for development (see
`example_configs/ainari/torii_single_node.toml` and [Development](development.md)). The "outside"
is a network-namespace `torii-outside`, which is connected to the host by a veth-pair:

```text
[ netns torii-outside ]  outside0 10.0.0.1/24    (uplink_next_hop)
           |
[ host, torii ]          uplink0  10.0.0.254/24  (uplink_iface)
```

The floating ip-addresses (`10.0.0.2` - `10.0.0.253`) are reachable from within the namespace.
Checksum-offloading is switched off on both ends, because the gateway rewrites the addresses with
incremental checksum-updates.

### Usage

```bash
sudo ./scripts/setup_single_node_uplink.sh         # create it (replaces an existing one)
sudo ./scripts/setup_single_node_uplink.sh down    # remove it again
sudo ip netns exec torii-outside ssh ubuntu@10.0.0.2
```

### Limitations

- Needs root and `ethtool`.
- The addresses are fixed within the script and collide with the floating ip-addresses of the
  local setups, so it can't be used together with them. The setups refuse to start, while
  `uplink0` exists.
- The virtual machines are only reachable from within the namespace and have no internet.
- Only for local development: on a real host the uplink is the physical network-interface and the
  next hop the router behind it.

## setup_vagrant_stack.sh

### Purpose

Starts the setup of `testing/vagrant`: eight virtual machines with nested virtualization and a
kubernetes-cluster (k3s), on which ansible deploys the helm-chart of `deploy/k8s/ainari`. Miko,
hanami, ryokan and omamori run with one replica on each of the three management-machines and share
the mysql-server on the machine `ainari-mysql`. The images are built with
[build_local_images.sh](#build_local_imagessh) on the host and copied into the virtual machines.
The host reaches the floating ip-addresses over a route towards the virtual machine `ainari-torii`.
See [Vagrant setup](local_testing/vagrant_setup.md) for the details of the setup.

### Usage

```bash
make up vagrant      # or ./scripts/setup_vagrant_stack.sh
make down vagrant    # or ./scripts/setup_vagrant_stack.sh --down
```

`make up vagrant` installs ansible into a virtual environment in `temporary_files`, if it is not
installed.

### Limitations

- `docker`, `vagrant` with the plugin `vagrant-libvirt`, libvirt, `ansible-playbook` and `openssl`
  have to be installed and the host needs nested virtualization.
- The eight virtual machines need about 32 GiB free memory and 40 GiB free disk on the host, see
  [Vagrant setup](local_testing/vagrant_setup.md).
- Adding the route needs root, so the script asks for the password of sudo.
- The floating ip-addresses `10.0.0.0/24` and the private network `192.168.56.0/24` of vagrant must
  not be used by any other interface of the host, so the setup can't run together with the other
  local setups.
- The first start takes long, because the virtual machines are installed and the images are copied
  into all of them.
- `--down` destroys the virtual machines, but keeps the saved images and the CA in
  `temporary_files/vagrant`.

## update_version.sh

### Purpose

Sets the version of all components in the repository to the same value:

- the rust-workspace (`Cargo.toml`, which all crates inherit, and `Cargo.lock`)
- the helm-chart (`version` and `appVersion` of `deploy/k8s/ainari/Chart.yaml`)
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
  `kitsudaiki/ainari_build_base` are not updated.
- The version in `setup.py` is only the default, the CI overwrites it with the tag.
- The script doesn't commit or tag anything.
