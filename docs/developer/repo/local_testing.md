# Local testing

There are three setups, which run the whole stack locally, with the same topology and the same
end-to-end test. They are described in detail in `testing/local_stack/Readme.md`.

| Setup   | Start / stop                            | Runs on                                                 |
| ------- | --------------------------------------- | ------------------------------------------------------- |
| local   | `make up local` / `make down local`     | docker compose on the host                              |
| kind    | `make up kind` / `make down kind`       | a kind-cluster on the host, with the helm-chart         |
| vagrant | `make up vagrant` / `make down vagrant` | a k3s-cluster of four virtual machines, with the helm-chart |

All of them build the images from the local source code, so no registry is required, and the
kind- and vagrant-setup deploy the local helm-chart of `deploy/k8s/ainari`. That way the code, the
dockerfiles, the helm-chart and the sdk are tested in a single workflow.

## Testing in multi-node kubernetes

The vagrant-setup (`testing/vagrant`) creates four libvirt virtual machines with nested
virtualization: one for the control-components, one for the gateway at the edge of the network and
two sakura-hosts, which really boot the virtual machines of ainari. Ansible installs k3s within
them and deploys the helm-chart.

### Minimal Requirements

- **CPU**: 8 threads (better 16 threads to avoid cpu-overcommit)
- **Memory**: 20 GiB for the virtual machines
- **Disk**: about 25 GiB (the virtual machines grow to about 20 GiB with a few virtual machines of
  ainari, plus the images)
- nested virtualization of kvm (`/sys/module/kvm_intel/parameters/nested` or
  `/sys/module/kvm_amd/parameters/nested` is `Y` or `1`)

### Installation of vagrant with libvirt

- Install apt-packages necessary for libvirt and the libvirt-provider

    ```bash
    sudo apt update
    sudo apt install -y \
        qemu-kvm \
        libvirt-daemon-system \
        libvirt-clients \
        virtinst \
        bridge-utils \
        cpu-checker \
        build-essential \
        ruby-dev \
        pkg-config \
        libvirt-dev \
        libxml2-dev \
        libxslt-dev \
        zlib1g-dev
    ```

- Enable and start libvirt:

    ```bash
    sudo systemctl enable --now libvirtd
    ```

- So you don’t need sudo every time

    ```bash
    sudo usermod -aG libvirt,kvm $USER
    ```

    ( after this you have to logout and login again )

- Install the libvirt provider plugin

    ```bash
    vagrant plugin install vagrant-libvirt
    ```

Ansible doesn't have to be installed: `make up vagrant` installs it into a virtual environment in
`temporary_files`, if it is missing.

### Usage

See the section *Vagrant setup* of `testing/local_stack/Readme.md`. The dashboard is reachable at
`https://192.168.56.10:11422` with the user `asdf` and the password `asdfasdf`.
