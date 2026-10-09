# Readme

## setup_vagrant_stack.sh

### Purpose

Starts the setup of `testing/vagrant`: nine virtual machines with nested virtualization and a
kubernetes-cluster (k3s), on which ansible installs the operator of `deploy/operator`, which deploys
the stack of the `Ainari`-resource [ainari.yaml](ainari.yaml). Miko,
hanami, ryokan and omamori run with one replica on each of the three management-machines and share
the mysql-server on the machine `ainari-mysql`. The nix-based images, the same as the ones of the
CI, are built with [build_local_images.sh](build_local_images.sh) on the host and copied into the
virtual machines.
The host reaches the floating ip-addresses over a route towards the virtual machine `ainari-torii`.
See [Vagrant setup](../../docs/developer/local_testing/vagrant_setup.md) for the details of the
setup.

### Usage

```bash
make up vagrant      # or ./testing/vagrant/setup_vagrant_stack.sh
make down vagrant    # or ./testing/vagrant/setup_vagrant_stack.sh --down
```

`make up vagrant` installs ansible into a virtual environment in `temporary_files`, if it is not
installed.

### Limitations

- `docker`, `vagrant` with the plugin `vagrant-libvirt`, libvirt, `ansible-playbook` and `openssl`
  have to be installed and the host needs nested virtualization.
- The nine virtual machines need about 34 GiB free memory and 40 GiB free disk on the host, see
  [Vagrant setup](../../docs/developer/local_testing/vagrant_setup.md).
- Adding the route needs root, so the script asks for the password of sudo.
- The floating ip-addresses `10.0.0.0/24` and the private network `192.168.56.0/24` of vagrant must
  not be used by any other interface of the host, so the setup can't run together with the other
  local setups.
- The first start takes long, because the virtual machines are installed and the images are copied
  into all of them.
- `--down` destroys the virtual machines, but keeps the saved images and the CA in
  `temporary_files/vagrant`.

## create_local_ca.sh

### Purpose

Creates the CA of a local setup, which signs the certificates of all components of the setup, if
it doesn't exist yet. The CA stays the same over all runs, so it only has to be added to the
trust-store of the host once (see
[The CA of the kind- and the vagrant-setup](../../docs/developer/local_testing/https_ca.md)).
Its name-constraints limit it to the names of the setup, so it can't be misused for any other
host, even if its key leaks. [setup_kind_stack.sh](../kind/setup_kind_stack.sh) and
[setup_vagrant_stack.sh](setup_vagrant_stack.sh) call this script, so it is normally not called
directly. Both setups have their own copy of the script next to their setup-script.

### Usage

```bash
./testing/kind/create_local_ca.sh <CERT_FILE> <KEY_FILE> <COMMON_NAME> <PERMITTED_NAMES>
```

!!! example

    ```bash
    ./testing/kind/create_local_ca.sh temporary_files/kind/ainari-kind-ca.crt \
        temporary_files/kind/ainari-kind-ca.key \
        "ainari kind-setup CA" \
        "permitted;IP:127.0.0.1/255.255.255.255,permitted;DNS:localhost,permitted;DNS:cluster.local"
    ```

### Limitations

- An existing CA is kept, as long as its common-name, its name-constraints and its key match. If
  one of them is different, the CA is replaced and the new one has to be added to the trust-store
  of the host again. The script prints a warning in this case.
- Only the entries `permitted;` and `excluded;` of the name-constraints are compared.
- The key is stored unencrypted. It is only readable by its owner, but should never be used
  outside of a local setup.
- The CA is valid for 10 years and has no revocation.
- `openssl` has to be installed on the host.

## build_local_images.sh

### Purpose

Builds the images of all components for the kubernetes-based local setups (kind and vagrant) with
the tag `local`, for example `ainari/miko:local`. They are built without docker compose and with
the faster `local`-profile of cargo. The argument selects the variant of the Dockerfiles (see
[Packages of the docker-images](../../docs/developer/docker_images.md)):

- `debian`: the Dockerfiles of `dockerfiles/debian_based`, which are easier to debug. They are the
  same images as the ones of the docker-compose setup.
  [setup_kind_stack.sh](../kind/setup_kind_stack.sh) uses them.
- `nix`: the Dockerfiles of `dockerfiles/nix_based`, which are the same as the ones of the CI.
  [setup_vagrant_stack.sh](setup_vagrant_stack.sh) uses them.

Both setups call this script before every start, so it only has to be called directly to rebuild
the images without restarting the setup. Both setups have their own copy of the script next to
their setup-script.

### Usage

```bash
./testing/kind/build_local_images.sh debian    # for the kind-setup
./testing/vagrant/build_local_images.sh nix    # for the vagrant-setup
```

The id of the group of `/dev/kvm`, which sakura is built with, can be given with `KVM_GID`. It
defaults to the one of the host:

```bash
KVM_GID=108 ./testing/kind/build_local_images.sh debian
```

### Limitations

- The `local`-profile is optimized for the build-time and not for the runtime, so the images are
  only meant for testing and never for a real deployment.
- The images are only built for the platform of the host.
- The images are not pushed anywhere, the setups load them into their cluster or virtual machines
  themselves.