# Repository structure

This section provides a basic overview of the repository and its components, in order to make it
easier for a new person to understand the code.

## General

```text
.
├── .githooks                   # git-hooks
├── .github                     # ci-pipeline, issue-templates
├── .vscode                     # launch-configs to run the stack locally in VSCode
├── deploy
│   ├── k8s                     # gateway, which publishes the stack
│   ├── operator                # kubernetes-operator, which deploys the whole stack
│   └── zarf                    # offline-installation of the whole stack with zarf
├── dockerfiles
│   ├── debian_based            # debian-based images for local debugging
│   ├── dockerhub               # descriptions of the images on Docker Hub
│   ├── files                   # start-scripts, shared by both variants
│   └── nix_based               # nix-based images of the ci, with the nix-flake
├── docs
├── example_configs
│   └── ainari
├── scripts
├── src
│   ├── binaries
│   │   ├── hanami
│   │   ├── isekai
│   │   ├── izakaya
│   │   ├── miko
│   │   ├── neko
│   │   ├── omamori
│   │   ├── omikuji
│   │   ├── onsen
│   │   ├── ryokan
│   │   ├── sakura
│   │   ├── torii
│   │   └── torii-ebpf
│   ├── cli
│   │   └── ainarictl
│   ├── dashboard
│   ├── libs
│   │   └── rust
│   │       ├── ainari_api
│   │       ├── ainari_api_structs
│   │       ├── ainari_clients
│   │       ├── ainari_common
│   │       ├── ainari_files
│   │       ├── ainari_hardware
│   │       └── torii_common
│   └── sdk
│       ├── go
│       └── python
├── testing
│   ├── ainari_test             # end-to-end tests
│   ├── kind
│   ├── local_stack
│   └── vagrant
├── Cargo.toml                  # cargo-workspace of all rust-crates
├── docker-compose.yml          # local test-environment
├── Makefile
└── zensical.toml               # config of the documentation
```

- **.githooks**

    Git-hooks, at the moment only a pre-commit-hook, which formats the c-files with clang-format.

- **.github**

    The ci-pipeline (`workflows/build_test.yml`), the code-scanning with CodeQL and OpenSSF
    Scorecard, the config of dependabot, the actions to generate the badges and the templates for
    issues and pull-requests.

- **.vscode**

    Launch-configs and tasks to run and debug the whole stack locally in VSCode without any
    container. See `.vscode/Readme.md`.

- **deploy**

    - **k8s/gateway.yaml**

        Gateway of the gateway-api with a tls-passthrough listener, to which the operator attaches
        the TLSRoutes of the components.

    - **operator**

        The kubernetes-operator, which deploys the whole stack from a single custom-resource of the
        kind `Ainari`, including all keys and passwords. See [Kubernetes-installation](../../deployer/installation/kubernetes_installation.md)
        and `deploy/operator/README.md`.

    - **zarf**

        Offline-installation of the whole stack with [zarf](https://zarf.dev): the package of
        `zarf.yaml` with the operator, the images of all components, cert-manager and Envoy
        Gateway, which is created by `scripts/create_zarf_package.sh`. See
        [Offline-installation](../../deployer/installation/offline_installation.md).

- **dockerfiles**

    Dockerfiles of the build-environment, of the services, of torii, of the dashboard, of the
    documentation, of the operator and of the container with the tools for the local testing
    (`Dockerfile_local_test_tools`). The Dockerfiles of the images exist in two variants:
    `nix_based` with the packages pinned by nix, which the CI and the vagrant-setup use, and
    `debian_based`, which is easier to debug and used by the docker-compose- and kind-setup. The
    image of the operator exists only in the nix-based variant. `dockerhub` contains the
    descriptions of the published images on Docker Hub. See
    [Packages of the docker-images](docker_images.md).

- **docs**

    Zensical-documentation, where also this page belongs to. The config is `zensical.toml` in the
    root of the repository.

- **example_configs/ainari**

    Example-configs of all services. They are also used for tests within the ci-pipeline to make
    sure, that these examples are up-to-date.

- **scripts**

    Scripts to build the docker-images, to create a local CA, to set up and tear down the local
    test-environments (docker-compose, kind and vagrant), to collect the OpenAPI-specs for the
    REST-API documentation and to create the offline-installation with zarf. See
    [Scripts](scripts.md).

- **src**

    - **binaries**

        All backend-services, written in rust. This is the main part of the repository.

        | Binary       | Description                                                                                          |
        | ------------ | ---------------------------------------------------------------------------------------------------- |
        | `miko`       | authentication, users, projects and quotas; entry-point for the clients                              |
        | `hanami`     | manages the sakura-hosts, schedules the VMs on them, networks, floating IPs and network filters      |
        | `sakura`     | runs and supervises the virtual machines of a single host with cloud-hypervisor                      |
        | `torii`      | gateway with the routes, proxies, packet-filters and NAT of the virtual networks                     |
        | `torii-ebpf` | eBPF/XDP-programs of the datapath of torii                                                           |
        | `ryokan`     | manages the onsen-hosts and the images and snapshots stored on them                                  |
        | `onsen`      | storage for the files of the images and snapshots; only reachable over grpc by ryokan and sakura     |
        | `omamori`    | key-manager to upload, generate and download keys                                                    |
        | `omikuji`    | central collector of the error- and audit-logs of all components                                     |
        | `isekai`     | orchestrates the encrypted backups and restores of virtual machines and whole projects               |
        | `izakaya`    | key-exchange with MLS, which distributes the IPsec-keys to the sakura-hosts and torii-gateways       |
        | `neko`       | root-wrapper, which executes only commands of an allow-list with root-privileges                     |

    - **cli/ainarictl**

        The CLI-client, written in Go. See [CLI and SDK](../../user/cli_sdk/cli_sdk_docu.md).

    - **dashboard**

        The web-dashboard, written in Vue and Typescript. The app itself is located in `app`.

    - **libs/rust**

        Libraries, which are shared by the binaries.

        - **ainari_api**

            Common parts of the REST-APIs like the authentication- and CORS-middleware, error-types
            and the endpoints, which all services provide.

        - **ainari_api_structs**

            Structs of all requests and responses of the REST-APIs.

        - **ainari_clients**

            Client-functions for the communication between the services, over http and grpc.

        - **ainari_common**

            Common functions, like the config-handling, the logger, the error-types and the handling
            of secrets.

        - **ainari_files**

            Encryption and decryption of the image-files.

        - **ainari_hardware**

            Reads the cpu, memory and disk of the host.

        - **torii_common**

            Structs, which are shared between torii and its eBPF-programs in `torii-ebpf`.

    - **sdk**

        The Python-SDK and the Go-SDK, which is used by the CLI.

- **testing**

    - **ainari_test**

        The end-to-end test `vm_lifecycle_test.py`, which walks through the whole life-cycle of
        virtual machines with the Python-SDK. The single test-steps are split into the suites of
        `suites`, for example for the authentication, networking, snapshots and backups.

    - **kind**

        Scripts and cluster-config for the test-environment with the operator on a kind-cluster
        (kubernetes in docker). See [Kind setup](local_testing/kind_setup.md).

    - **local_stack**

        Description, configs and setup-scripts of the local docker-compose test-environment. See
        [Local test environments](local_testing/local_testing.md).

    - **vagrant**

        Vagrant- and ansible-files for the test-environment with the operator on a kubernetes of
        multiple virtual machines.
