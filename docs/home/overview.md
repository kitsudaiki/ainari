# Overview

Ainari consist of a micro-service architecture. The overview of the current state of the setup is
shown and described below:

![Overview](ainari_overview.drawio)

## Components

There are 7 core-components at the moment, all of them written in Rust.

### Miko

Miko is the most trustworthy component of the setup and provides the authentication service. It
provides user-, project- and quota-management. Whenever a user want to interact with the backend, he
must at first login in his user-account over Miko, which returns a JWT-token. This this token the
user can also access all other component, except the Onsen. Each components checks the provided
JWT-tokens against Miko, to check if the token is valid.

!!! info

    At the moment the project-management is basically non-existing even the project-db-table and
    endpoints are present. Will be fixed in the future. Also RBAC roles will be handled later by Miko
    too.

### Omamori

Omamori is the protection component and is basically a key-manager. It provides basic functionality
to upload, generate and download keys.

!!! info

    Current only a simple crypto process is done by omamori, where uploaded and generated keys are
    encrypted by a key from the config and stored encrypted within the database. Will be updated by a
    Vault-connection and other backends in the future.

### Onsen

The Onsen is the storage-pool all datasets and checkpoints from the Ryokan and Sakura are stored
here. This component is/must not be accessible from the internet. Its also doesn't check JWT-tokens,
because it doesn't contain a REST-API. It interacts with the Ryokan and Sakura over a
grpc-connection and protobuffer-messages. In the kubernetes-setup these 2 connections are secured by
a wireguard-tunnel.

!!! info

    In the kubernetes-installation only one Onsen-host is currently supported, because of the
    wireguard-config. Will be fixed in the future.

### Ryokan

Ryokan manage the Onsen-hosts within the Onsen stored datasets and checkpoints. CSV- and MNIST-files
can be uploaded, and will be converted and against a key from Omamori encrypted before the files are
placed in the Onsen.

### Sakura

Sakura is the core of the project and contains the handling and processing of the artificial neural
networks (cluster). Checkpoints created from cluster are encrypted with an auto-generated key from
Omamori, before placing the data in the Onsen.

!!! info

    Migration of cluster between Sakura-hosts is currently not possible. Will come later.

### Hanami

Hanami manage the Sakura-hosts. Whenever a new cluster is requested by the user, this request goes
against Hanami, which selects the Sakura-host of the new cluster and configures the Torii for the
new connection. Also list and delete networks is done by Hanami.

!!! info

    This scheduling is at the moment only a random selection, but be updated to a real scheduling in the
    future.

### Torii

Torii is the gateway-component. It is basically only a layer-3-proxy, which is configured by Hanami
for each cluster. Torii creates a port for each cluster and the user can directly interact with the
cluster of the hosted Sakura-host over this port. Torii doesn't terminate the HTTPS-connection
between the user and the Sakura-host with the cluster, which basically provides an end-to-end
encrypted connection for all user interactions with the cluster. Torii doesn't have a benefit at the
moment, but was already added to the setup as early as possible to avoid later problems, when
cluster-migration was implemented in Sakura and a proxy is needed to not break user-connections.

!!! info

    Maybe in the future it will be changed to a lower osi-layer, because the current setup also has its
    disadvantages.

## other

### Dashboard

!!! info

    under construction and is written in Vue.js and Typescript.

### Python-SDK

The SDK is written in python, because most data analytic tools are written in python, because most
of the tools and frameworks in this field are python. This should make integration in other
workflows more easily.

### CLI

As alternative option there is a CLI-tool written in Golang.

### Test-scripts

There are a few test-scripts to tests the basic functionality of the python-SDK and the CLI. They
are used in the CI-pipeline for testing.

### Kubernetes-Installation

Beside the development-setup with vscode or a manual deployment, the only deployment-method is
currently a kubernetes installation with a helm-chart.

!!! info

    The kubernetes-setup is currently very basic and still lacks a bunch of security and configration
    updates. An Ansible-installation will come in the future too.
