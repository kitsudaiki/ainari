# Repository structure

This section provides a basic overview of the repository and its components, in order to make it
easier for a new person to understand the code.

## General

```text
.
├── .github                     # ci-pipeline, issue-templates
├── deploy
│   ├── k8s                     # service of an ingress-nginx-controller
│   └── operator                # kubernetes-operator, which deploys the whole stack
├── dockerfiles
│   ├── debian_based            # debian-based images for local debugging
│   ├── files                   # start-scripts, shared by both variants
│   └── nix_based               # nix-based images of the ci, with the nix-flake
├── docs
├── example_configs
│   └── ainari
├── scripts
├── src
│   ├── archive
│   ├── binaries
│   │   ├── hanami
│   │   ├── miko
│   │   ├── neko
│   │   ├── omamori
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
│   ├── local_stack
│   └── vagrant
├── Cargo.toml                  # cargo-workspace of all rust-crates
├── docker-compose.yml          # local test-environment
├── Makefile
└── zensical.toml               # config of the documentation
```

- **.github**

    The ci-pipeline (`workflows/build_test.yml`), code-scanning and the templates for issues.

- **deploy**

    - **k8s/ingress-nginx-controller.yaml**

        Service of the type `LoadBalancer` for an ingress-nginx-controller.

    - **operator**

        The kubernetes-operator, which deploys the whole stack from a single custom-resource of the
        kind `Ainari`, including the wireguard-configs between onsen, ryokan and sakura and all keys
        and passwords. See [Kubernetes-installation](../../deployer/installation/kubernetes_installation.md)
        and `deploy/operator/README.md`.

- **dockerfiles**

    Dockerfiles of the build-environment, of the services, of torii, of the dashboard, of the
    documentation and of the container with the tools for the local testing. The Dockerfiles of the
    images exist in two variants: `nix_based` with the packages pinned by nix, which the CI and the
    vagrant-setup use, and `debian_based`, which is easier to debug and used by the docker-compose-
    and kind-setup. See [Packages of the docker-images](docker_images.md).

- **docs**

    Zensical-documentation, where also this page belongs to. The config is `zensical.toml` in the
    root of the repository.

- **example_configs/ainari**

    Example-configs of all services. They are also used for tests within the ci-pipeline to make
    sure, that these examples are up-to-date.

- **scripts**

    Scripts to build the docker-images, to create a local CA, to set up and tear down the local
    test-environments (docker-compose, kind and vagrant) and to collect the OpenAPI-specs for the
    REST-API documentation.

- **src**

    - **archive**

        Old archived code, which may be used or refactored again in the future. It was placed into
        this dedicated directory, because dead code shouldn't be mixed with the rest. At the moment
        this is only the old ansible-deployment.

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
        | `omamori`    | public-keys and encrypted secrets                                                                    |
        | `neko`       | root-wrapper, which executes only commands of an allow-list with root-privileges                    |

    - **cli/ainarictl**

        The CLI-client, written in Go. See [CLI and SDK](../../user/cli_sdk/cli_sdk_docu.md).

    - **dashboard**

        The web-dashboard, written in Vue and Typescript.

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

    - **local_stack**

        Description of the local test-environments and the end-to-end test `vm_lifecycle_test.py`,
        which walks through the whole life-cycle of virtual machines with the Python-SDK. See [Local
        test environments](local_testing/local_testing.md).

    - **vagrant**

        Vagrant- and ansible-files for the test-environment with the operator on a kubernetes of
        multiple virtual machines.
