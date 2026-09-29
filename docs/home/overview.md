# Overview

Ainari consist of a micro-service architecture. The overview of the current state of the setup is
shown and described below:

![Overview](/img/ainari_overview.jpg)

## Core-Components

### Miko

Miko is the most trustworthy component of the setup and provides the authentication service. It
provides user-, project- and quota-management. Whenever a user want to interact with the backend, he
must at first login in his user-account over Miko, which returns a JWT-token. This this token the
user can also access all other component, except the Onsen. Each components checks the provided
JWT-tokens against Miko, to check if the token is valid.

!!! info

    At the moment the project-management is basically non-existing even the project-db-table and
    endpoints are present. Will be fixed in the future. Also RBAC roles will be handled later by
    Miko too.

### Omamori

Omamori is the protection component and is basically a key-manager. It provides basic functionality
to upload, generate and download keys.

!!! info

    Current only a simple crypto process is done by omamori, where uploaded and generated keys are
    encrypted by a key from the config and stored encrypted within the database. Will be updated by
    a Vault-connection and other backends in the future.

### Onsen

The Onsen is the storage-pool all images and snapshots from the Ryokan and Sakura are stored
here. This component is/must not be accessible from the internet. Its also doesn't check JWT-tokens,
because it doesn't contain a REST-API. It interacts with the Ryokan and Sakura over a
grpc-connection and protobuffer-messages. In the kubernetes-setup these 2 connections are secured by
a wireguard-tunnel.

!!! info

    In the kubernetes-installation only one Onsen-host is currently supported, because of the
    wireguard-config. Will be fixed in the future.

### Ryokan

Ryokan manage the Onsen-hosts. Images for new virtual machines
can be uploaded, and will be converted and against a key from Omamori encrypted before the files are
placed in the Onsen.

### Sakura

Sakura is the compute hosts of this IaaS stack. It manage the cloud hypervisor VMs.

!!! info

    Migration of model between Sakura-hosts is currently not possible. Will come later.

### Hanami

Hanami manage the Sakura-hosts. Whenever a new virtual machine is requested by the user, this request goes
against Hanami, which selects the Sakura-host of the new model and configures the Torii for the
new connection. Also the handling of networks and floating ips is done by Hanami.

### Torii

Torii is the gateway-component, is configured by hanami and handles 2 differen pathways:

1. It is basically only a layer-3-proxy. Torii creates a port for each virtual machine and the user
   can directly interact with the Sakura compute host, which holds the VM. Torii doesn't terminate
   the HTTPS-connection between the user and the Sakura-host, which basically provides an end-to-end
   encrypted connection for all user interactions with the control endpoints for the VM. The path
   was added to the setup instead of direct access to the compute host, to make later added
   migration of virtual machines easier for the user to handle.

2. Torii also provides the networks between the virtual machines and from the virtual machines to
   the internet. There is a public Torii as gateway to the outside and one Torii on each Sakura
   Host.

## other

### Dashboard

See [Dashboard docu](/user/dashboard/dashboard/)

### Python-SDK

See [Python SDK docu](/user/cli_sdk/cli_sdk_docu/#__tabbed_2_2)

### CLI

See [CLI docu](/user/cli_sdk/cli_sdk_docu/)
