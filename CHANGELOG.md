# Changelog

## [Unreleased]

### BREAKING-CHANGES

#### API-Breaking

- renamed cluster to model

#### Database-Breaking

- in the meta-cluster table of hanami the name of the cluster is now also stored
- renamed cluster to model

#### Checkpoint-Breaking

- block-size was changed from 128 to 512 as workaround for a conceptional bug in bigger tests

### Added

- Vagrantfile with ansible playbook to deploy multiple virtual machines with a kubernestes with fully deployed ainari-setup and sdk-api-test
- image pull-policy can now be changed by the helm-values
- support other namespaces thand "default"
- support for multiple sakura instances
- validation for task-input to avoid a mismatch between given input and the model
- validation for task-input to avoid a mismatch between given input and the dataset
- dashboard:
    - added error-popups
    - validation of input-fields and error-message for invalid inputs
    - added cluster-list without actions to overview-page
    - automatic logout in case the token is expired
    - quota-management was added to the admin-section of the dashboard
    - dataset list in the dashboard now prints the number of rows and columns
    - cluster list in the dashboard now prints the cluster adress
- added forecast-length as new parameter for train-tasks (only via python-sdk at the moment)

### Changed

- changed the image-build process for the new vagrant-test-setup
- sakura and onsen are now statefulsets within the kubernetes setup
- changed learning-factor from 0.1 to 0.05 for core-blocks
- changed border-value from 0.05 to 0.1
- dashboard
    - updated imports
    - messages when api-requests failed
    - new progress-bar for task-progress with live update of the current task progress
    - hide admin-section in case that the user is not an admin
    - user-logo in the right upper corner now is always white with the starting letter of the user-id
    - gauge-charts of the overview-page now has animation
    - disabled the hexagon background pattern in the left upper corner again

### Fixed

- fixed broken version-output of the pre-build docker images
- restart of hanami and sakura instance in regard of the cluster was now fixed to avoid broken cluster and hosts after restart
- restarts of the same sakura-host doesn't result in duplications in hanami anymore
- progress-bar in dashboard for multiple epochs is now calculated correctly
- fixed number of output-values in case of int and float output-type
- task-abort-endpoint not works
- task-abort-modal was fixed in the dashboard
- fixed delete model with running tasks
- race-condition in resizing of the model was fixed (hopefully)

## v0.10.0

(Changes since v0.9.0)

### BREAKING-CHANGES

#### API-Breaking

- split "hanami" into a new "hanami", "sakura" and "torii"
- added new component named "Miko", where the endpoints for tokens, users and projects moved to.
- Sakura now uses the Miko to check provided token over a new client-connection
- improved version-output
- admin-flag is not a string instead of a boolean

#### Config-Breaking

- Because of the new component, Sakura requires a new client-connection in its config

#### Database-Breaking

- The size of the status-field in most of the tables was changed
- fixed size of uuid-fields in database

#### Dataset-Breaking

- datasets are now always AES-256 encrypted with a key generated and stored in omamori

#### Checkpoint-Breaking

- checkpoints are now always AES-256 encrypted with a key generated and stored in omamori

### Added

- new component "Miko" for authentication
- new componentn "Ryokan" for storage
- new component "Sakura", which contains the core of the previous "Hanami"
- new component "Torii" as proxy to the sakura-hosts
- new component "Omamori" with a very simple AES-crypto backend to store and download secrets
- use shared storage in kubernetes-setup to share files between Ryokan and Sakura
- the new Miko-component provides the addresses of all components
- new basic dashboard written in Vue.js and Typescript
- added Dashboard to the kubernetes-setup
- endpoints to get the number of used cluster, datasets, checkpoints and secrets
- in the kuberentes-setup internal used keys were moved into kubernetes-secrets

### Changed

- use context-object, which includes token, public-addresses and insecure-flag in one object, to
    make the sdk-functions smaller and cleaner
- keys are internally handled as new object, to prevent them from being printed on the console by
    accident
- Dockerfiles:
    - split into multiple-files, where each build only its own binary and not all, to reduce the
        build-time in the CI
    - base-images are pinned by a hash now
    - switch from ubuntu to debian-slim as base image to reduce the size
- arm64 images are now build by a nativ arm-runner instead of cross-compiling
- renamed the "insecure_clients"-flag to "skip_tls_verification" on client-side
- restructured the documenation to have user, developer and deployer section
- use "map_err" in the rust-code to reduce the code-base and make the code more readable

### Fixed

- fixed miscalculation in the potential-overflow between sections, to fix massive accuracy problems
    in bigger tests
- fixed cleanup of temporary resources


## v0.9.0

**Release-Date**: 2025-08-24

- Renamed project from OpenSakura to Ainari
- Translated the rest of the C++-code into Rust

### BREAKING-CHANGES

#### Checkpoint-Breaking

- Because new core, the checkpoint-files have a complete new structure.

### Added

- central cluster-handler was added
- added rust fmt check to CI-pipeline
- added rust clippy check to CI-pipeline
- added version-endpoint to request the version of the server (updated python and go sdk and cli for
    the new endpoint)
- added config-flag to limit the number of used cpu-codes (default is 0, which means all threads are
    used)

### Changed

- refactored core into Rust-code
- block can trigger other blocks more indepentantly
- also some new concepts in the core as response to the behavior of Rust
- removed and forbid unsafe
- replace unwrap-functions outside of tests

### Fixed

- fixed randomly failing dataset-upload by adding a file-sync to ensure the data are written on disc
    before converting
- fixed commit-linter in CI, which was misconfigured

### Removed

- removed the `use_of_free_memory`-config (for now), because memory for the network is not
    pre-allocated anymore

## v0.8.0

**Release-Date**: 2025-06-10

Translated big parts of the program from C++ into Rust:

- REST-API endpoints
- Database-connection
- Config-file
- Cluster-Template-Parser
- Task-handling

### BREAKING-CHANGES

#### API-Breaking

- IDs of API-requests of now in the query-path instead of the body
- general changes, based on the new api-framework
- datasets are now uploaded over the REST-API instead of websockets
- OAuth-styling of token-requests for a more standardized styling and behaviour
- upload files of datasets via API-endpoint instead of websocket
- removed direct- and task-mode separation, because it is not necessary anymore the with the new
    architecture
- removed websocket-connections in general
- Allow the definition of multiple epochs in one single train-task
- Inputs of the CLI were in general updated by removing and changing a bunch of flags

#### Template-Breaking

- Added `;` to the end of the files of the templates

#### Database-Breaking

- new dataset-file-header, because of the better serialization-features of rust

#### Database-Breaking

- updated database-schemas in confext of the code-translation

#### Config-Breaking

- switched to toml-config
- some restructuring and changes, based on the new toml-format and the new Rust-implemenation

#### Temorarily removed (will be added in the future again)

- policies with roles for the endpoints (only admin and user at the moment)
- attaching projects to users
- visibility settings of resources
- central audit and error logs in the database

### Added

- New REST-API endpoint to interact directly with a cluster for training and requests

### Changed

- Direct-requests against the cluster can now be done in parallel to active tasks

## v0.7.0

**Release-Date**: 2025-03-19

### BREAKING-CHANGES

#### API-Breaking

- removed the Binary input option again
- functions of the python-sdk now returning a dict instead of a json-formated string
- added `created_by`, `updated_at`, `updated_by` and `deleted_by` to all endpoints, where the
    database was updated

#### Checkpoint-breaking

- updated core-structure for a cleaner communication between different hexagons
- merge connection-, neuron- and synapse-block into one block object
- updated section-processing for better input-handling especially for binary input data
- replaced the old split-process for sections to avoid loosing half of the trained section in the
    process
- dynamically link sections to connections in order to make memory usage more efficient
- net input can now also handle negative values as input

#### Template-Breaking

- removed the reductions-mode setting

#### Database-Breaking

- Added `task_uuid` to datasets
- Datasets, which are created as result form request-task, have now a random uuid instead of the
    uuid of the initial request-task
- replaced the old `creator_id` for users and projects
- added `created_by`, `updated_at`, `updated_by` and `deleted_by` to all relevant database-tables

### Added

- delete-all functions for all resources were added to the python-sdk
- function to wait for a task to be finished was added to the python-sdk
- a request-task with multiple output-hexagon generates now multiple datasets for each of them

### Changed

- the resize-process was updated to avoid the search-loop for a new free section
- updated ubuntu-version in docker-images to 24.04

### Fixed

- wrong return-code in one error-case of the create-train-task endpoint was fixed
- fixed offset in the output of the timeseries training from datasets
- download of dataset with a too big number of requested rows and offset was fixed to handle this
    cases correctly
- cluster where a hexagon has multiple possible next was not correct in path-finding

### Removed

- removed the remaining code of the old broken reduction-process, which doesn't really makes a
    difference with the current core

## v0.6.0

**Release-Date**: 2024-12-14

### BREAKING-CHANGES

#### API-Breaking

- the input definition for the tasks was changes to removed the naming restriction between
    dataset-column and hexagon-name
- the external REST-API endpoint to validate a token was removed and replaced by an internal
    function, which is used instead for all incoming api-requests
- changed password to passphrase, which is now required to be base64 encoded

#### Checkpoint-breaking

- added axons to be able to connect hexagons of the entire cluster with each other
- the synapse-block-links were moved into its own vector for cleaner separation and better sizes of
    the objects within the memory

#### Template-Breaking

- removed `target`-keyword from cluster-templates
- removed number of nodes from inputs and outputs from the cluster-templates

#### Database-Breaking

- the base64-representation of the passphrases is now used for the pw-hash inside the database,
    which makes all old hashes in the database invalid

### Added

- added new workaround to be able to handle binary input-data
- added new output-modi to provide more output-options to combine multiple outputs bitwise into a
    bigger value
- added metrics for number of blocks and number of sections to cluster get output

### Changed

- renamed repo from `Sakura` to `Ainari`
- tls-termination in kubernetes setup was moved from the ingress into a container within the same
    pod like the sakura-process
- number of input- and output-neurons now scales with the given data, so defining them hard inside
    of the cluster-template is not longer necessary
- moved converting user-context from the endpoints to a central position to avoid unnecessary
    redundant code
- moved cluster io-functions internally into the processing and backpropagarion files for the
    cpu-code
- item-buffer was internally changed to be more compatible for further attempts of GPU-support
- made internal processing more generic, to have the same worker-workflow for all coming backends
- use debug builds with ASan-check for tests of the CI (unit-, functional-, memory-leak-, sdk-api-
    and cli-api-tests)
- default-values for initial admin-credentials and token-key were removed from the helm-chart and
    are now marked as required
- synapse-tolerance was changed to static values

### Fixed

- fixed memory-leak in DataSetFileHandle
- fixed memory violations found by the new added ASan-memory-check
- some cases of invalid http-requests had returned an 200 reponse with empty body, which was fixed
- changed token-key in helm-chart from configmap to secret
- fixed memory-corruption in cluster-resize, because a pointer was used after free
- fixed missing drawio-images in documentation when generating in CI
- fixed random crash, which can appear while creating checkpoints
- when making a request over direct-io but the output is empty, there was no response and the
    connection got stuck, which was fixed
- when using direct-io right from the beginning to train a cluster, the cluster remained empty,
    which was fixed

### Removed

- the ansible-playbook was moved ot the archive and removed from the documentation and CI

## v0.5.0

**Release-Date**: 2024-08-11

### BREAKING-CHANGES

#### API-breaking

- tasks were updated to a more generic way
- request-results were removed from the databases and api, because results are now also stored as
    data-sets again
- renamed REST-API endpoints after removing the dashboard, because there is no separation between
    dashboard and commands anymore
- added version-numbers to the REST-API endpoints

#### Checkpoint-Breaking

- number of neurons per brick now grows dynamically and doesn't have to be defined in the
    cluster-templates anymore
- changed output-bricks to modifiy the size of the output-brick independently from the number of
    output-neurons
- updated buffer in clusters to remove the old byte-arrays
- new implementation for the checkpoints for the new data-structure of the clusters and for more and
    better error-handling
- header of checkpoint-files was changed from 512 byte to 4096 byte to have more space for
    additional configs

#### Dataset-Breaking

- new implementation for the datasets for a more generic handling of data and read-function to allow
    get specific sections from the data-set file
- header of dataset--files was changed from 512 byte to 4096 byte to have more space for additional
    configs

#### Template-Breaking

- cluster-templates were modified for the new dynamic neuron-allocation and the new output
- renamed bricks into hexagons

#### Database-Breaking

- database-tables now have a `created_at`-timestamp for the entry
- entries of the database-tables are now deleted anymore, but instead are only marked as deleted
    with a `deleted_at`-timestamp

### Added

- added base64-type database-entries to easier store and get more complex strings from the database
- added documentation of the CLI-commands
- added function to download content of datasets
- added flag to the CLI to get every output as json-string
- added flag to the CLI to disable the TLS-check, when necessary
- database-tables now have proper functions to add and get structs from/to the database instead of
    only json-objects, which makes the code more explizit
- added new approach for time-series for input-data of csv-files
- added unit-tests for:
    - clusters
    - ckeckpoint create and restore
    - data-sets read and write
    - all database-tables
- improved CI-pipeline:
    - added commit-linter
    - added check for binary executables within the commited code
    - added secret-scan
    - build, use and upload helm-package
    - build, use and upload python-package of sdk
    - build, use and upload client
    - added CLI tests
    - automatic versioning of python- and helm-package
    - added compile-tests for different C++ compiler
    - added sdk-api-test with different Python versions
    - added integration-tests with different Kubernetes versions
    - added CodeQL-check as additional workflow
    - added OpenSSF-checks as additional workflow
- added dependabot-bot to repository

### Changed

- datasets are now streamed internally and not fully loaded into the memory anymore, when creating a
    task, which is a massive reductions in the memory-consumption
- the registration of new endpoints within the code were updated by removing old leftovers
- the registration of new database-columns within the code were updated to look like the style of
    config-registrations and so on
- internal handling of tasks was updated for cleaner code
- endpoint to list projects of a user, were moved within the code
- renamed src-subdirectory `Sakura` to `sakura` to be more consistent to the rest
- split CUDA-kernel into multiple files
- replace the json submodule by the apt-package

### Fixed

- fixed build of ARM64 docker-images, where were completely broken...
- fixed compiling of the C++ code under Ubuntu 24.04
- fixed csv-upload in python-sdk-lib
- fixed a value-limitation of 16bit, where it was not necessary
- fixed multiple commands within the CLI while adding automatic tests to the CI-pipeline
- fixed security-issues found by the new added CodeQL:
    - tls-check in CLI is not disabled be default anymore
    - fixed clear-text-logging of the password-input in the CLI
    - fixed a range-check in a for-loop within the backend

### Removed

- the `common`-directory in Sakura was removed and most of the functions were moved to the
    `ainari_common`-lib
- disabled CUDA-kernel, because at the moment it doesn't brings a performance increasement, but only
    slows down the development (will be added in the future again, after finding a better use-case
    for the GPU)
- disabled dashboard and moved it to the archive subdirectory (will be added in the future again,
    after a complete refactoring of the code)
- removed old OpenCL-kernel

## v0.4.1

**Release-Date**: 2024-03-25

### Added

- provide docker-images also for arm64 architecture
- new steps were added to the CI-pipeline:
    - added automatic build and push of amd64 and arm64 docker-images to docker hub
    - added automatic build and push for the documentation to docker hub
    - added SDK-API-Test
    - added integration-tests for kubernetes- and ansible-setup with amd64 docker-images
    - added flake8 to lint python-code
    - added ansible-lint to lint ansible-code

### Changed

- reduce cpu usage in idle state by blocking inactive threads
- use [earthly](https://github.com/earthly/earthly) as new build-script and replace old pre-build
    docker-images in CI
- use random instead of static salt-value for password-hashes
- websocket-connections in SDK use now async-functions

### Fixed

- fixed linting-errors in python- and ansbile-scripts
- fixes to run code in virtual machines, where reading system information had broke the setup

## v0.4.0

**Release-Date**: 2024-01-21

### BREAKING-CHANGES

#### Checkpoint-Breaking

- Complete restructure of the core to improve the performance of the CUDA-kernel by enabling more
    capabilities for parallel processing.
- Synapse-blocks are not limited to a single cluster-scoped buffer, but instead are stored in a
    global buffer, which is shared by multiple cluster.
- The number of used synapse-blocks is now independed from the number of neuron-blocks of a brick.
    They scale now better over time. This way the number of neurons in the clsuter-template are only
    a maximum value. This way it is easier to too high values in the cluster-template are no
    pain-point anymore.

#### API-Breaking

- removed c++ version of the SDK

### Added

- multi-threading, so now multiple cpu-threads can process the same cluster at the same time
- new endpoint to list logical hosts (cpu's and gpu's)
- new endpoint to move a cluster between cpu and gpu
- re-added the spiking neural network-feature for future testing-purpose
- re-added a connection-distance of more the one for future testing-purpose
- python version of the SDK
- basic cli-client in go
- new entry in config to define a location for temporary files while uploading
- new cleanup-loop for tempfiles, which deleted incative temporary files after a certain amount of
    time, to remove file of a broken upload-process
- the reduction-process was re-added to limit the amount of used memory while learning.

### Changed

- physical host is now separated into logical hosts (a cpu and a gpu are handled as 2 different
    logical hosts)
- replaced old api-test-tool by a new python-script, which use and test the new python version of
    the SDK
- moved from `qmake` to `cmake` as build-tool for the c++ code

### Removed

- removed GET-requests from audit-log
- removed c++ version of the SDK

## v0.3.3

**Release-Date**: 2023-11-17

### Fixed

- fixed broken login to dashboard
- fixed broken internal error-log and the error-log API-endpoint

## v0.3.2

**Release-Date**: 2023-11-10

### Added

- basic Ansible-playbook was added in order to deploy the project without kubernetes
- in kubernetes-setup:
    - now use cert-manager to generate certificates
    - ingress was added for ssl-termination of https-connections
    - persistent volume was added to persist data like database within the kuberntes-setup
    - node label was added to define the node, where sakura should be deployed by kubernetes
- `clang-format` file was added with a git pre-commit hook
- `clang-format` check and basic `cppcheck` were added to the ci-pipeline
- first contribution-guide and code-styling guide was added
- example-configs for testing purpose and guide to setup local test-environment
- at start there is now a check if the directories for the checkpoints and dataset, defined by the
    config, even exist

### Fixed

- fixed compile-error when trying to build on ARM64 architecture
- fixed compile-error in cude-code with the standard nvidia-toolkit in ubuntu 22.04
- fixed broken json-strings in javascript-sdk
- fixed false error response-codes in API

### Changed

- moved old readme files into the normal documentation
- use clang++ instead of g++ as compiler
- non-critical API-errors, like for example a 404 (not found) when searching for an unknown id,
    doesn't produce an internal error-output anymore

### Removed

- ssl-termination was removed from backend-API, because it will be done by another service in front
    of the API, like nginx

## v0.3.1

**Release-Date**: 2023-09-24

### Changed

- replaced json-parser by third-party library https://github.com/nlohmann/json
- updated internal registration of config-values
- updated internal registration of cli-arguments

### Fixed

- fixed bug in synapse-segmentation, which had impact in samller tests

### Removed

- Removed unused and deprecated checkpoint create and finalize endpoints

## v0.3.0

**Release-Date**: 2023-09-05

### BREAKING-CHANGES

#### All breaking

- Move the entire project from a micro-service architecture to a more monolithic architecture with
    only one executable but still multiple libraries, to reduce unnecessary complexity and increase
    performance and reliability of the program. Also it makes the developing process easier and
    faster, with is necessary in regard of my limited time-resources.
- Removed entire segment-layer between the clusters and bricks from core structure. This layer was
    originally intended for the separation of multiple threads, but now there is another strategy
    planed

#### API-Breaking

- REST-API endpoint for the generation of the documentation was removed
- Renamed `snapshot` to `checkpoint` and `learn` to `train`

#### Database-Breaking

- fixed sizes of some database-columns for user- and project-id's

#### Checkpoint-Breaking

- Bigger rework of the core structs to get rid of the update-positions struct

### Added

- documentation of config-entries and database-schemas can now also be generated as
    marddown-documents
- error-codes are now also written into the REST-API documentation
- new config-entry to enable the extremely experimental CUDA-processing
- more then one thread is now created for the core-processing, to be able to process multiple
    clusters in parallel
- the CUDA-version now also works with Checkpoints (create and read)

### Changed

- Rename repository/project from `Sakura-AI` to `Sakura`
- Renamed libraries to `ainari_...` and namespaces of the libraries to `Sakura`, because the
    originally naming and structure was from the time, when they were separate repositores and
    desired also for other projects. The new naming makes the names shorter and easier.
- replace custom jwt lib by `jwt-cpp` (https://github.com/Thalhammer/jwt-cpp)
- REST-API documentation is now generated as OpenAPI-specification and added to the documentation
    via swagger-ui
- merged networking-libraries in context of the renaming
- config-handler was internally restructured for the new generator of the config-documentation
- registration of fields in API-endpoints was updated for cleaner code
- Button in dashboard for the geneartion of the Documentation was removed and is now done by a new
    CLI flag.
- merged entire code into one single docker-image
- better segmentation of the synapses

### Fixed

- solved all compiler-warnings (disabled libraries not included)
- fixed stupid memory leaks in API and task-handling
- fixed handling in database-requests to separate correctly between an internal-error and a
    not-found to give the correct HTTP response to the user
- after a restart of the backend, all clusters are not removed from database at the start to avoid
    broken clusters, because cluster are in-memory and don't survive a restart
- positioning of header-texts in dashboard was fixed

### Removed

- Removed OpenCL-kernel for the moment. The CUDA-variant is enough for testing and easier to update.

## v0.2.0

**Release-Date**: 2023-03-15

### Breaking-Changes

#### API-Breaking

- use now custom syntax for cluster- and segment-templates instead of json
- use protobuf-messages to send payload of files
- merge endpoints to create train- and request-tasks for csv- and mnist-files into one endpoint
- creating a task doesn't require the explicit selecting of the data-set type. This is now read from
    the data-set metadata

#### Checkpoint-Breaking

- changed internal data-structure of the core in order to make GPU-support possible without
    race-conditions

### Changed

- **merged all repositories (source-code, documentation, build, deploy) into this main-repository
    here**
- made token-key and registry in helm-chart configurable
- made token expire-time configurable in Misaki
- core-internal renamings of structs and functions
- use dropdown-menu instead of single buttons in table-entries within the dashboard
- use nested namespaces in sakura- and sakura-layer
- changed a huge amount of map-iterators to c++17 style

### Added

- Added experimental GPU-support with CUDA and OpenCL (per default hard in source-code disbabled at
    the moment)
- internal tokens requestable form Misaki
- API-endpoints and database-table to request logs with page-selector
- print audit-logs in dashboard

### Fixed

- fix random breaks of the websocket under high load in the Miko
- fixed memory-corruption in segment-header of the clusters in Kyouko
- solved uninitialized warning in item-buffer
- fixed stupid memory-leak when sending internal messages

### Removed

- Removed additional namespaces of the libraries in the common-layer
- Disabled reduction-process of the neural-networkds for the moment.

## v0.1.0

**Release-Date**: 2022-10-18

### Added

- First experimental prototype version with **very basic implementation** of following features:
    - first incomplete implementation of the core-concept of a dynamic network with:
        - creates connections while learning
        - has nearly no upper limit for inputs
    - capable to train MNIST-Datasets and basic CSV-files
    - creating neuronal networks based on templates in json-format
    - authorization with username + password and JWT-token
    - creating multiple user and multiple projects in one system
    - basic role-system
    - automatic creation of REST-API-documentation in PDF-, RST- or Markdown-format
    - creating checkpoints of neuronal networks and restore them again
    - thread-binding for all components for more optimal usage of the CPU
    - PoC to change CPU-frequency based on load
    - measuring of power consumption, thermal production and frequency of the CPU
    - Dashboard with HTML, Javasript and CSS with rendering on client-side
    - sdk-library in C++ and Javascript (incomplete)
    - Build-scripts to build docker-images for all components
    - basic Helm-chart to deploy all components on a kubernetes

### Repositories

| Repository-Name                  | Version-Tag | Download-Path                                                    |
| -------------------------------- | ----------- | ---------------------------------------------------------------- |
| Sakura-AI-Documentation          | v0.1.0      | https://github.com/kitsudaiki/Sakura-AI-Documentation.git        |
| Sakura-AI-Dockerbuilder          | v0.1.0      | https://github.com/kitsudaiki/Sakura-AI-Dockerbuilder.git        |
| Sakura-AI-Dashboard              | v0.2.0      | https://github.com/kitsudaiki/Sakura-AI-Dashboard.git            |
| Sakura-AI-Dashboard-Dependencies | v0.1.0      | https://github.com/kitsudaiki/anamiAI-Dashboard-Dependencies.git |
| Sakura-AI-K8s                    | v0.1.0      | https://github.com/kitsudaiki/Sakura-AI-K8s.git                  |
| libSakuraAiSdk                   | v0.4.0      | https://github.com/kitsudaiki/libSakuraAiSdk.git                 |
| KyoukoMind                       | v0.9.1      | https://github.com/kitsudaiki/KyoukoMind.git                     |
| MisakiGuard                      | v0.3.0      | https://github.com/kitsudaiki/MisakiGuard.git                    |
| ShioriArchive                    | v0.4.0      | https://github.com/kitsudaiki/ShioriArchive.git                  |
| AzukiHeart                       | v0.3.1      | https://github.com/kitsudaiki/AzukiHeart.git                     |
| ToriiGateway                     | v0.7.0      | https://github.com/kitsudaiki/ToriiGateway.git                   |
| TsugumiTester                    | v0.4.0      | https://github.com/kitsudaiki/TsugumiTester.git                  |
| libAzukiHeart                    | v0.3.0      | https://github.com/kitsudaiki/libAzukiHeart.git                  |
| libMisakiGuard                   | v0.2.0      | https://github.com/kitsudaiki/libMisakiGuard.git                 |
| ibShioriArchive                  | v0.3.0      | https://github.com/kitsudaiki/ibShioriArchive.git                |
| libKitsunemimiSakuraMessages     | v0.1.0      | https://github.com/kitsudaiki/libKitsunemimiSakuraMessages.git   |
| libKitsunemimiSakuraNetwork      | v0.5.0      | https://github.com/kitsudaiki/libKitsunemimiSakuraNetwork.git    |
| libKitsunemimiSakuraPolicies     | v0.2.0      | https://github.com/kitsudaiki/libKitsunemimiSakuraPolicies.git   |
| libKitsunemimiSakuraEndpoints    | v0.2.0      | https://github.com/kitsudaiki/libKitsunemimiSakuraEndpoints.git  |
| libKitsunemimiSakuraDatabase     | v0.4.0      | https://github.com/kitsudaiki/libKitsunemimiSakuraDatabase.git   |
| libKitsunemimiSakuraCommon       | v0.3.0      | https://github.com/kitsudaiki/libKitsunemimiSakuraCommon.git     |
| libKitsunemimiSakuraNetwork      | v0.9.0      | https://github.com/kitsudaiki/libKitsunemimiSakuraNetwork.git    |
| libKitsunemimiSakuraLang         | v0.13.0     | https://github.com/kitsudaiki/libKitsunemimiSakuraLang.git       |
| libKitsunemimiSakuraDatabase     | v0.6.1      | https://github.com/kitsudaiki/libKitsunemimiSakuraDatabase.git   |
| libKitsunemimiSakuraHardware     | v0.2.0      | https://github.com/kitsudaiki/libKitsunemimiSakuraHardware.git   |
| libKitsunemimiArgs               | v0.5.0      | https://github.com/kitsudaiki/libKitsunemimiArgs.git             |
| libKitsunemimiConfig             | v0.5.0      | https://github.com/kitsudaiki/libKitsunemimiConfig.git           |
| libKitsunemimiCrypto             | v0.3.0      | https://github.com/kitsudaiki/libKitsunemimiCrypto.git           |
| libKitsunemimiCpu                | v0.4.1      | https://github.com/kitsudaiki/libKitsunemimiCpu.git              |
| libKitsunemimiJwt                | v0.5.1      | https://github.com/kitsudaiki/libKitsunemimiJwt.git              |
| libKitsunemimiSqlite             | v0.4.0      | https://github.com/kitsudaiki/libKitsunemimiSqlite.git           |
| libKitsunemimiJson               | v0.12.0     | https://github.com/kitsudaiki/libKitsunemimiJson.git             |
| libKitsunemimiJinja2             | v0.10.0     | https://github.com/kitsudaiki/libKitsunemimiJinja2.git           |
| libKitsunemimiIni                | v0.6.0      | https://github.com/kitsudaiki/libKitsunemimiIni.git              |
| libKitsunemimiNetwork            | v0.9.0      | https://github.com/kitsudaiki/libKitsunemimiNetwork.git          |
| libKitsunemimiCommon             | v0.27.1     | https://github.com/kitsudaiki/libKitsunemimiCommon.git           |
