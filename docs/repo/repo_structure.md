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
    │   │   ├── frontend
    │   │   │   └── OpenHanami-Dashboard
    │   │   └── sdk
    │   │       └── javascript
    │   │
    │   ├── cli
    │   │   └── hanamictl
    │   │       ├── common
    │   │       └── resources
    │   │
    │   ├── hanami
    │   │   └── (see below)
    │   │
    │   ├── libs
    │   │   ├── cpp
    │   │   │   └── hanami_core
    │   │   ├── rust
    │   │   │   ├── hanami_cluster_parser
    │   │   │   ├── hanami_common
    │   │   │   └── hanami_dataset
    │   │   └── protobuf
    │   │
    │   ├── sdk
    │   │   ├── go
    │   │   └── python
    │   │
    │   └── third-party-libs
    │       └── jwt-cpp
    │
    └── testing
        ├── go_cli_api
        └── python_sdk_api

-   **deploy**

    Contains the helm-chart doploying hanami on kubernetes.

-   **docs**

    Mkdocs-Documentation, where also this page here belongs to.

-   **example_configs**

    Example-configs for hanami. They are also used for tests within the CI-pipeline to make sure,
    that these examples are up-to-date.

-   **src**

    -   **archive**

        Old archived code, which is planned to be used or refactored again in the future. This was
        placed into the dedicated directory, because dead-code shouldn't be mixed within the rest.

    -   **cli**

        Code of the CLI-client written in Go

    -   **hanami**

        Contains the main-part of [hanami](/repo/repo_structure/#hanami-source-code)

    -   **libs**

        Libraries used by the binaries.
        
        -   **cpp**

            -   **hanami_core**

                Core-Library, which contains all functions to handle, process and interact with the neural networks
        
        -   **rust**

            -   **hanami_cluster_parser**

                Contains the parser for the cluster-templates.

            -   **hanami_common**

                Common rust-functions used in the project.

            -   **hanami_dataset**

                Contains functions to read and write dataset-files.
        
    -   **sdk**

        Code of the python SDK library and the go-version of the SDK-lib used by the CLI

    -   **third-party-libs**

        Third-party libraries as submodules. At the moment this is only the jwt-lib

-   **testing**

    Skripts for basic tests of the python SDK and the CLI. They are used within the CI-pipeline for
    basic tests of the components and the API.

## hanami source-code

    └── src
        └── hanami
            ├── src
            │   ├── api
            │   │   └── http_endpoints
            │   │       ├── auth
            │   │       ├── checkpoint
            │   │       ├── cluster
            │   │       │   └── task
            │   │       ├── dataset
            │   │       ├── project
            │   │       └── user
            │   ├── core
            │   ├── database
            │   └── documentation
            └── tests

-   **api**

    All functions for the API to interact with the server.

    -   **http_endpoints**

        Definitions of all REST-API-endpoints

-   **core**

    Functionality to interact with the c++-core-libraries for the neural networks. Also contains the handling of cluster-tasks.

-   **database**

    Contains the definitions of the database-tables and all functions to interact with the
    SQL-database
