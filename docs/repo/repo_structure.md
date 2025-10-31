# Repoistory structure

This sections should provide a basic overview of the repository and its components, in order to make
it easier for a new person to understand the code.

## General

    .
    ├── deploy
    │   └── k8s
    │
    ├── docs
    │
    ├── example_configs
    │
    ├── src
    │   ├── archive
    │   │
    │   ├── binaries
    │   │       ├── bento
    │   │       ├── sakura
    │   │       └── miko
    │   │
    │   ├── cli
    │   │   └── ainarictl
    │   │
    │   ├── dashboard
    │   │
    │   ├── libs
    │   │   ├── rust
    │   │   │   ├── ainari_api
    │   │   │   ├── ainari_api_structs
    │   │   │   ├── ainari_clients
    │   │   │   ├── ainari_cluster_parser
    │   │   │   ├── ainari_common
    │   │   │   ├── ainari_dataset
    │   │   │   └── ainari_hardware
    │   │   └── protobuf
    │   │
    │   └── sdk
    │       ├── go
    │       └── python
    │
    └── testing
        ├── go_cli_api
        └── python_sdk_api

-   **deploy**

    Contains the helm-chart doploying sakura on kubernetes.

-   **docs**

    Mkdocs-Documentation, where also this page here belongs to.

-   **example_configs**

    Example-configs for sakura. They are also used for tests within the CI-pipeline to make sure,
    that these examples are up-to-date.

-   **src**

    -   **archive**

        Old archived code, which is planned to be used or refactored again in the future. This was
        placed into the dedicated directory, because dead-code shouldn't be mixed within the rest.

    -   **cli**

        Code of the CLI-client written in Go

    -   **dashboard**

        Contains the dashboard written in Vue and Typescript.

    -   **binaries**

        Contains all backend-binaries. Its the main-part of the repository.

    -   **libs**

        Libraries used by the binaries.
        
        -   **rust**

            -   **ainari_api**

                Common functions for the REST-API like authentication stuff and commonly used endpoints.

            -   **ainari_api_structs**

                Contains all structs of each request and response message.

            -   **ainari_client**

                Client-functions for communication between the binaries within the project.

            -   **ainari_cluster_parser**

                Contains the parser for the cluster-templates.

            -   **ainari_common**

                Common rust-functions used in the project.

            -   **ainari_dataset**

                Contains functions to read and write dataset-files.

            -   **ainari_hardware**

                Hardware related functions to read and write system settings.
        
    -   **sdk**

        Code of the python SDK library and the go-version of the SDK-lib used by the CLI

    -   **third-party-libs**

        Third-party libraries as submodules. At the moment this is only the jwt-lib

-   **testing**

    Skripts for basic tests of the python SDK and the CLI. They are used within the CI-pipeline for
    basic tests of the components and the API.
