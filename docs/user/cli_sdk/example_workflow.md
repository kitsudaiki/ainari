# Example-Workflow

This chapter shows an example workflow for the current state of the project with the CLI-client:
a virtual machine is created from an ubuntu-cloud-image and accessed over ssh through a floating
IP. The same workflow is also used for automated testing within the project with the python-SDK
(`testing/ainari_test/vm_lifecycle_test.py`). See for further information the
[CLI and SDK documentation](cli_sdk_docu.md).

## Preparation

- install Ainari based on the [Installation-guide](../../deployer/installation/kubernetes_installation.md)

- Download the pre-build binary of the CLI-client from the [file-share](https://files.ainari.cloud/)

- Export env-variables for connect and login-information:

    ```bash
    export AINARI_ADDRESS=<ADDRESS_OF_MIKO>
    export AINARI_USER=<USER_ID>
    export AINARI_PASSPHRASE=<USER_PASSPHRASE>
    ```

    !!! example

        ```bash
        export AINARI_ADDRESS=https://local-miko
        export AINARI_USER=asdf
        export AINARI_PASSPHRASE=asdfasdf
        ```

    !!! info

        If the CA of the installation is not trusted by the system, the `--insecure`-flag has to be
        added to all following `ainarictl`-commands, to skip the tls-check.

## Example

1. **Check the hosts** (admin only)

    At least one sakura-host has to be registered, before a virtual machine can be created:

    ```bash
    ./ainarictl host list
    ```

1. **Upload a public key**

    Create a ssh-key-pair and upload the public key. It is deployed into the virtual machine for the
    user of the image.

    ```bash
    ssh-keygen -t ed25519 -N "" -f ./ainari_key

    ./ainarictl public_key upload -k ./ainari_key.pub my-key
    ```

1. **Upload an image**

    Download an ubuntu-cloud-image and upload it as boot-disk for virtual machines:

    ```bash
    wget https://cloud-images.ubuntu.com/noble/current/noble-server-cloudimg-amd64.img

    ./ainarictl image create disk -i ./noble-server-cloudimg-amd64.img ubuntu-noble
    ```

    The upload takes a while.

1. **Create a network**

    ```bash
    ./ainarictl network create -s 192.168.100.1/24 my-network
    ```

1. **Create a virtual machine**

    With 2 cores, 2048 MiB memory and a disk of 10 GiB. The UUIDs are the ones of the previous
    steps:

    ```bash
    ./ainarictl vm create -c 2 -m 2048 -d 10 \
        -u <NETWORK_UUID> \
        -i <IMAGE_UUID> \
        -k <PUBLIC_KEY_UUID> \
        my-vm
    ```

    The virtual machine is created in the background. Repeat the following command, until
    `vm_state` is `RUNNING`:

    ```bash
    ./ainarictl vm get <VIRTUAL_MACHINE_UUID>
    ```

1. **Add a floating IP**

    Reserve a free floating IP and attach it directly to the virtual machine:

    ```bash
    ./ainarictl floating_ip add -n my-vm-fip -v <VIRTUAL_MACHINE_UUID>
    ```

    Alternatively reserve it first and attach it afterwards:

    ```bash
    ./ainarictl floating_ip add -n my-vm-fip
    ./ainarictl floating_ip attach <FLOATING_IP_UUID> <VIRTUAL_MACHINE_UUID>
    ```

1. **Log in over ssh**

    The floating IP is reachable from the network behind the uplink of the gateway. The virtual
    machine needs a moment to boot, before it answers on ssh.

    ```bash
    ssh -i ./ainari_key ubuntu@<FLOATING_IP>
    ```
