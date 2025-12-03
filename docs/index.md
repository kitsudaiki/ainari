!!! info

    This is the documentation of the develop-branch.

# Ainari

<p align="center">
  <img src="img/ainari-logo-with-text.jpg" width="1500" height="700" />
</p>

!!! danger "IMPORTANT"

    # **IMPORTANT: This project is still an experimental prototype and NOT ready for any productive usage. There is still a lot of evaluation and improvement necessary, but because this is only a spare time project beside a 40h/week job, I have only a very limited amount of time available to work on it.**

!!! info "WIP"

    # **The docutionation is currently in a bigger update-process, so there are still many broken links and other outdated stuff.**

## Intro

Ainari contains in its core a custom experimental artificial neural network, which can work on
unnormalized and unfiltered input-data, like sensor measurement data. The network growth over time
by creating new nodes and connections between the nodes while learning new data. The original
concept was created by myself, merged with classical deep-learning and the code was written from
scratch without any frameworks. The goal behind Ainari is to create something unique, which works
more like the human brain. It wasn't targeted to get a higher accuracy than classical artificial
neural networks like Tensorflow, but to be more flexible and easier to use and more efficient in
resource-consumption for big amounts of inputs and users. Additionally it also provides an
as-a-Service architecture within a cloud native environment and multi-tenancy.

## Current experimal and prototypically implemented features:

- **Growing neural network**:

    The artificial neural network, which is the core of the project, growth over time while learning
    new things by creating new nodes and connections between the nodes based on the given input. A
    resize of the network is also quite linear in complexity.

- **No normalization of input**

    The input of the network is not restricted to range of 0.0 - 1.0 . Every value can be inserted,
    even negative values. Also if there is a single broken value in the input-data, which is million
    times higher, than the rest of the input-values, it has nearly no effect on the rest of the
    already trained data.

- **No strict layer structure**

    The base of a new neural network is defined by a cluster-template. In these templates the
    structure of the network in planed in hexagons, indeed of layer. When a node tries to create a
    new synapse, the location of the target-node depends on the location of the source-node within
    these hexagons. The target is random and the probability depends on the distance to the source.
    This way it is possible to break the static layer structure. But when defining a line of
    hexagons and allow nodes only to connect to the nodes of the next hexagon, a classical
    layer-structure can still be enforced.

    See [short explanation](/developer/inner_workings/core/core/#no-strict-layer-structure) and
    [measurement-examples](/developer/inner_workings/measurements/measurements)

- **Spiking neural network**

    The concept also supports a special version of working as a spiking neural network. This is
    optional for a created network and basically has the result, that an input is impacted by an
    older input, based on the time how long ago this input happened.

    See [short explanation](/developer/inner_workings/core/core/#spiking-neural-network) and
    [measurement-examples](/developer/inner_workings/measurements/measurements)

- **3-dimensional networks**

    It is basically possible to define 3-dimensional networks. This was only added, because the human
    brain is also a 3D-object. This feature exist in the
    [cluster-templates](/user/cluster_templates/cluster_template/), but was never tested until now.
    Maybe in bigger tests in the future this feature could become useful to better mix information
    with each other.

## Further characteristics:

- **Rust as programming language for the backend without unsafe**

    Even the project started with C++ as primary programming language until v0.7.0, the whole backend
    is now written in Rust without unsafe code and use `#![forbid(unsafe_code)]` to prevent the
    usage of unsafe. Based on `cargo geiger` many used dependencies sadly still use much unsafe
    code, but at least in this repository here no unsafe code is added.

- **Parallelism**

    The processing structure works also for multiple threads, which can work at the same time on the
    same network. (GPU-support with CUDA is disabled at the moment for various reasons).

- **Generated OpenAPI-Documentation**

    The OpenAPI-documentation is generated directly from the code. So changing the settings of a
    single endpoint in the code automatically results in changes of the resulting documentation, to
    make sure, that code and documentation are in sync.

- **Multi-user and multi-project**

    The projects supports multiple user and multiple projects with different roles (member,
    project-admin and admin) and also managing the access to single api-endpoints via policy-file.
    Each user can login by username and passphrase and gets an JWT-token to access the user- and
    project-specific resources.

    See [Authorization-docu](/developer/inner_workings/user_and_projects/)

- **Efficient resource-usage**

    1. The concept of the neural network results in the effect, that only necessary synapses of an
        active node of the network is processed, based on the input. So if only very few input-nodes
        get data pushed in, there is less processing-time necessary to process the network.

    1. Because of the multi-user support, multiple networks of multiple users can be processed on the
        same physical host and share the RAM, CPU-cores and even the GPU, without splitting them via
        virtual machines or vCPUs.

    <!-- Disabled this entry because it is not part of the current state of the implementation
    1. Capability to regulate the cpu-frequencey and measure power-consumption. (disabled currently)
    See [Monitoring-docu](/inner_workings/monitoring/monitoring/#controlling-cpu-frequency) -->

- **Network-input**

    Interaction with the network by direct synchronous single requests or with asynchronous task in a
    task-queue.

- **Installation on Kubernetes**

    The backend can be basically deployed on kubernetes via Helm-chart.

    See [Installation-docu](/deployer/installation/kubernetes_installation/)

## Overview

Ainari is split into a micro-service architecture. See here for
[Overview-Description](/home/overview/)

![Overview](home/ainari_overview.drawio)

## Summary important links

<div class="grid cards" markdown>

- :material-clock-fast:{ .lg .middle } **Getting started**

    ______________________________________________________________________

    [:octicons-arrow-right-24: Example-Workflow](/user/cli_sdk/example_workflow/)

    [:octicons-arrow-right-24: Installation-Guide](/deployer/installation/kubernetes_installation/)

    [:octicons-arrow-right-24: SDK and CLI documentation](/user/cli_sdk/cli_sdk_docu/)

    [:octicons-arrow-right-24: OpenAPI documentation](/user/rest_api/rest_api_docu_sakura/)

- :octicons-codespaces-24:{ .lg .middle } **Development**

    ______________________________________________________________________

    [:octicons-arrow-right-24: How to build](/developer/repo/build_guide/)

    [:octicons-arrow-right-24: Development-Guide](/developer/repo/development/)

    [:octicons-arrow-right-24: Dependency-Overview](/developer/repo/dependencies/)

- :octicons-package-24:{ .lg .middle } **Pre-build objects**

    ______________________________________________________________________

    All objects are automatically build and uploaded by the
    [CI-pipeline](https://github.com/kitsudaiki/ainari/actions/workflows/build_test.yml) for each
    merge on `develop`-branch and for each tag.

    [:octicons-arrow-right-24: Docker-images](https://hub.docker.com/u/kitsudaiki)

    [:octicons-arrow-right-24: client, SDK and helm-chart](https://files.ainari.cloud/)

- :octicons-milestone-24:{ .lg .middle } **Roadmap**

    ______________________________________________________________________

    [:octicons-arrow-right-24: Roadmap](/home/ROADMAP/)

</div>

## Currently disabled features

There are some features, which existed in the past, were disabled temporary and will be
added/enabled again in the near future:

1. Dashboard

    As a PoC a first dashboard was created, without any framework. At the moment it is in process of
    refactoring and re-implementation in Vue.js and Typescript.

1. Regulation of CPU-speed

    Also in older version there also was the function available to regulate the speed of the CPU
    based on the workload. The dashboard was used to visualize the CPU metrics like the speed.
    Since the dashboard was disabled, there is at the moment not feedback available, so for
    usability reasons the feature was not further maintained and disabled for now.

1. GPU-support

    There already were some attempts in the past to add GPU-support with CUDA and OpenCL in the past.
    Some version like 0.4.0 also had a working version implemented. The problem was disappointing
    performance and some restrictions for the CPU-version too. There will be some further attempts
    in the future, to fix this issue and bring GPU support back into the project, but because there
    is no definite solution now, it is unknown when this happens.

1. Role-based policies

    Until 0.7.0 there were policies and roles, which were removed for the moment, because they were
    not translated into the new Rust code so far. Will be added in one of the next releases again.

## Author

**Tobias Anker**

eMail: tobias.anker@kitsunemimi.moe

## License

The complete project is under
[Apache 2 license](https://github.com/kitsudaiki/ainari/blob/developer/LICENSE).
