# CLI-SDK-Docu

The CLI `ainarictl` and the Python-SDK provide functions to interact with the API of Ainari. The
CLI is based on the Go-SDK in `src/sdk/go/ainari_sdk`.

For a complete walk-through from the upload of an image to the ssh-login into a virtual machine,
see the [Example-Workflow](example_workflow.md).

## Installation

=== "CLI"

    Requires `go` in the version of `src/cli/ainarictl/go.mod`.

    ```bash
    git clone https://github.com/kitsudaiki/ainari.git
    cd ainari/src/cli/ainarictl/

    go build .
    ```

    Alternatively the pre-build binary can be downloaded from the
    [file-share](https://files.ainari.cloud/).

=== "Python-SDK"

    ```bash
    git clone https://github.com/kitsudaiki/ainari.git

    # create python-env (optional)
    python3 -m venv ainari_sdk_env
    source ainari_sdk_env/bin/activate

    pip3 install ./ainari/src/sdk/python/ainari_sdk
    ```

## Login

Every action requires a token of the user, which is requested from miko. The token is only valid
for a certain amount of time, based on the configuration of the server. Besides the token, miko
returns the addresses of all other components, so only the address of miko is required.

=== "CLI"

    The address and the login-credentials are set via environment-variables. A new token is
    requested automatically for every command.

    ```bash
    export AINARI_ADDRESS=http://127.0.0.1:11417
    export AINARI_USER=asdf
    export AINARI_PASSPHRASE=asdfasdf
    ```

    Global flags, which are available for all commands:

    | Flag                 | Description                                                         |
    | -------------------- | ------------------------------------------------------------------- |
    | `--insecure`         | disable the tls-verification, for example for self-signed certificates |
    | `-j`, `--json_output`| print the output as json instead of a table, for example for `jq`  |

=== "Python-SDK"

    ```python
    from ainari_sdk import login

    context = login.request_context("http://127.0.0.1:11417", "asdf", "asdfasdf")
    ```

    The returned `context` is the first argument of all other functions. To disable the
    tls-verification, for example for self-signed certificates, add `verify_connection=False`:

    ```python
    context = login.request_context(address, user_id, passphrase, verify_connection=False)
    ```

## Exceptions

=== "Python-SDK"

    Each of the used HTTP-error-codes results in a different exception:

    ```python
    from ainari_sdk import ainari_exceptions, image

    try:
        image.get_image(context, image_uuid)
    except ainari_exceptions.BadRequestException as e:        # 400
        print(e)
    except ainari_exceptions.UnauthorizedException as e:      # 401
        print(e)
    except ainari_exceptions.NotFoundException as e:          # 404
        print(e)
    except ainari_exceptions.ConflictException as e:          # 409
        print(e)
    except ainari_exceptions.InternalServerErrorException:    # 500
        print("internal error")

    # example output:
    #
    # b"image with UUID '00000000-0000-0000-0000-000000000001' not found."
    ```

    !!! info

        The `InternalServerErrorException` doesn't contain a message. If this exception appears,
        you have to look into the logs of the server.

## Token, endpoints and version

=== "CLI"

    ```bash
    # validate the own token and print its user-context
    ainarictl token validate

    # request a new token
    ainarictl token renew

    # addresses of the components
    ainarictl endpoints

    # version of the backend
    ainarictl get version
    ```

    example:

    ```bash
    ainarictl endpoints

    ┌─────────┬────────────────────────────────────────────────────────────────────────────────────────────┐
    │ HANAMI  │ {"internal_address":"http://hanami:10418","public_address":"http://127.0.0.1:11418"}       │
    │ OMAMORI │ {"internal_address":"http://omamori:10421","public_address":"http://127.0.0.1:11421"}      │
    │ RYOKAN  │ {"internal_address":"http://ryokan:10416","public_address":"http://127.0.0.1:11416"}       │
    │ TORII   │ {"internal_address":"http://torii-public:10419","public_address":"http://127.0.0.1:11419"} │
    └─────────┴────────────────────────────────────────────────────────────────────────────────────────────┘
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import common, login

    login.validate_token(context)     # {"context": {"user_id": "asdf", "is_admin": "true", ...}}
    login.renew_token(context)        # {"access_token": "eyJ0eXAi..."}
    login.get_endpoints(context)      # {"hanami": {"public_address": ..., "internal_address": ...}, ...}

    common.get_version(context, context.miko_address)

    # example-content of result:
    #
    # {
    #     "version": "unknown",
    #     "commit_hash": "",
    #     "timestamp": "2026-09-26 19:52:26"
    # }
    ```

    `renew_token` doesn't modify the context, so the new token has to be set with
    `context.token = result["access_token"]`.

## Hosts

The sakura-hosts run the virtual machines, the onsen-hosts store the images and snapshots. Both
register themselves at startup.

!!! info

    Only admins are allowed to manage hosts.

=== "CLI"

    ```bash
    ainarictl host list
    ainarictl host get <HOST_UUID>
    ainarictl host delete <HOST_UUID>

    ainarictl onsen_host list
    ainarictl onsen_host get <HOST_UUID>
    ainarictl onsen_host delete <HOST_UUID>
    ```

    example:

    ```bash
    ainarictl host list

    ┌───────────────────────────┬───────────────────────┬────────────┬───────────────────────┬─────────────┬──────────────┬─────────────────┬──────────────────────┬──────────────────────────────────────┐
    │ AMOUNT OF USED DISK SPACE │ AMOUNT OF USED MEMORY │ DISK SPACE │     HOST ADDRESS      │ MEMORY SIZE │     NAME     │ NUMBER OF CORES │ USED NUMBER OF CORES │                 UUID                 │
    ├───────────────────────────┼───────────────────────┼────────────┼───────────────────────┼─────────────┼──────────────┼─────────────────┼──────────────────────┼──────────────────────────────────────┤
    │ 10                        │ 2048                  │ 211        │ http://sakura-2:11420 │ 64209       │ c248af01c8a1 │ 16              │ 2                    │ 5251f5ab-587b-41a6-a12c-f847d6931e68 │
    │ 10                        │ 2048                  │ 211        │ http://sakura:11420   │ 64209       │ 25310b148a34 │ 16              │ 2                    │ f882fe4e-bab3-4ac7-b464-687ea3ef3571 │
    └───────────────────────────┴───────────────────────┴────────────┴───────────────────────┴─────────────┴──────────────┴─────────────────┴──────────────────────┴──────────────────────────────────────┘
    ```

    Memory is given in MiB, disk-space in GiB.

=== "Python-SDK"

    ```python
    from ainari_sdk import host

    host.list_hosts(context)
    host.get_host(context, host_uuid)
    host.delete_host(context, host_uuid)

    host.list_onsen_hosts(context)
    host.get_onsen_host(context, host_uuid)
    host.delete_onsen_host(context, host_uuid)

    # example-content of the result of list_hosts:
    #
    # {
    #     "hosts": [
    #         {
    #             "uuid": "5251f5ab-587b-41a6-a12c-f847d6931e68",
    #             "name": "c248af01c8a1",
    #             "host_address": "http://sakura-2:11420",
    #             "number_of_cores": 16,
    #             "used_number_of_cores": 2,
    #             "memory_size": 64209,
    #             "amount_of_used_memory": 2048,
    #             "disk_space": 211,
    #             "amount_of_used_disk_space": 10
    #         },
    #         ...
    #     ]
    # }
    ```

## Public keys

ssh-public-keys, which are deployed into virtual machines for the default-user of the image.

=== "CLI"

    ```bash
    ainarictl public_key upload -k <PUBLIC_KEY_FILE> <NAME>
    ainarictl public_key list
    ainarictl public_key get <PUBLIC_KEY_UUID>
    ainarictl public_key delete <PUBLIC_KEY_UUID>
    ```

    example:

    ```bash
    ainarictl public_key upload -k ./ainari_key.pub my-key

    ┌─────────────┬────────────────────────────────────────────────────┐
    │ CREATED AT  │ 2026-09-26T20:03:31.680995885Z                     │
    │ CREATED BY  │ asdf                                               │
    │ FINGERPRINT │ SHA256:NDAzYbceDFgBNX08rg+jkvRqGZyPR+NZpDyIoLGtgRg │
    │ NAME        │ my-key                                             │
    │ UPDATED AT  │ 2026-09-26T20:03:31.680996125Z                     │
    │ UPDATED BY  │ asdf                                               │
    │ UUID        │ bd9f3a6b-df8f-42cc-803b-21f997feea96               │
    └─────────────┴────────────────────────────────────────────────────┘
    ```

=== "Python-SDK"

    The public key is given as string in its one-line openssh-representation.

    ```python
    from ainari_sdk import public_key

    with open("./ainari_key.pub") as f:
        result = public_key.upload_public_key(context, "my-key", f.read().strip())

    public_key.list_public_keys(context)    # {"public_keys": [{"uuid": ..., "name": ..., "fingerprint": ...}]}
    public_key.get_public_key(context, public_key_uuid)
    public_key.delete_public_key(context, public_key_uuid)
    public_key.delete_all_public_keys(context)
    ```

## Images

Disk-images, which are used as boot-disk of virtual machines, for example an ubuntu-cloud-image.
Snapshots of virtual machines are stored as images too, with `is_snapshot` set.

=== "CLI"

    ```bash
    ainarictl image create disk -i <INPUT_FILE_PATH> <NAME>
    ainarictl image list
    ainarictl image get <IMAGE_UUID>
    ainarictl image count
    ainarictl image delete <IMAGE_UUID>
    ```

    example:

    ```bash
    ainarictl image list

    ┌─────────────┬───────────────────────┬──────────────────────────────────────┐
    │ IS SNAPSHOT │         NAME          │                 UUID                 │
    ├─────────────┼───────────────────────┼──────────────────────────────────────┤
    │ false       │ ubuntu-noble          │ 0fefb138-6077-489a-930c-afad9c5c54bf │
    │ true        │ my-snapshot           │ 0ff16ea5-5380-4f3d-af62-08b1fe5d615b │
    └─────────────┴───────────────────────┴──────────────────────────────────────┘
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import image

    result = image.upload_disk_file(context, "ubuntu-noble", "./noble-server-cloudimg-amd64.img")

    image.list_images(context)        # {"images": [{"uuid": ..., "name": ..., "is_snapshot": false}]}
    image.get_image(context, image_uuid)
    image.get_image_count(context)    # {"number_of_items": 1}
    image.delete_image(context, image_uuid)
    image.delete_all_images(context)
    ```

!!! info

    The image-file is stored encrypted. For every image a secret with the name
    `autogenerated secret for image <IMAGE_UUID>` is created automatically.

## Networks

=== "CLI"

    ```bash
    ainarictl network create -s <SUBNET> <NAME>
    ainarictl network list
    ainarictl network get <NETWORK_UUID>
    ainarictl network delete <NETWORK_UUID>
    ```

    example:

    ```bash
    ainarictl network create -s 192.168.200.1/24 my-network

    ┌────────────┬──────────────────────────────────────┐
    │ CREATED AT │ 2026-09-26T20:03:31.734173936Z       │
    │ CREATED BY │ asdf                                 │
    │ NAME       │ my-network                           │
    │ SUBNET     │ 192.168.200.1/24                     │
    │ UPDATED AT │ 2026-09-26T20:03:31.734174146Z       │
    │ UPDATED BY │ asdf                                 │
    │ UUID       │ 1769874d-07e4-4bea-a11f-3ea09caa9ef4 │
    └────────────┴──────────────────────────────────────┘
    ```

    The first address of the subnet is the gateway of the virtual machines.

=== "Python-SDK"

    ```python
    from ainari_sdk import network

    result = network.create_network(context, "my-network", "192.168.200.1/24")

    network.list_networks(context)    # {"networks": [{"uuid": ..., "name": ..., "subnet": ...}]}
    network.get_network(context, network_uuid)
    network.delete_network(context, network_uuid)
    network.delete_all_networks(context)
    ```

## Virtual machines

A virtual machine is reserved on one of the sakura-hosts first. Afterwards the image and the public
key are installed into it and it is booted by a [task](#tasks) on this host. The host is addressed
over the `torii_port` of the virtual machine.

The state of the virtual machine is shown in `vm_state`:

| State       | Description                                          |
| ----------- | ---------------------------------------------------- |
| `RESERVED`  | reserved on a host, but not created yet              |
| `CREATED`   | the task, which creates the virtual machine, is running |
| `RUNNING`   | booted                                               |
| `STOPPED`   | powered off                                          |
| `RESTORING` | the root-disk is reset to a snapshot                 |
| `ERROR`     | something blocks the start of the virtual machine    |

### Create virtual machine

=== "CLI"

    `vm create` does both steps, the reservation and the creation. Memory is given in MiB, the
    disk in GiB.

    ```bash
    ainarictl vm create -c <NUMBER_OF_CORES> -m <MEMORY_SIZE> -d <DISK_SIZE> \
        -u <NETWORK_UUID> -i <IMAGE_UUID> -k <PUBLIC_KEY_UUID> <NAME>
    ```

    example:

    ```bash
    ainarictl vm create -c 1 -m 1024 -d 5 \
        -u 1769874d-07e4-4bea-a11f-3ea09caa9ef4 \
        -i 0fefb138-6077-489a-930c-afad9c5c54bf \
        -k bd9f3a6b-df8f-42cc-803b-21f997feea96 \
        my-vm

    ┌─────────────────┬──────────────────────────────────────┐
    │ CREATED AT      │ 2026-09-26T20:03:37.596527148Z       │
    │ CREATED BY      │ asdf                                 │
    │ DISK SIZE       │ 5                                    │
    │ IMAGE UUID      │ 0fefb138-6077-489a-930c-afad9c5c54bf │
    │ INTERNAL IP     │ 192.168.200.2                        │
    │ MEMORY SIZE     │ 1024                                 │
    │ NAME            │ my-vm                                │
    │ NETWORK UUID    │ 1769874d-07e4-4bea-a11f-3ea09caa9ef4 │
    │ NUMBER OF CORES │ 1                                    │
    │ TORII PORT      │ 10044                                │
    │ UPDATED AT      │ 2026-09-26T20:03:37.690766143Z       │
    │ UPDATED BY      │ asdf                                 │
    │ UUID            │ 943e56bb-c6e4-4716-a26c-f1fd46a45618 │
    │ VM STATE        │ CREATED                              │
    └─────────────────┴──────────────────────────────────────┘
    ```

    The creation runs in the background; repeat `ainarictl vm get <VIRTUAL_MACHINE_UUID>`, until
    `VM STATE` is `RUNNING`.

=== "Python-SDK"

    ```python
    from ainari_sdk import virtual_machine

    # 1. reserve the virtual machine on one of the sakura-hosts
    reserved = virtual_machine.reserve_virtual_machine(context,
                                                       "my-vm",
                                                       1,        # number of cores
                                                       1024,     # memory in MiB
                                                       5,        # disk in GiB
                                                       network_uuid)

    # 2. install image and public key and boot it
    task = virtual_machine.create_virtual_machine(context,
                                                  reserved["torii_port"],
                                                  reserved["uuid"],
                                                  image_uuid,
                                                  public_key_uuid)

    # 3. wait until it is running
    while virtual_machine.get_virtual_machine(context, reserved["uuid"])["vm_state"] != "RUNNING":
        time.sleep(2.0)
    ```

    If the second step fails, the virtual machine stays reserved and has to be deleted.

### Get, list and delete virtual machines

=== "CLI"

    ```bash
    ainarictl vm list
    ainarictl vm get <VIRTUAL_MACHINE_UUID>
    ainarictl vm count
    ainarictl vm delete <VIRTUAL_MACHINE_UUID>
    ```

    example:

    ```bash
    ainarictl vm list

    ┌───────────┬─────────────┬─────────┬─────────────────┬────────────┬──────────────────────────────────────┐
    │ DISK SIZE │ MEMORY SIZE │  NAME   │ NUMBER OF CORES │ PROXY PORT │                 UUID                 │
    ├───────────┼─────────────┼─────────┼─────────────────┼────────────┼──────────────────────────────────────┤
    │ 10        │ 2048        │ vm-1    │ 2               │ 10042      │ 2310f0f6-f62f-439a-80c3-0f6ce9ba3bbd │
    │ 5         │ 1024        │ my-vm   │ 1               │ 10044      │ 943e56bb-c6e4-4716-a26c-f1fd46a45618 │
    └───────────┴─────────────┴─────────┴─────────────────┴────────────┴──────────────────────────────────────┘
    ```

    The deletion runs in the background:

    ```bash
    ainarictl vm delete 943e56bb-c6e4-4716-a26c-f1fd46a45618

    deletion of virtual machine '943e56bb-c6e4-4716-a26c-f1fd46a45618' started
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import virtual_machine

    virtual_machine.list_virtual_machines(context)    # {"virtual_machines": [...]}
    virtual_machine.get_virtual_machine_count(context)    # {"number_of_items": 2}
    virtual_machine.delete_virtual_machine(context, virtual_machine_uuid)
    virtual_machine.delete_all_virtual_machines(context)

    virtual_machine.get_virtual_machine(context, virtual_machine_uuid)

    # example-content of result:
    #
    # {
    #     "uuid": "943e56bb-c6e4-4716-a26c-f1fd46a45618",
    #     "name": "my-vm",
    #     "vm_state": "RUNNING",
    #     "number_of_cores": 1,
    #     "memory_size": 1024,
    #     "disk_size": 5,
    #     "image_uuid": "0fefb138-6077-489a-930c-afad9c5c54bf",
    #     "network_uuid": "1769874d-07e4-4bea-a11f-3ea09caa9ef4",
    #     "internal_ip": "192.168.200.2",
    #     "torii_port": 10044,
    #     "created_at": "2026-09-26T20:03:37.596527148Z",
    #     "created_by": "asdf",
    #     "updated_at": "2026-09-26T20:03:41.705820591Z",
    #     "updated_by": "asdf"
    # }
    ```

### Start, stop and reboot

Each of these creates a [task](#tasks) on the sakura-host of the virtual machine. A stopped
virtual machine keeps all of its resources.

=== "CLI"

    ```bash
    ainarictl vm stop <VIRTUAL_MACHINE_UUID>
    ainarictl vm start <VIRTUAL_MACHINE_UUID>
    ainarictl vm reboot <VIRTUAL_MACHINE_UUID>
    ```

    example:

    ```bash
    ainarictl vm stop 943e56bb-c6e4-4716-a26c-f1fd46a45618

    ┌─────────────┬─────────────────────────────────────────────────────────────────────┐
    │ CREATED BY  │ asdf                                                                │
    │ DESCRIPTION │ Stop virtual machine with UUID 943e56bb-c6e4-4716-a26c-f1fd46a45618 │
    │ FINISHED AT │ <nil>                                                               │
    │ MESSAGES    │ []                                                                  │
    │ QUEUED AT   │ 2026-09-26T20:03:43.047004500Z                                      │
    │ STARTED AT  │ <nil>                                                               │
    │ STATE       │ Queued                                                              │
    │ TASK TYPE   │ VirtualMachineStop                                                  │
    │ UUID        │ 0ae2b3ed-a008-4c31-a1a8-434f558d5be1                                │
    └─────────────┴─────────────────────────────────────────────────────────────────────┘
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import virtual_machine

    torii_port = virtual_machine.get_virtual_machine(context, virtual_machine_uuid)["torii_port"]

    task = virtual_machine.stop_virtual_machine(context, torii_port, virtual_machine_uuid)
    task = virtual_machine.start_virtual_machine(context, torii_port, virtual_machine_uuid)
    task = virtual_machine.reboot_virtual_machine(context, torii_port, virtual_machine_uuid)
    ```

## Floating IPs

Floating IPs make a virtual machine reachable from the network behind the uplink of the gateway.

=== "CLI"

    ```bash
    # reserve a free floating IP, or a specific one, and optionally attach it directly
    ainarictl floating_ip add -n <NAME> [-v <VIRTUAL_MACHINE_UUID>] [<FLOATING_IP>]

    ainarictl floating_ip attach <FLOATING_IP_UUID> <VIRTUAL_MACHINE_UUID>
    ainarictl floating_ip detach <FLOATING_IP_UUID>
    ainarictl floating_ip list
    ainarictl floating_ip get <FLOATING_IP_UUID>
    ainarictl floating_ip delete <FLOATING_IP_UUID>
    ```

    example:

    ```bash
    ainarictl floating_ip add -n my-fip

    ┌──────────────┬──────────────────────────────────────┐
    │ CREATED AT   │ 2026-09-26T20:03:31.786090618Z       │
    │ CREATED BY   │ asdf                                 │
    │ FLOATING IP  │ 10.0.0.4                             │
    │ INTERNAL IP  │ <nil>                                │
    │ NETWORK UUID │ <nil>                                │
    │ UPDATED AT   │ 2026-09-26T20:03:31.786090838Z       │
    │ UPDATED BY   │ asdf                                 │
    │ UUID         │ c4521f1a-fe9f-4eea-b146-e7cb66d59a54 │
    └──────────────┴──────────────────────────────────────┘

    ainarictl floating_ip attach c4521f1a-fe9f-4eea-b146-e7cb66d59a54 943e56bb-c6e4-4716-a26c-f1fd46a45618

    ┌──────────────┬──────────────────────────────────────┐
    │ CREATED AT   │ 2026-09-26T20:03:31.786090618Z       │
    │ CREATED BY   │ asdf                                 │
    │ FLOATING IP  │ 10.0.0.4                             │
    │ INTERNAL IP  │ 192.168.200.2                        │
    │ NETWORK UUID │ 1769874d-07e4-4bea-a11f-3ea09caa9ef4 │
    │ UPDATED AT   │ 2026-09-26T20:03:42.824349537Z       │
    │ UPDATED BY   │ asdf                                 │
    │ UUID         │ c4521f1a-fe9f-4eea-b146-e7cb66d59a54 │
    └──────────────┴──────────────────────────────────────┘
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import floating_ip

    # reserve and attach within one call
    result = floating_ip.create_floating_ip(context, "my-fip",
                                            virtual_machine_uuid=virtual_machine_uuid)

    # or reserve first and attach afterwards
    result = floating_ip.create_floating_ip(context, "my-fip")
    result = floating_ip.attach_floating_ip(context, result["uuid"], virtual_machine_uuid)

    floating_ip.detach_floating_ip(context, floating_ip_uuid)
    floating_ip.list_floating_ips(context)    # {"floating_ips": [...]}
    floating_ip.get_floating_ip(context, floating_ip_uuid)
    floating_ip.delete_floating_ip(context, floating_ip_uuid)
    floating_ip.delete_all_floating_ips(context)

    # example-content of result:
    #
    # {
    #     "uuid": "c4521f1a-fe9f-4eea-b146-e7cb66d59a54",
    #     "floating_ip": "10.0.0.4",
    #     "internal_ip": "192.168.200.2",
    #     "network_uuid": "1769874d-07e4-4bea-a11f-3ea09caa9ef4",
    #     ...
    # }
    ```

    A specific floating IP can be requested with `floating_ip="10.0.0.10"`.

## Tasks

Actions, which take longer, run as tasks in the background on the sakura-host of a virtual machine:
the creation, start, stop and reboot of virtual machines and the snapshots. The `STATE` of a task
is one of `Created`, `Queued`, `Active`, `Finished`, `Aborted` or `Error`. In case of an error,
the reason is listed in `MESSAGES`.

=== "CLI"

    The CLI selects the sakura-host over the given virtual machine.

    ```bash
    ainarictl task list <VIRTUAL_MACHINE_UUID>
    ainarictl task get <VIRTUAL_MACHINE_UUID> <TASK_UUID>
    ainarictl task abort <VIRTUAL_MACHINE_UUID> <TASK_UUID>
    ```

    example:

    ```bash
    ainarictl task get 943e56bb-c6e4-4716-a26c-f1fd46a45618 0ae2b3ed-a008-4c31-a1a8-434f558d5be1

    ┌─────────────┬─────────────────────────────────────────────────────────────────────┐
    │ CREATED BY  │ asdf                                                                │
    │ DESCRIPTION │ Stop virtual machine with UUID 943e56bb-c6e4-4716-a26c-f1fd46a45618 │
    │ FINISHED AT │ 2026-09-26T20:03:43.083014431Z                                      │
    │ MESSAGES    │ []                                                                  │
    │ QUEUED AT   │ 2026-09-26T20:03:43.047004500Z                                      │
    │ STARTED AT  │ 2026-09-26T20:03:43.054219173Z                                      │
    │ STATE       │ Finished                                                            │
    │ TASK TYPE   │ VirtualMachineStop                                                  │
    │ UUID        │ 0ae2b3ed-a008-4c31-a1a8-434f558d5be1                                │
    └─────────────┴─────────────────────────────────────────────────────────────────────┘
    ```

=== "Python-SDK"

    The sakura-host is selected over the `torii_port` of a virtual machine on this host.

    ```python
    from ainari_sdk import task

    task.list_tasks(context, torii_port)    # {"tasks": [...]}
    task.get_task(context, torii_port, task_uuid)
    task.abort_task(context, torii_port, task_uuid)

    # wait until the task is done and check its result
    result = task.wait_for_task_finished(context, torii_port, task_uuid)
    if result["state"] != "Finished":
        print(result["messages"])
    ```

### Snapshots

A snapshot saves the root-disk of a virtual machine as new image with `is_snapshot` set. It can
be restored into a virtual machine later on.

!!! warning

    A running virtual machine is only paused during the copy, so data, which is still in its
    memory, is missing in the snapshot. Run `sync` inside the virtual machine right before creating
    the snapshot, or stop the virtual machine before.

=== "CLI"

    ```bash
    ainarictl task create snapshot_create <VIRTUAL_MACHINE_UUID> <SNAPSHOT_NAME>
    ainarictl task create snapshot_restore -i <SNAPSHOT_IMAGE_UUID> <VIRTUAL_MACHINE_UUID>
    ```

    example:

    ```bash
    ainarictl task create snapshot_create 943e56bb-c6e4-4716-a26c-f1fd46a45618 my-snapshot

    ┌─────────────┬──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
    │ CREATED BY  │ asdf                                                                                                                         │
    │ DESCRIPTION │ Create snapshot-image 0ff16ea5-5380-4f3d-af62-08b1fe5d615b of virtual machine with UUID 943e56bb-c6e4-4716-a26c-f1fd46a45618 │
    │ FINISHED AT │ <nil>                                                                                                                        │
    │ MESSAGES    │ []                                                                                                                           │
    │ QUEUED AT   │ 2026-09-26T20:06:03.073346092Z                                                                                               │
    │ STARTED AT  │ <nil>                                                                                                                        │
    │ STATE       │ Queued                                                                                                                       │
    │ TASK TYPE   │ SnapshotSave                                                                                                                 │
    │ UUID        │ ae64a8be-0031-4e72-8ba4-05939dbd2abe                                                                                         │
    └─────────────┴──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import task

    task.create_snapshot_save_task(context, torii_port, virtual_machine_uuid, "my-snapshot")
    task.create_snapshot_restore_task(context, torii_port, virtual_machine_uuid, snapshot_image_uuid)
    ```

## Secrets

Secrets are stored encrypted in omamori.

=== "CLI"

    ```bash
    # upload a secret
    ainarictl secret create -p <SECRET_PAYLOAD> <NAME>

    # let the server generate the payload
    ainarictl secret generate <NAME>

    ainarictl secret list
    ainarictl secret get <SECRET_UUID>
    ainarictl secret get-payload <SECRET_UUID>
    ainarictl secret count
    ainarictl secret delete <SECRET_UUID>
    ```

    example:

    ```bash
    ainarictl secret create -p my-secret-value my-secret

    ┌────────────┬──────────────────────────────────────┐
    │ CREATED AT │ 2026-09-26T20:03:31.590033494+00:00  │
    │ CREATED BY │ asdf                                 │
    │ NAME       │ my-secret                            │
    │ UPDATED AT │ 2026-09-26T20:03:31.590036109+00:00  │
    │ UPDATED BY │ asdf                                 │
    │ UUID       │ 4121ad18-1115-4197-a716-0031e68f327f │
    └────────────┴──────────────────────────────────────┘

    ainarictl secret get-payload 4121ad18-1115-4197-a716-0031e68f327f

    ┌────────────────┬─────────────────┐
    │ SECRET PAYLOAD │ my-secret-value │
    └────────────────┴─────────────────┘
    ```

    !!! warning

        With `-p` the secret is visible in the command-line and the shell-history.

=== "Python-SDK"

    ```python
    from ainari_sdk import secret

    result = secret.create_secret(context, "my-secret", "my-secret-value")
    result = secret.generate_secret(context, "my-generated-secret")

    secret.list_secrets(context)    # {"secrets": [{"uuid": ..., "name": ...}]}
    secret.get_secret(context, secret_uuid)
    secret.get_secret_payload(context, secret_uuid)    # {"secret_payload": "my-secret-value"}
    secret.get_secret_count(context)    # {"number_of_items": 1}
    secret.delete_secret(context, secret_uuid)
    secret.delete_all_secrets(context)
    ```

## Proxies

Every virtual machine gets a proxy-port on the torii automatically, which forwards to its
sakura-host (the `torii_port` of the virtual machine). So proxies normally don't have to be managed
by hand.

=== "CLI"

    ```bash
    ainarictl proxy list
    ainarictl proxy get <PROXY_UUID>
    ainarictl proxy set -t <TARGET_ADDRESS> -v <VIRTUAL_MACHINE_UUID>
    ainarictl proxy delete <PROXY_UUID>
    ```

    example:

    ```bash
    ainarictl proxy list

    ┌───────┬───────────────────────┬──────────────────────────────────────┬──────────────────────────────────────┐
    │ PORT  │    TARGET ADDRESS     │                 UUID                 │         VIRTUAL MACHINE UUID         │
    ├───────┼───────────────────────┼──────────────────────────────────────┼──────────────────────────────────────┤
    │ 10042 │ http://sakura:11420   │ fb0e6638-b840-48e3-b474-5c7fed4dded9 │ 2310f0f6-f62f-439a-80c3-0f6ce9ba3bbd │
    │ 10043 │ http://sakura-2:11420 │ 70871149-dec8-4911-b2cd-4a343f8c4614 │ 070af3fd-1a37-4546-b3e2-035e03b3bd65 │
    └───────┴───────────────────────┴──────────────────────────────────────┴──────────────────────────────────────┘
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import proxy

    proxy.list_proxys(context)    # {"proxys": [...]}
    proxy.get_proxy(context, proxy_uuid)
    proxy.set_proxy(context, target_address, virtual_machine_uuid)
    proxy.delete_proxy(context, proxy_uuid)
    proxy.delete_all_proxys(context)
    ```

## Projects

Projects are used for logical separation of the resources of users.

!!! info

    Only admins are allowed to manage projects.

=== "CLI"

    ```bash
    ainarictl project create -n <NAME> <PROJECT_ID>
    ainarictl project list
    ainarictl project get <PROJECT_ID>
    ainarictl project delete <PROJECT_ID>
    ```

    example:

    ```bash
    ainarictl project create -n "my project" my_project

    ┌────────────┬─────────────────────────────────────┐
    │ CREATED AT │ 2026-09-26T20:03:31.501358043+00:00 │
    │ CREATED BY │ asdf                                │
    │ ID         │ my_project                          │
    │ NAME       │ my project                          │
    │ UPDATED AT │ 2026-09-26T20:03:31.501360318+00:00 │
    │ UPDATED BY │ asdf                                │
    └────────────┴─────────────────────────────────────┘
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import project

    result = project.create_project(context, "my_project", "my project")

    project.list_projects(context)    # {"projects": [{"id": "my_project", "name": "my project"}]}
    project.get_project(context, "my_project")
    project.delete_project(context, "my_project")
    project.delete_all_projects(context)
    ```

## Users

!!! info

    Only admins are allowed to manage users.

=== "CLI"

    ```bash
    ainarictl user create -n <NAME> [--is_admin] <USER_ID>
    ainarictl user list
    ainarictl user get <USER_ID>
    ainarictl user delete <USER_ID>
    ```

    Without `-p <PASSPHRASE>` the passphrase is requested interactively. The flag should only be
    used for automated testing, because the passphrase is visible in the command-line and the
    shell-history.

    example:

    ```bash
    ainarictl user create -n "my user" my_user

    ┌────────────┬─────────────────────────────────────┐
    │ CREATED AT │ 2026-09-26T20:03:31.521602790+00:00 │
    │ CREATED BY │ asdf                                │
    │ ID         │ my_user                             │
    │ IS ADMIN   │ false                               │
    │ NAME       │ my user                             │
    │ UPDATED AT │ 2026-09-26T20:03:31.521603672+00:00 │
    │ UPDATED BY │ asdf                                │
    └────────────┴─────────────────────────────────────┘
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import user

    result = user.create_user(context, "my_user", "my user", "my-passphrase", False)

    user.list_users(context)    # {"users": [{"id": ..., "name": ..., "is_admin": "false"}]}
    user.get_user(context, "my_user")
    user.delete_user(context, "my_user")
    user.delete_all_user(context)
    ```

## Quotas

Maximum number of resources per user. Every user can see the own quota, only admins can see and
set the quotas of other users.

=== "CLI"

    ```bash
    # own quota
    ainarictl quota show

    # admin only
    ainarictl quota list
    ainarictl quota get <USER_ID>
    ainarictl quota set <USER_ID> \
        --max_virtual_machine <N> --max_image <N> --max_secret <N> \
        --max_network <N> --max_floating_ip <N>
    ```

    example:

    ```bash
    ainarictl quota list

    ┌─────────────────┬───────────┬─────────────┬────────────┬─────────────────────┬─────────┐
    │ MAX FLOATING IP │ MAX IMAGE │ MAX NETWORK │ MAX SECRET │ MAX VIRTUAL MACHINE │ USER ID │
    ├─────────────────┼───────────┼─────────────┼────────────┼─────────────────────┼─────────┤
    │ 10              │ 10        │ 10          │ 10         │ 10                  │ asdf    │
    │ 2               │ 5         │ 2           │ 5          │ 5                   │ my_user │
    └─────────────────┴───────────┴─────────────┴────────────┴─────────────────────┴─────────┘
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import quota

    quota.get_own_quota(context)

    # admin only
    quota.list_quotas(context)    # {"quotas": [...]}
    quota.get_quota(context, "my_user")
    quota.set_quota(context, "my_user",
                    5,    # max_virtual_machine
                    5,    # max_image
                    5,    # max_secret
                    2,    # max_network
                    2)    # max_floating_ip
    ```
