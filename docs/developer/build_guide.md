# How to build

## Requirements

The datapath of torii attaches eBPF/XDP-programs to the interfaces and redirects the packets
between them, so the kernel of the host, where the services are running, requires eBPF/XDP-support.
The recommended minimum is a kernel `5.10`, which is the first LTS-kernel, where XDP-redirect is
mature enough. The following distributions provide such a kernel:

| Distribution               | Minimal version     | Default kernel             | Notes                                                                                            |
| -------------------------- | ------------------- | -------------------------- | ------------------------------------------------------------------------------------------------ |
| Ubuntu                     | 22.04 LTS           | 5.15                       | 24.04 LTS ships 6.8. 20.04 only has 5.4 by default and is only usable with the HWE-kernel (5.15) |
| Debian                     | 11 (Bullseye)       | 5.10                       | 12 ships 6.1, 13 ships 6.12                                                                      |
| RHEL / Rocky / AlmaLinux   | 9                   | 5.14 (with many backports) | 8 with kernel 4.18 is not recommended, because parts of XDP are only a Technology Preview there  |
| CentOS Stream              | 9                   | 5.14                       | Same kernel line as RHEL 9                                                                       |
| Fedora                     | any supported (41+) | 6.x                        | New kernels arrive quickly                                                                       |
| SUSE Linux Enterprise      | 15 SP4              | 5.14                       | SP3 only has 5.3                                                                                 |
| openSUSE Leap / Tumbleweed | 15.4 / rolling      | 5.14 / current 6.x         |                                                                                                  |
| Amazon Linux               | 2023                | 6.1                        | Amazon Linux 2 only works with the optional 5.10-kernel                                          |
| Arch Linux                 | rolling             | current 6.x                | Fine as long as it is up to date                                                                 |

The containers use the kernel of the host, so for WSL2 or Docker Desktop the kernel of their VM
is relevant. The kernel of a host can be checked with:

```bash
uname -r
grep -E 'CONFIG_BPF=|CONFIG_BPF_SYSCALL=|CONFIG_XDP_SOCKETS=' /boot/config-$(uname -r)
```

## Preparation

- Install packages

    - For Ubuntu 24.04 and Debian 12:

        ```bash
        sudo apt-get install gcc curl git pkg-config protobuf-compiler libssl-dev libsqlite3-dev libmariadb-dev
        ```

- Install the Rust-compiler with [rustup](https://rustup.rs/) (minimum version: `1.85.1`)

- Install the nightly-toolchain and `bpf-linker`

    The build-script of torii compiles its eBPF-programs for the `bpfel-unknown-none` target. This
    requires the nightly-toolchain with `rust-src` beside the default stable toolchain, and the
    linker for eBPF-programs:

    ```bash
    rustup toolchain install nightly --component rust-src
    cargo install bpf-linker
    ```

- Clone the repository

    ```bash
    git clone https://github.com/kitsudaiki/ainari.git
    ```

## Build the services

- Compile all services

    ```bash
    cd ainari

    cargo build --release
    ```

    A single service can be built with `-p`, for example `cargo build --release -p sakura`.

- The resulting binaries are placed in `./target/release/`, for example `./target/release/sakura`

- Cargo-profiles

    | Profile   | Usage                                                                              |
    | --------- | ---------------------------------------------------------------------------------- |
    | `dev`     | default of `cargo build`, without optimization                                     |
    | `release` | the release-builds with link-time-optimization                                     |
    | `local`   | like `release`, but without link-time-optimization, so much faster to build; used by the local test-environments |

    The profile is selected with `--profile`, for example `cargo build --profile local`.

!!! info

    To run sakura, the hypervisor [cloud-hypervisor](https://github.com/cloud-hypervisor/cloud-hypervisor)
    and its firmware are required on the host, at the paths of `binary_path` and `firmware_path` in
    the [config](../../deployer/config/sakura_config.md). The docker-image of sakura already
    contains both.

## Run cargo tests

- The tests read the example-configs from `/etc/ainari` and create their databases beside them, so
    the directory has to belong to the user, which runs the tests:

    ```bash
    sudo cp -r ./example_configs/ainari /etc/ainari
    sudo chown -R "$(id -u):$(id -g)" /etc/ainari
    ```

- Run the tests with only one thread, because some tests access the same database:

    ```bash
    cargo test -- --test-threads=1
    ```

For the tests of the whole stack, see [Local test environments](local_testing/local_testing.md).

## Build docker-images

All rust-services except torii are built from one Dockerfile, which compiles them together and has
one target per service: `miko`, `hanami`, `sakura`, `ryokan`, `onsen` and `omamori`.

Run `docker build -f dockerfiles/Dockerfile_services --target <SERVICE> -t <DOCKER_IMAGE_NAME> .`

!!! example

    ```bash
    docker build -f dockerfiles/Dockerfile_services --target sakura -t sakura:test .
    ```

Torii needs the toolchain for its eBPF-programs, so it has its own Dockerfile:

```bash
docker build -f dockerfiles/Dockerfile_torii -t torii:test .
```

Both Dockerfiles take the build-argument `CARGO_PROFILE`, which defaults to `release`. The local
test-environments use `--build-arg CARGO_PROFILE=local`, which is much faster to build.

The dashboard has its own Dockerfile too:

```bash
docker build -f dockerfiles/Dockerfile_dashboard -t dashboard:test .
```

!!! info

    `scripts/build_docker_images.sh` builds all images with the tag `local_test` and saves them in
    `temporary_files/ainari_docker_files.tar`. `scripts/build_local_images.sh` builds the images
    for the local test-environments with the tag `local`.

## Build CLI-client

- Install [go](https://go.dev/doc/install) (the required version is defined in the `go.mod`-file
    of the cli)

- Build the cli

    ```bash
    cd ./src/cli/ainarictl
    go build .
    ```

- The resulting binary `ainarictl` is placed beside the sources within the same directory

## Build python-SDK

- Install the SDK directly from the repository

    ```bash
    pip3 install ./src/sdk/python/ainari_sdk
    ```

- Or build it as wheel-package, like the CI-pipeline does

    ```bash
    cd ./src/sdk/python/ainari_sdk
    pip3 install wheel
    python3 setup.py bdist_wheel --universal
    ```

    The package is placed in `./dist/`. The version of the package is set with the
    environment-variable `PYTHON_PACKAGE_VERSION`.

## Build dashboard

- The dashboard reads its config from `/etc/ainari/dashboard_config.json`. Copy the example-config,
    if it doesn't exist yet:

    ```bash
    sudo mkdir -p /etc/ainari
    sudo cp ./example_configs/ainari/dashboard_config.json /etc/ainari/
    ```

- Build and start the dashboard

    ```bash
    cd ./src/dashboard
    docker compose up
    ```

- Open the address, which is shown in the output, in the web-browser (`http://localhost:5173/`).
    The sources in `./src/dashboard/app` are mounted into the container, so changes are shown live.

## Prechecks

There are a bunch of pre-checks at the beginning of the CI-pipeline, which can fail and where it is
useful to be able to run the same checks locally for debugging.

### Rust-checks

- Formatting: `cargo fmt --check` (or `cargo fmt` to fix it)

- Clippy: `cargo clippy --all-targets --all-features`

### Flake8-check

- run `pip3 install flake8` and then `flake8 src/sdk/python`

### Commit-messages

- The commit-messages are checked with [commitlint](https://commitlint.js.org/) and the rules of
    `commitlint.config.cjs`. See [Git-Workflow](git_workflow.md) for the format.

### Secret-scan

- run `git ls-files -z | xargs -0 detect-secrets-hook --baseline .secrets.baseline`

It is possible, that the check fails, even if there are no (new) secrets in the code, because of
some other code-movements. The check compares all to the `.secrets.baseline`-file, where also
line-numbers are marked. To update the file to get the test green again:

- install [detect-secrets](https://github.com/Yelp/detect-secrets)

- update file with `detect-secrets scan > .secrets.baseline`

## Build docs

The documentation is built with [Zensical](https://zensical.org/) and configured in
`zensical.toml`.

- Install Zensical

    ```bash
    pip3 install zensical
    ```

- Run the local preview in the root of the repository

    ```bash
    zensical serve
    ```

- Open the web-browser with the address `http://localhost:8000/` to see the docs. The preview is
    updated live with all changes of the files.

- To build the static site into `./site/`, run `zensical build`
