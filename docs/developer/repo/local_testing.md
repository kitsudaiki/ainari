# Local testing

## Testing in multi-node kubernetes

In the repository there is a Vagrantfile with connected minimalistic playbook, which creates 3
locally hosted libvirt virtual machines, installes a k3s-kubernetes within these instances,
configures and installes ainari in there and runs the sdk-api-test against the setup. This
test-setup doesn't require docker-hub. At the beginning of the setup it build docker-images from the
local source code and pushes these local images direcly into the k3s-kubernetes without any
additional registry and also uses the local helm chart for the deployment within the kubernetes.
That way the code, dockerfiles, helm-charts and sdk-lib are tested in single automated workflow in a
virtual multi-node environment.

### Minimal Requirements

- **CPU**: 8 threads (better 16 threads to avoid cpu-overcommit)
- **Memory**: 14 GiB

### Installation

This installation uses Vagrant with libvirt as provider to deploy the virtual machines.

- Install apt-packages necessary for libvirt and the libvirt-provider

    ```bash
    sudo apt update
    sudo apt install -y \
        qemu-kvm \
        libvirt-daemon-system \
        libvirt-clients \
        virtinst \
        bridge-utils \
        cpu-checker
        build-essential \
        ruby-dev \
        pkg-config \
        libvirt-dev \
        libxml2-dev \
        libxslt-dev \
        zlib1g-dev \
        rsync
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

- Update `/etc/hosts` by adding the content

    ```plain
    192.168.56.10  local-vagrant-hanami
    192.168.56.10  local-vagrant-miko
    192.168.56.10  local-vagrant-ryokan
    192.168.56.10  local-vagrant-sakura
    192.168.56.10  local-vagrant-torii
    192.168.56.10  local-vagrant-omamori
    192.168.56.10  local-vagrant-onsen
    192.168.56.10  local-vagrant-ainari
    ```

- Install local ansible required to execute the playbook

    ```bash
    python3 -m venv venv
    source venv/bin/activate
    pip3 install ansible
    ```

### Usage

#### Vagrant-actions:

- create a complete new test-installation

    `vagrant up`

- in case a run failed and you want to run it again or with updated playbooks against the same
    already existing vagrant environment

    `vagrant provision`

- ssh into server of the virtual machines

    `vagrant ssh server`

    with this you will enter the machine with the kubernetes api. It is only a minimalistic shell, so
    run `/bin/bash` at first in there to get a real bash shell. There you can also run `kubectl` and
    `helm` commands.

- delete previous vagrant environment

    `vagrant destroy -f`

#### access the dashboard

After a finished `vagrant up`, enter the address `https://local-vagrant-ainari` into your browser
and you should see the dashboard

Login:

- **user**: `asdf`
- **password**: `asdfasdf`
