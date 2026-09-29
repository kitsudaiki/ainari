# Features

This page provides an overview of the important features of the Ainari IaaS stack, outside of the
base capabilities of uploading resources and creating a virtual machine based on these.

!!! warning

    **IMPORTANT: even the implemented features are mostly currently just in a prototypical state.**
    **They work, but are not ready for productive usage yet.**

!!! info

    All implementation version stamps starting at least with v0.20.0, even when they were already
    implemented in older version. But v0.20.0 was a reboot of the project, so the older versions
    are not relevant in this overview here.

## Security

### Encrypted Images

**Status: <span style="color:#4caf50">implemented</span> (since v0.20.0)**

All uploaded images are always encrypted with AES-256-GCM before storing them within the storage
backend. So even if the Storage Data Plane is compromised, the data within the images are safe and
can not be stolen or manipulated.

### Encrypted Snapshots

**Status: <span style="color:#4caf50">implemented</span> (since v0.20.0)**

Snapshots of root disks of virtual machines are handled as images again and so they are also always
AES-256-GCM encrypted before moving them from the compute host to the storage backend.

### Encrypted tenant networks

**Status: <span style="color:#e6b800">partially implemented</span>**

!!! info

    Implemented in the network stack, but key-exchange not added yet

Network traffic between virtual machines can be encrypted with IPsec and AES-256-GCM by using XFRM.
It is planned to use MLS with the [OpenMLS-library](https://github.com/openmls/openmls) for the
key-exchange.

### Encrypted volumes

**Status: <span style="color:#e53935">planned</span>**

!!! info

    There are no remote mounted volumes in general, so there is nothing to encrypt here currently

Remote mounted volumes will be encrypted as LUKS format with AES-256-XTS encryption.

### Filter for network traffic

**Status: <span style="color:#e6b800">partially implemented</span>**

!!! info

    Implemented in the network stack, but not provided to the API at the moment

Network rules can be set, to limit incoming and outgoing network traffic to specific ports and
ip-addresses.

### "dumb" compute hosts

**Status: <span style="color:#4caf50">implemented</span> (since v0.20.0)**

Every action on the compute-hosts is done with the context and token of the user, who triggered the
action. In contrast to Openstack, the compute hosts have no own user with admin permissions or
MessageQueue or Database credentials in their config-files. The only keys in the config are an
API-key, to allow the access of non-public API endpoints, which still require an additional access
token, and a key to allow them to register themselves at the control-plane. No more. The second one
will also be changed into a host-specific certificate, to limit it even further. Besides this, the
compute hosts have their own local SQLite database only. So even if an attacker gets control over a
compute host (for example by breaking out of a virtual machine), the information on the host is not
sufficient to take over the entire deployment. Without additional tokens of other users, the 2 keys
in the config of the compute host are not enough to steal other information outside of the
compute-host.

### Project host isolation

**Status: <span style="color:#e53935">planned</span>**

Compute hosts can optionally be allocated for a specific project, so only virtual machines of this
specific project can be scheduled on this host. That way you know, who else is located on the same
hardware as you. In this case an attacker, who tries to steal your data by breaking out of the
virtual machine, has to be in the same project as you, to get the chance to be on the same compute
host.

### Bring your own key

**Status: <span style="color:#e6b800">partially implemented</span>**

!!! info

    Currently it is possible to upload a key, but the implementation to use it is still not done.

Instead of letting the backend generate a key, users can upload their own keys to the key management
and use them, in case they do not trust the random function of the backend.

### Hold your own key

**Status: <span style="color:#e53935">planned</span>**

Users can hold their keys themselves, instead of using the key management in the backend. In case of
an action on a virtual machine, like creating and restoring an encrypted snapshot, the user can
provide the key directly themselves over an end-to-end encrypted path. In case of the dashboard, the
user gets a popup to enter the keys, whenever required by the backend. The keys are then only stored
in the memory of the specific compute host and only as long as necessary for the task.

### 2-factor authentication

**Status: <span style="color:#e53935">planned</span>**

To make logins much more secure, a second factor like OTP can be used as second login factor.

### Yubikey support for key management

**Status: <span style="color:#e53935">planned</span>**

As minimal HSM solution, [YubiKey](https://www.yubico.com/der-yubikey/?lang=de) can be used for
hardware encryption for the key management of the stack.

### AMD SEV SNP support

**Status: <span style="color:#e53935">planned</span>**

!!! info

    Even though the current cloud hypervisor already supports it, it cannot be tested currently
    without the necessary hardware :(

To encrypt the memory of the virtual machine itself, AMD SEV SNP can be enabled for virtual
machines. That way, even a snapshot of the memory of the compute host contains only encrypted user
information. Besides this, it provides attestation features.

### Backend in Rust

**Status: <span style="color:#4caf50">Current state</span>**

The entire backend is written in Rust. Besides the low-level network layer, the code avoids `unsafe`
marked code too.

### Using Cloud-Hypervisor

**Status: <span style="color:#4caf50">implemented</span> (since v0.20.0)**

To manage the virtual machine, the
[Cloud-Hypervisor](https://github.com/cloud-hypervisor/cloud-hypervisor) is used. In contrast to
qemu it has a minimal code base and is written in Rust.

### Offline deployable

**Status: <span style="color:#e53935">planned</span>**

To be able to run in air gapped environments, the whole deploy process will be done by an offline
mirror.

### Optional four-eyes principle for projects

**Status: <span style="color:#e53935">planned</span>**

Projects can be marked as 4-eyes principle necessary. If this is enabled, every action triggered by
a user against a virtual machine, must be approved by another user of the same project, in order to
be started.

### Audit-Log

**Status: <span style="color:#e6b800">partially implemented</span>**

!!! info

    For the API: Middleware already exists, but there is no target to send the information at the
    moment.

Any API access by any user is handled by a middleware in front of each API. Every REST-API request
must go through this middleware. The access information is then sent to a central log service, where
it is tracked who accessed which endpoint at which point in time.

Besides this, each action on a virtual machine on the compute hosts is handled by a task. The task
log is stored on the compute host and can be checked by any user of the virtual machine. In this
task log it is visible which action (create, snapshot create, snapshot restore, ...) is done by
which user, at which point in time and if this task was successful. This log is visible by the user,
as long as the virtual machine lives.

### SBOM

**Status: <span style="color:#e53935">planned</span>**

An automatic summary of all packages with versions used by the entire stack, to keep track of CVEs
and which versions are affected by them.

## Other

### OpenAPI specs generated from source-code

**Status: <span style="color:#4caf50">implemented</span> (since v0.20.0)**

OpenAPI specs are generated out of the source-code with the help of the
[apistos](https://github.com/netwo-io/apistos) crates. This ensures, that the API documentation is
always up-to-date.

### Own network stack with eBPF

**Status: <span style="color:#4caf50">implemented</span> (since v0.20.0)**

The stack doesn't rely on third party software for its software-defined network (SDN), like OVN, OVS
or Linux-Bridges. The only Linux network element used are tap-devices, to connect the virtual
machines with the network layer. The network layer is written entirely in Rust, which runs in the
kernel space with the help of [eBPF](https://ebpf.io/).

### Built-in central error-logging

**Status: <span style="color:#e53935">planned</span>**

Any internal error is sent to a central error-log, with all information around the error with
user-id and so on, to make problems of users easier to find and debug.

### Deployable on Kubernetes

**Status: <span style="color:#4caf50">implemented</span> (since v0.20.0)**

The whole stack is deployable on Kubernetes by providing helm charts and docker-images. See
[Kubernetes installation-guide](/deployer/installation/kubernetes_installation/)

### Kubernetes Operator

**Status: <span style="color:#e53935">planned</span>**

Besides the helm chart, the complete stack can be deployed by a kubernetes operator on the
kubernetes deployment. So only the operator has to be deployed by the user and the rest is handled
by the operator.

### Easy local development

**Status: <span style="color:#4caf50">Current state</span>**

The whole stack can be deployed and tested on the local workstation and doesn't require strong
hardware, complex deployment or remote debugging. There are 3 different ways to run the complete
stack as [Local deployment](/developer/local_testing/local_testing/). Besides this, the entire stack
can also be deployed and live-debugged with breakpoints in VSCode/VSCodium, as shown in the
[Developer documentation](/developer/development/).

### No heavy message queue

**Status: <span style="color:#4caf50">Current state</span>**

Compared to Openstack, there is no heavy message queue, like RabbitMQ. Instead of this, after a
virtual machine was reserved on a compute-host, the user communicates directly with the compute host
itself. So there is no high traffic through the control plane.

### Monorepo

**Status: <span style="color:#4caf50">Current state</span>**

Monorepos can become quite fast very big, but they have the advantage to be easier to handle. Other 
IaaS-projects, like Openstack and IronCore, have each component and library in a different git repo.
Implementing new features over multiple components and repositories can easily become very frustrating. 
You have multiple merge requests to approve, higher risks of conflicts and so on. Even this project was
at one point in time split over 40 repositories. Dependency updates were a total time consuming pain.
A Monorepo makes implementing new features and updating dependecies much easier. It is also easy to 
identify which versions of the different components belong together. Based on my experiences I made,
a Monorepo makes much more sense for this project, even it contains microservices.