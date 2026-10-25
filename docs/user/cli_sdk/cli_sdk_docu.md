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
    # optional, the default-project of the user is used, if not set
    export AINARI_PROJECT=my_project
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

    By default, the token is created for the default-project of the user. To use another project,
    to which the user is assigned, add `project_id`:

    ```python
    context = login.request_context(address, user_id, passphrase, project_id="my_project")
    ```

Each user has its own project `default-<USER_ID>`, in which the user is admin. The token is only
created, if the user is assigned to the requested project, and contains the role of the user
within this project. The role is one of `admin`, `member` or `observer`. Observers, which are not
admins, can only read resources. Creating, changing or deleting them is rejected with `403`.

!!! info "Invalidation of tokens"

    Besides its expiration, a token is checked against the current state of miko with every
    request. A token is invalidated immediately and rejected with `401`, as soon as

    - its user was deleted or the admin-flag of the user was changed,
    - the user was removed from the project of the token or the project was deleted,
    - the role of the user within the project was changed.

    After a change of the role, the token can still be renewed with `token renew` or
    `login.renew_token` to get a new token with the new role. In all other cases a new login is
    required.

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
    except ainari_exceptions.ForbiddenException as e:         # 403
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
register themselves at startup. An onsen-host, which is marked as backup-target, only stores the
[backups](#backups) and never images or snapshots. It can only be marked, while it stores no
images or snapshots.

!!! info

    Only admins are allowed to manage hosts.

A sakura-host can be isolated, so it is only used by the virtual machines of a single project. An
isolated host is bound to the project of the first virtual machine, which is created with
`isolated_host` on it, and is released again, when its last virtual machine is deleted. Virtual
machines without `isolated_host` are never placed on isolated hosts and virtual machines with it
only on isolated hosts. The isolation can only be changed, while no virtual machine runs on the
host.

An admin can also place a new virtual machine on a specific host (see
[Create virtual machine](#create-virtual-machine)) and move an existing one to another host (see
[Migrate virtual machine](#migrate-virtual-machine)).

=== "CLI"

    ```bash
    ainarictl host list
    ainarictl host get <HOST_UUID>
    ainarictl host set_isolation <HOST_UUID> <true|false>
    ainarictl host delete <HOST_UUID>

    ainarictl onsen_host list
    ainarictl onsen_host get <HOST_UUID>
    ainarictl onsen_host delete <HOST_UUID>
    ```

    example:

    ```bash
    ainarictl host list

    ┌───────────────────────────┬───────────────────────┬────────────┬───────────────────────┬──────────────────┬─────────────┬──────────────┬─────────────────┬──────────────────────┬──────────────────────────────────────┐
    │ AMOUNT OF USED DISK SPACE │ AMOUNT OF USED MEMORY │ DISK SPACE │     HOST ADDRESS      │ IS HOST ISOLATED │ MEMORY SIZE │     NAME     │ NUMBER OF CORES │ USED NUMBER OF CORES │                 UUID                 │
    ├───────────────────────────┼───────────────────────┼────────────┼───────────────────────┼──────────────────┼─────────────┼──────────────┼─────────────────┼──────────────────────┼──────────────────────────────────────┤
    │ 10                        │ 2048                  │ 211        │ http://sakura-2:11420 │ true             │ 64209       │ c248af01c8a1 │ 16              │ 2                    │ 5251f5ab-587b-41a6-a12c-f847d6931e68 │
    │ 10                        │ 2048                  │ 211        │ http://sakura:11420   │ false            │ 64209       │ 25310b148a34 │ 16              │ 2                    │ f882fe4e-bab3-4ac7-b464-687ea3ef3571 │
    └───────────────────────────┴───────────────────────┴────────────┴───────────────────────┴──────────────────┴─────────────┴──────────────┴─────────────────┴──────────────────────┴──────────────────────────────────────┘
    ```

    Memory is given in MiB, disk-space in GiB.

=== "Python-SDK"

    ```python
    from ainari_sdk import host

    host.list_hosts(context)
    host.get_host(context, host_uuid)
    host.set_host_isolation(context, host_uuid, True)
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
    #             "amount_of_used_disk_space": 10,
    #             "is_host_isolated": true
    #         },
    #         ...
    #     ]
    # }
    ```

## Error and audit log

Omikuji collects the internal errors and the api-requests of all components centrally. The
error-log contains every internal error together with the component and the host, where it
occurred, and the user and project, whose request caused it. The audit-log contains every request
against the api of a component with its type and endpoint. The entries are only added by the
components themselves, so they can only be read, but not changed or deleted.

!!! info

    Only admins are allowed to read the error- and audit-log.

Both lists are sorted by the creation-time of the entries with the newest first. They can be
filtered by project, user and host and limited to a maximum number of entries, which is 100, if it
is not set.

=== "CLI"

    ```bash
    ainarictl error_log list [--project <PROJECT_ID>] [--user <USER_ID>] [--host <HOST_NAME>] [--limit <NUMBER>]
    ainarictl error_log get <ENTRY_UUID>

    ainarictl audit_log list [--project <PROJECT_ID>] [--user <USER_ID>] [--host <HOST_NAME>] [--limit <NUMBER>]
    ainarictl audit_log get <ENTRY_UUID>
    ```

    example:

    ```bash
    ainarictl audit_log list --user asdf --limit 2

    ┌────────────────┬────────────────────────────────┬──────────────────────────┬──────────────┬──────────────┬──────────────┬─────────┬──────────────────────────────────────┐
    │ COMPONENT NAME │           CREATED AT           │         ENDPOINT         │  HOST NAME   │  PROJECT ID  │ REQUEST TYPE │ USER ID │                 UUID                 │
    ├────────────────┼────────────────────────────────┼──────────────────────────┼──────────────┼──────────────┼──────────────┼─────────┼──────────────────────────────────────┤
    │ miko           │ 2026-10-10T20:15:31.506364899Z │ /v1alpha/endpoints       │ 98f844d338bf │ default-asdf │ GET          │ asdf    │ 265d9234-0297-430f-b87d-3465ae57a29a │
    │ omikuji        │ 2026-10-10T20:14:56.038895072Z │ /v1alpha/audit_log/admin │ 510a6b6407fb │ default-asdf │ GET          │ asdf    │ 171bb3b8-b7a4-4a1c-804b-1bb616d95148 │
    └────────────────┴────────────────────────────────┴──────────────────────────┴──────────────┴──────────────┴──────────────┴─────────┴──────────────────────────────────────┘
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import audit_log, error_log

    # every filter is optional
    error_log.list_error_logs(context, project_id=None, user_id=None, host_name=None, limit=None)
    error_log.get_error_log(context, entry_uuid)

    audit_log.list_audit_logs(context, project_id=None, user_id=None, host_name=None, limit=None)
    audit_log.get_audit_log(context, entry_uuid)

    # example-content of the result of list_error_logs:
    #
    # {
    #     "entries": [
    #         {
    #             "uuid": "1df55583-ef83-43c1-a10c-71df71b3ccc4",
    #             "message": "Task b95d8f26-0520-410b-bfe5-85af4e17b25f (Create virtual machine with UUID cf7b3ad5-d3a9-4868-9f9c-cd148ef14ac5) failed: cloud-localds failed with exit status: exit status: 1",
    #             "component_name": "sakura",
    #             "host_name": "06e69bf521b8",
    #             "user_id": "asdf",
    #             "project_id": "default-asdf",
    #             "created_at": "2026-10-10T20:14:16.408813599Z"
    #         },
    #         ...
    #     ]
    # }
    #
    # the entries of the audit-log have the fields "request_type" and "endpoint" instead of
    # "message"
    ```

    The entries of background jobs, which no user caused, have an empty `user_id` and
    `project_id`. Without omikuji in the deployment, `context.omikuji_address` is empty.

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

While an image is uploaded with the CLI, a progress-bar shows the uploaded part of the file. It is
written to stderr and only shown in a terminal, so the output on stdout can still be parsed. After
the upload, ryokan still encrypts the image and stores it, which takes a while for big images.

Before an image is encrypted, its type is identified and its size is measured. Both are returned as
`file_type` and `file_size` (in bytes) by the upload, get and list. The type is named like the formats
of `qemu-img` (e.g. `qcow2` or `iso`), a file without known format is `raw`.

The `image_status` shows the state of the file of the image. An uploaded image appears, after its
file was completely uploaded. While ryokan encrypts it, it is `ENCRYPTING`, while ryokan stores the
encrypted file in the onsen, it is `UPLOADING` and afterwards `READY`. Only a `READY` image can be
used to create a virtual machine or to restore a snapshot, other images are rejected with a
conflict. If the encryption or the storing fails, the image is removed again. `IN_USE` is reserved
and not set at the moment.

=== "CLI"

    ```bash
    ainarictl image create disk -i <INPUT_FILE_PATH> [-s <SECRET_UUID>] <NAME>
    ainarictl image list
    ainarictl image get <IMAGE_UUID>
    ainarictl image count
    ainarictl image delete <IMAGE_UUID>
    ```

    example:

    ```bash
    ainarictl image list

    ┌───────────┬───────────┬──────────────┬─────────────┬───────────────────────┬──────────────────────────────────────┐
    │ FILE SIZE │ FILE TYPE │ IMAGE STATUS │ IS SNAPSHOT │         NAME          │                 UUID                 │
    ├───────────┼───────────┼──────────────┼─────────────┼───────────────────────┼──────────────────────────────────────┤
    │ 597688320 │ qcow2     │ READY        │ false       │ ubuntu-noble          │ 0fefb138-6077-489a-930c-afad9c5c54bf │
    │ 912261120 │ qcow2     │ READY        │ true        │ my-snapshot           │ 0ff16ea5-5380-4f3d-af62-08b1fe5d615b │
    └───────────┴───────────┴──────────────┴─────────────┴───────────────────────┴──────────────────────────────────────┘
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import image

    result = image.upload_disk_file(context, "ubuntu-noble", "./noble-server-cloudimg-amd64.img")
    # encrypt the image with an already existing secret instead of a new generated one
    result = image.upload_disk_file(context, "ubuntu-noble", "./noble-server-cloudimg-amd64.img",
                                    secret_uuid=secret_uuid)

    # {"images": [{"uuid": ..., "name": ..., "file_type": "qcow2", "file_size": 597688320,
    #              "image_status": "READY", "is_snapshot": false}]}
    image.list_images(context)
    image.get_image(context, image_uuid)
    image.get_image_count(context)    # {"number_of_items": 1}
    image.delete_image(context, image_uuid)
    image.delete_all_images(context)
    ```

!!! info

    The image-file is stored encrypted. For every image a secret with the name
    `autogenerated secret for image <IMAGE_UUID>` is created automatically. Alternatively an
    already existing [secret](#secrets) can be provided at the upload (`-s <SECRET_UUID>` in the
    CLI, `secret_uuid` in the SDK), which is used to encrypt the image instead. Like a generated
    secret, it is deleted together with the image.

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

## VM types

A vm-type defines the number of cores and the amount of memory of a virtual machine, which is
created with it. The vm-types are shared by all projects, so every user can list them and use them
for new virtual machines.

!!! info

    Only admins are allowed to create, update and delete vm-types.

=== "CLI"

    ```bash
    ainarictl vm_type list
    ainarictl vm_type get <VM_TYPE_UUID>

    # admin only
    ainarictl vm_type create -c <NUMBER_OF_CORES> -m <AMOUNT_OF_MEMORY> <NAME>
    ainarictl vm_type update [-n <NAME>] [-c <NUMBER_OF_CORES>] [-m <AMOUNT_OF_MEMORY>] <VM_TYPE_UUID>
    ainarictl vm_type delete <VM_TYPE_UUID>
    ```

    `vm_type update` only changes the values, which are given.

    example:

    ```bash
    ainarictl vm_type create -c 1 -m 1024 small

    ┌──────────────────┬──────────────────────────────────────┐
    │ AMOUNT OF MEMORY │ 1024                                 │
    │ CREATED AT       │ 2026-10-03T16:20:11.512834114Z       │
    │ CREATED BY       │ asdf                                 │
    │ NAME             │ small                                │
    │ NUMBER OF CORES  │ 1                                    │
    │ UPDATED AT       │ 2026-10-03T16:20:11.512834325Z       │
    │ UPDATED BY       │ asdf                                 │
    │ UUID             │ 3c1f0d7e-5a2b-4e8f-9d61-7b4a2c9e1f05 │
    └──────────────────┴──────────────────────────────────────┘

    ainarictl vm_type list

    ┌──────────────────┬────────┬─────────────────┬──────────────────────────────────────┐
    │ AMOUNT OF MEMORY │  NAME  │ NUMBER OF CORES │                 UUID                 │
    ├──────────────────┼────────┼─────────────────┼──────────────────────────────────────┤
    │ 1024             │ small  │ 1               │ 3c1f0d7e-5a2b-4e8f-9d61-7b4a2c9e1f05 │
    │ 8192             │ large  │ 8               │ a87e42b1-0c3d-4f59-8e16-d2b05f7c4a93 │
    └──────────────────┴────────┴─────────────────┴──────────────────────────────────────┘
    ```

    Memory is given in MiB.

=== "Python-SDK"

    ```python
    from ainari_sdk import vm_type

    vm_type.list_vm_types(context)    # {"vm_types": [{"uuid": ..., "name": ..., "number_of_cores": ..., "amount_of_memory": ...}]}
    vm_type.get_vm_type(context, vm_type_uuid)

    # admin only
    result = vm_type.create_vm_type(context,
                                    "small",
                                    1,       # number of cores
                                    1024)    # memory in MiB

    # only the given values are changed
    vm_type.update_vm_type(context, vm_type_uuid, number_of_cores=4)
    vm_type.update_vm_type(context, vm_type_uuid, name="medium", amount_of_memory=4096)

    vm_type.delete_vm_type(context, vm_type_uuid)
    ```

## Virtual machines

A virtual machine is reserved on one of the sakura-hosts first. Afterwards the image and the public
key are installed into it and it is booted by a [task](#tasks) on this host. The host is addressed
over the `torii_address` of the virtual machine, which is the address of its [proxy](#proxies).

The state of the virtual machine is shown in `vm_state`:

| State       | Description                                          |
| ----------- | ---------------------------------------------------- |
| `RESERVED`  | reserved on a host, but not created yet              |
| `CREATED`   | the task, which creates the virtual machine, is running |
| `RUNNING`   | booted                                               |
| `STOPPED`   | powered off                                          |
| `RESTORING` | the root-disk is reset to a snapshot                 |
| `MIGRATING` | the virtual machine is moved to another host         |
| `ERROR`     | something blocks the start of the virtual machine    |

### Create virtual machine

=== "CLI"

    `vm create` does both steps, the reservation and the creation. The number of cores and the
    memory are taken from the [vm-type](#vm-types), the disk is given in GiB.

    ```bash
    ainarictl vm create -t <VM_TYPE_UUID> -d <DISK_SIZE> \
        -u <NETWORK_UUID> -i <IMAGE_UUID> -k <PUBLIC_KEY_UUID> [--isolated_host] \
        [--host <HOST_UUID>] <NAME>
    ```

    With `--isolated_host` the virtual machine is only placed on a host, which is
    [isolated](#hosts) for the project.

    With `--host` an admin places the virtual machine on this specific [sakura-host](#hosts)
    instead of a randomly selected one. The creation fails, if the host has not enough free
    resources or its isolation doesn't match `--isolated_host`.

    example:

    ```bash
    ainarictl vm create -t 3c1f0d7e-5a2b-4e8f-9d61-7b4a2c9e1f05 -d 5 \
        -u 1769874d-07e4-4bea-a11f-3ea09caa9ef4 \
        -i 0fefb138-6077-489a-930c-afad9c5c54bf \
        -k bd9f3a6b-df8f-42cc-803b-21f997feea96 \
        my-vm

    ┌─────────────────┬─────────────────────────────────────────────────────────────────────┐
    │ CREATED AT      │ 2026-09-26T20:03:37.596527148Z                                      │
    │ CREATED BY      │ asdf                                                                │
    │ DISK SIZE       │ 5                                                                   │
    │ IMAGE UUID      │ 0fefb138-6077-489a-930c-afad9c5c54bf                                │
    │ INTERNAL IP     │ 192.168.200.2                                                       │
    │ MEMORY SIZE     │ 1024                                                                │
    │ NAME            │ my-vm                                                               │
    │ NETWORK UUID    │ 1769874d-07e4-4bea-a11f-3ea09caa9ef4                                │
    │ NUMBER OF CORES │ 1                                                                   │
    │ TORII ADDRESS   │ https://943e56bb-c6e4-4716-a26c-f1fd46a45618.ainari.localhost:10042 │
    │ UPDATED AT      │ 2026-09-26T20:03:37.690766143Z                                      │
    │ UPDATED BY      │ asdf                                                                │
    │ UUID            │ 943e56bb-c6e4-4716-a26c-f1fd46a45618                                │
    │ VM STATE        │ CREATED                                                             │
    └─────────────────┴─────────────────────────────────────────────────────────────────────┘
    ```

    The creation runs in the background; repeat `ainarictl vm get <VIRTUAL_MACHINE_UUID>`, until
    `VM STATE` is `RUNNING`.

=== "Python-SDK"

    ```python
    from ainari_sdk import virtual_machine

    # 1. reserve the virtual machine on one of the sakura-hosts
    reserved = virtual_machine.reserve_virtual_machine(context,
                                                       "my-vm",
                                                       vm_type_uuid,
                                                       5,        # disk in GiB
                                                       network_uuid,
                                                       isolated_host=False,
                                                       host_uuid=None)   # admin only

    # 2. install image and public key and boot it
    task = virtual_machine.create_virtual_machine(context,
                                                  reserved["torii_address"],
                                                  reserved["uuid"],
                                                  image_uuid,
                                                  public_key_uuid)

    # 3. wait until it is running
    while virtual_machine.get_virtual_machine(context, reserved["uuid"])["vm_state"] != "RUNNING":
        time.sleep(2.0)
    ```

    If the second step fails, the virtual machine stays reserved and has to be deleted. With
    `host_uuid` an admin places the virtual machine on this specific sakura-host, which fails with
    a `ConflictException`, if the host has not enough free resources.

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

    ┌───────────┬─────────────┬───────┬─────────────────┬─────────────────────────────────────────────────────────────────────┬──────────────────────────────────────┐
    │ DISK SIZE │ MEMORY SIZE │  NAME │ NUMBER OF CORES │                            PROXY ADDRESS                            │                 UUID                 │
    ├───────────┼─────────────┼───────┼─────────────────┼─────────────────────────────────────────────────────────────────────┼──────────────────────────────────────┤
    │ 10        │ 2048        │ vm-1  │ 2               │ https://2310f0f6-f62f-439a-80c3-0f6ce9ba3bbd.ainari.localhost:10042 │ 2310f0f6-f62f-439a-80c3-0f6ce9ba3bbd │
    │ 5         │ 1024        │ my-vm │ 1               │ https://943e56bb-c6e4-4716-a26c-f1fd46a45618.ainari.localhost:10042 │ 943e56bb-c6e4-4716-a26c-f1fd46a45618 │
    └───────────┴─────────────┴───────┴─────────────────┴─────────────────────────────────────────────────────────────────────┴──────────────────────────────────────┘
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
    #     "torii_address": "https://943e56bb-c6e4-4716-a26c-f1fd46a45618.ainari.localhost:10042",
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

    vm = virtual_machine.get_virtual_machine(context, virtual_machine_uuid)
    torii_address = vm["torii_address"]

    task = virtual_machine.stop_virtual_machine(context, torii_address, virtual_machine_uuid)
    task = virtual_machine.start_virtual_machine(context, torii_address, virtual_machine_uuid)
    task = virtual_machine.reboot_virtual_machine(context, torii_address, virtual_machine_uuid)
    ```

### Migrate virtual machine

!!! info

    Only admins are allowed to migrate virtual machines.

A virtual machine can be moved to another sakura-host, for example to empty a host for its
maintenance. It is a cold migration: the virtual machine is shut down gracefully, its disk is
transferred directly to the new host and it is booted there again, if it was running before. It
keeps its UUID, its internal IP, its `torii_address`, its floating IP and its
[network filters](#network-filters).

The migration runs in the background. While the virtual machine is moved, its `vm_state` is
`MIGRATING` and it can not be started, stopped, rebooted, snapshotted or deleted and its network
filters can not be changed. The new host has to have enough free resources for the virtual machine
and the same kind of isolation as its current host. If the migration fails, the virtual machine is
started on its current host again.

=== "CLI"

    ```bash
    ainarictl vm migrate -t <TARGET_HOST_UUID> <VIRTUAL_MACHINE_UUID>
    ```

    example:

    ```bash
    ainarictl vm migrate -t 8f5be498-a942-4b6a-9542-f43e9a53c3e2 943e56bb-c6e4-4716-a26c-f1fd46a45618

    ┌──────────────────────┬──────────────────────────────────────┐
    │ SOURCE HOST UUID     │ edbb59ce-2714-4d97-9d4b-3ab532c961d3 │
    │ TARGET HOST UUID     │ 8f5be498-a942-4b6a-9542-f43e9a53c3e2 │
    │ VIRTUAL MACHINE UUID │ 943e56bb-c6e4-4716-a26c-f1fd46a45618 │
    └──────────────────────┴──────────────────────────────────────┘
    ```

    Repeat `ainarictl vm get <VIRTUAL_MACHINE_UUID>`, until `VM STATE` is not `MIGRATING`
    anymore. The used resources of the old host in `ainarictl host list` are released at the very
    end of the migration.

=== "Python-SDK"

    ```python
    from ainari_sdk import virtual_machine

    result = virtual_machine.migrate_virtual_machine(context,
                                                     virtual_machine_uuid,
                                                     target_host_uuid)

    # example-content of result:
    #
    # {
    #     "virtual_machine_uuid": "943e56bb-c6e4-4716-a26c-f1fd46a45618",
    #     "source_host_uuid": "edbb59ce-2714-4d97-9d4b-3ab532c961d3",
    #     "target_host_uuid": "8f5be498-a942-4b6a-9542-f43e9a53c3e2"
    # }
    ```

    A second migration of the same virtual machine, while the first one runs, fails with a
    `ConflictException`, like a target host without enough free resources.

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

## Network filters

Network filters restrict the traffic of a virtual machine. Every virtual machine has two of them,
one per direction:

- `ingress` filters the traffic towards the virtual machine by its **source** address
- `egress` filters the traffic, which the virtual machine sends, by its **destination** address

A filter consists of two include-lists, ip-ranges and ports. As long as a list is empty, it
allows everything. As soon as it has an entry, only the listed addresses or ports are allowed:

- An ip-range is a single address (`10.0.0.7`), a subnet in CIDR notation (`10.0.0.0/24`) or an
  explicit range (`10.0.0.5-10.0.0.9`). Ranges are stored in their shortest notation, so
  `10.0.0.0-10.0.0.255` becomes `10.0.0.0/24`, and can be removed with every notation, which
  covers the same addresses.
- A port is a single port (`22`) or an explicit range (`8000-8100`). A TCP- or UDP-packet is
  allowed, if its source- *or* its destination-port is listed. Traffic without ports, like ICMP,
  is only checked against the ip-ranges.

The filters are stateless, so the answers of an allowed connection have to be allowed in the
other direction of the virtual machine as well, if this direction is filtered too. A filter,
whose lists are both empty again, is removed. The filters of a virtual machine are deleted
together with the virtual machine.

=== "CLI"

    ```bash
    ainarictl network_filter add_ip_range <VIRTUAL_MACHINE_UUID> <DIRECTION> <IP_RANGE> [<IP_RANGE> ...]
    ainarictl network_filter remove_ip_range <VIRTUAL_MACHINE_UUID> <DIRECTION> <IP_RANGE> [<IP_RANGE> ...]
    ainarictl network_filter add_port <VIRTUAL_MACHINE_UUID> <DIRECTION> <PORT> [<PORT> ...]
    ainarictl network_filter remove_port <VIRTUAL_MACHINE_UUID> <DIRECTION> <PORT> [<PORT> ...]
    ainarictl network_filter list
    ainarictl network_filter get <VIRTUAL_MACHINE_UUID> <DIRECTION>
    ainarictl network_filter delete <VIRTUAL_MACHINE_UUID> <DIRECTION>
    ```

    `<DIRECTION>` is `ingress` or `egress`.

    example:

    ```bash
    ainarictl network_filter add_ip_range a49ee919-6546-4081-9ee0-11e89f23f826 egress 0.0.0.0/0 10.0.0.0-10.0.0.255

    ┌──────────────────────┬──────────────────────────────────────┐
    │ CREATED AT           │ 2026-10-03T19:05:17.536720096Z       │
    │ CREATED BY           │ asdf                                 │
    │ DIRECTION            │ egress                               │
    │ IP RANGES            │ [0.0.0.0/0 10.0.0.0/24]              │
    │ PORTS                │ []                                   │
    │ UPDATED AT           │ 2026-10-03T19:05:17.536720597Z       │
    │ UPDATED BY           │ asdf                                 │
    │ UUID                 │ b7814a3a-97d7-4455-83b8-807d35252158 │
    │ VIRTUAL MACHINE UUID │ a49ee919-6546-4081-9ee0-11e89f23f826 │
    └──────────────────────┴──────────────────────────────────────┘

    ainarictl network_filter add_port a49ee919-6546-4081-9ee0-11e89f23f826 ingress 22 8000-8100

    ┌──────────────────────┬──────────────────────────────────────┐
    │ CREATED AT           │ 2026-10-03T19:05:17.484982382Z       │
    │ CREATED BY           │ asdf                                 │
    │ DIRECTION            │ ingress                              │
    │ IP RANGES            │ []                                   │
    │ PORTS                │ [22 8000-8100]                       │
    │ UPDATED AT           │ 2026-10-03T19:05:17.484982933Z       │
    │ UPDATED BY           │ asdf                                 │
    │ UUID                 │ a899d8a6-1e42-4140-8ccc-161ea3036c11 │
    │ VIRTUAL MACHINE UUID │ a49ee919-6546-4081-9ee0-11e89f23f826 │
    └──────────────────────┴──────────────────────────────────────┘

    ainarictl network_filter delete a49ee919-6546-4081-9ee0-11e89f23f826 egress

    successfully deleted egress network filter of virtual machine 'a49ee919-6546-4081-9ee0-11e89f23f826'
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import network_filter

    result = network_filter.add_network_filter_ip_ranges(context, virtual_machine_uuid, "egress",
                                                         ["0.0.0.0/0", "10.0.0.0-10.0.0.255"])
    result = network_filter.delete_network_filter_ip_ranges(context, virtual_machine_uuid,
                                                            "egress", ["10.0.0.0/24"])
    result = network_filter.add_network_filter_ports(context, virtual_machine_uuid, "ingress",
                                                     ["22", "8000-8100"])
    result = network_filter.delete_network_filter_ports(context, virtual_machine_uuid, "ingress",
                                                        ["8000-8100"])

    network_filter.list_network_filters(context)    # {"network_filters": [...]}
    network_filter.get_network_filter(context, virtual_machine_uuid, "ingress")
    network_filter.delete_network_filter(context, virtual_machine_uuid, "ingress")
    network_filter.delete_all_network_filters(context)

    # example-content of result:
    #
    # {
    #     "uuid": "b7814a3a-97d7-4455-83b8-807d35252158",
    #     "virtual_machine_uuid": "a49ee919-6546-4081-9ee0-11e89f23f826",
    #     "direction": "egress",
    #     "ip_ranges": ["0.0.0.0/0", "10.0.0.0/24"],
    #     "ports": [],
    #     ...
    # }
    ```

    `get_network_filter` raises a `NotFoundException`, if the direction of the virtual machine
    has no filter.

## Tasks

Actions, which take longer, run as tasks in the background on the sakura-host of a virtual machine:
the creation, start, stop and reboot of virtual machines and the snapshots. The steps of a
[migration](#migrate-virtual-machine) on the old and the new host are tasks as well
(`MigrationPrepare`, `MigrationImport`, `MigrationRemove` and `MigrationCancel`), which are started
by hanami. The `STATE` of a task
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

    The sakura-host is selected over the `torii_address` of a virtual machine on this host.

    ```python
    from ainari_sdk import task

    task.list_tasks(context, torii_address)    # {"tasks": [...]}
    task.get_task(context, torii_address, task_uuid)
    task.abort_task(context, torii_address, task_uuid)

    # wait until the task is done and check its result
    result = task.wait_for_task_finished(context, torii_address, task_uuid)
    if result["state"] != "Finished":
        print(result["messages"])
    ```

### Snapshots

A snapshot saves the root-disk of a virtual machine as new image with `is_snapshot` set. It can
be restored into a virtual machine later on. The root-disk is stored as qcow2-image, so the
`file_type` of a snapshot is `qcow2`. While the snapshot-task is still running, the image has the
`file_type` `unknown` and the `file_size` `0`. Its `image_status` is `ENCRYPTING`, while sakura copies
and encrypts the root-disk, then `UPLOADING`, while the encrypted file is stored, and `READY` at the
end. Only a `READY` snapshot can be restored.

!!! warning

    A running virtual machine is only paused during the copy, so data, which is still in its
    memory, is missing in the snapshot. Run `sync` inside the virtual machine right before creating
    the snapshot, or stop the virtual machine before.

!!! info

    Like an uploaded image, a snapshot is stored encrypted with a new generated secret. Alternatively
    an already existing [secret](#secrets) can be provided at the creation of the snapshot
    (`-s <SECRET_UUID>` in the CLI, `secret_uuid` in the SDK), which is used to encrypt the snapshot
    instead. Like a generated secret, it is deleted together with the snapshot.

=== "CLI"

    ```bash
    ainarictl task create snapshot_create [-s <SECRET_UUID>] <VIRTUAL_MACHINE_UUID> <SNAPSHOT_NAME>
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

    task.create_snapshot_save_task(context, torii_address, virtual_machine_uuid, "my-snapshot")
    # encrypt the snapshot with an already existing secret instead of a new generated one
    task.create_snapshot_save_task(context, torii_address, virtual_machine_uuid, "my-snapshot",
                                   secret_uuid=secret_uuid)
    task.create_snapshot_restore_task(context, torii_address, virtual_machine_uuid,
                                      snapshot_image_uuid)
    ```

## Backups

Isekai saves single virtual machines or whole projects as backups and restores them again. A
backup of a project contains its quota, secrets, public keys, images, networks, virtual machines,
network filters and floating IPs together with a copy of the root-disk of every virtual machine.
A backup of a virtual machine contains the virtual machine with its root-disk, public key,
network and network filters. Users and their assignments to the project are never part of a
backup.

A backup and a restore run as job in the background, which can be followed. During the job the
project is locked, so its resources can only be read and every other change is rejected with
`423`. A second backup or restore of the same project is rejected with `409`.

!!! warning

    A backup is encrypted with a key, which is derived from its passphrase. The passphrase is
    never stored, so a backup can't be restored without it. The CLI never takes it as argument,
    because it would be visible in the history of the shell. It is read from the file of
    `--passphrase_file`, from the env-variable `AINARI_BACKUP_PASSPHRASE` or from the terminal.

A restore always calculates its plan first: which resources get a new UUID, which are
overwritten, which are deleted and how the quota is used afterwards. With `--dry_run` only the
plan is shown.

- Without `--force` a resource keeps its UUID, if it is free, otherwise it gets a new one. Restored
  into another project (`--target_project`), all resources get new UUIDs.
- With `--force` the existing resources with the same UUID are overwritten.
- `--prune`, which is only allowed together with `--force` and only for a backup of a project,
  deletes the resources of the project, which are not part of the backup.

If a restore fails, the project stays locked, until the restore is repeated with `--force` or an
admin unlocks it with `ainarictl project unlock <PROJECT_ID>`.

=== "CLI"

    ```bash
    ainarictl backup create_project [--stop_vms] [--without_images] [-w] <BACKUP_NAME>
    ainarictl backup create_vm [--stop_vm] [-w] <VIRTUAL_MACHINE_UUID> <BACKUP_NAME>
    ainarictl backup list
    ainarictl backup get <BACKUP_UUID>
    ainarictl backup delete <BACKUP_UUID>
    ainarictl backup restore [--force] [--prune] [--target_project <PROJECT_ID>] [--start_vms] \
        [--without_dependencies] [--dry_run] [-w] <BACKUP_UUID>
    ainarictl backup job_list [--backup <BACKUP_UUID>]
    ainarictl backup job_get <JOB_UUID>

    # admin only
    ainarictl onsen_host set_backup_target <HOST_UUID> true|false
    ainarictl backup scan <ONSEN_HOST_UUID>
    ainarictl backup import [-n <NAME>] <ONSEN_HOST_UUID> <BACKUP_UUID>
    ainarictl project lock <PROJECT_ID> <REASON>
    ainarictl project unlock <PROJECT_ID>
    ```

    `-w` waits, until the job is finished, and fails, if the job failed.

    example:

    ```bash
    ainarictl backup restore --force --dry_run 87b415d3-5c04-4e9b-bb72-ccdf619765ab
    Enter passphrase of the backup:
    target project: default-asdf
    overwritten resources:
    ┌─────────────────────────────┬─────────────────┬──────────────────────────────────────┐
    │            NAME             │  RESOURCE TYPE  │                 UUID                 │
    ├─────────────────────────────┼─────────────────┼──────────────────────────────────────┤
    │ local-stack-test-5775b092-1 │ virtual_machine │ 33fb6332-7528-46a0-b4c2-aad32bf0b7dd │
    └─────────────────────────────┴─────────────────┴──────────────────────────────────────┘
    quota after the restore:
    ┌───────────────┬─────────┬─────┬─────────────────┐
    │ AFTER RESTORE │ CURRENT │ MAX │  RESOURCE TYPE  │
    ├───────────────┼─────────┼─────┼─────────────────┤
    │ 2             │ 2       │ 10  │ virtual_machine │
    │ 2             │ 2       │ 10  │ floating_ip     │
    │ 1             │ 1       │ 10  │ network         │
    └───────────────┴─────────┴─────┴─────────────────┘
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import backup

    result = backup.create_virtual_machine_backup(context, virtual_machine_uuid, "my-backup",
                                                  passphrase)
    job = backup.wait_for_job_finished(context, result["job"]["uuid"])

    # only the plan
    plan = backup.restore_backup(context, result["backup"]["uuid"], passphrase, force=True,
                                 dry_run=True)["plan"]
    restore = backup.restore_backup(context, result["backup"]["uuid"], passphrase, force=True)
    backup.wait_for_job_finished(context, restore["job"]["uuid"])
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

Every virtual machine gets a proxy on the torii automatically, which forwards to its sakura-host.
The proxy has the UUID of its virtual machine, so its address `https://<vm-uuid>.<domain>:<port>`
shows, which virtual machine it reaches. It is the `torii_address` of the virtual machine.
All proxies share one port: torii picks the proxy of a connection by the server name of its
TLS-connection, without terminating it. So the address stays the same, when the virtual machine
is moved to another sakura-host, and the clients never see the sakura-host itself. The proxies are
created and deleted together with the virtual machines, so they can only be listed and read. This
is done over hanami, because the api of torii is only reachable within the cluster.

=== "CLI"

    ```bash
    ainarictl proxy list
    ainarictl proxy get <PROXY_UUID>
    ```

    example:

    ```bash
    ainarictl proxy list

    ┌─────────────────────────────────────────────────────────────────────┬───────────────────────┬──────────────────────────────────────┬──────────────────────────────────────┐
    │                               ADDRESS                               │     TARGET ADDRESS    │                 UUID                 │         VIRTUAL MACHINE UUID         │
    ├─────────────────────────────────────────────────────────────────────┼───────────────────────┼──────────────────────────────────────┼──────────────────────────────────────┤
    │ https://2310f0f6-f62f-439a-80c3-0f6ce9ba3bbd.ainari.localhost:10042 │ https://sakura:9443   │ 2310f0f6-f62f-439a-80c3-0f6ce9ba3bbd │ 2310f0f6-f62f-439a-80c3-0f6ce9ba3bbd │
    │ https://070af3fd-1a37-4546-b3e2-035e03b3bd65.ainari.localhost:10042 │ https://sakura-2:9443 │ 070af3fd-1a37-4546-b3e2-035e03b3bd65 │ 070af3fd-1a37-4546-b3e2-035e03b3bd65 │
    └─────────────────────────────────────────────────────────────────────┴───────────────────────┴──────────────────────────────────────┴──────────────────────────────────────┘
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import proxy

    proxy.list_proxys(context)    # {"proxys": [...]}
    proxy.get_proxy(context, proxy_uuid)
    ```

## Projects

Projects are used for logical separation of the resources of users.

!!! info

    Only admins are allowed to manage projects and their users. Listing the users of the project
    of the current login is allowed for every user of this project.

!!! info "Deleting a project"

    Only projects without any resources can be deleted. Before a project is deleted, it is checked,
    if there are still virtual machines, networks, floating IPs, images, secrets or public-keys
    within the project. If there is at least one of them, the delete is rejected with a conflict
    and the remaining resources are listed in the error-message. They have to be deleted first.

    Additionally, no user with the role `admin` or `member` may be assigned to the project anymore,
    because these users could create a new resource at the same time as the delete. They have to
    be removed from the project with `remove_user_from_project` or made observers with
    `set_project_role` first, otherwise the delete is rejected with a conflict, which names them.
    Observers can't create resources, so they don't block the delete.

    Together with the project, its quota and the assignments of all of its users are deleted.

    The default-project `default-<USER_ID>` of a user can never be deleted directly, it is only
    deleted together with its user. So every user always has its default-project. Within its own
    default-project, a user can also not be removed with `remove_user_from_project` or made an
    observer with `set_project_role`.

=== "CLI"

    ```bash
    ainarictl project create -n <NAME> <PROJECT_ID>
    ainarictl project list
    ainarictl project get <PROJECT_ID>
    ainarictl project delete <PROJECT_ID>
    ainarictl project add_user_to_project <PROJECT_ID> <USER_ID> <PROJECT_ROLE>
    ainarictl project remove_user_from_project <PROJECT_ID> <USER_ID>
    ainarictl project list_users_in_project_admin <PROJECT_ID>

    # every user, not only admins: users of the project of the current login and the role of
    # each of them
    ainarictl project list_users_in_project
    ```

    `<PROJECT_ROLE>` is one of `admin`, `member` or `observer`. A user can be added to the same
    project only once at the same time.

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

    project.add_user_to_project(context, "my_project", "my_user", "member")
    # {"user_id": "my_user", "project_id": "my_project", "project_role": "member"}
    project.list_users_in_project_admin(context, "my_project")
    # {"members": [{"user_id": "my_user", "project_role": "member"}, ...]}
    project.remove_user_from_project(context, "my_project", "my_user")

    # every user, not only admins: users of the project of the context and the role of each of
    # them
    project.list_users_in_project(context)
    # {"members": [{"user_id": "my_user", "project_role": "member"}, ...]}
    ```

## Users

!!! info

    Only admins are allowed to manage users.

!!! info "Deleting a user"

    A user can not delete himself, this is rejected with a conflict.

    A user can only be deleted, if its default-project `default-<USER_ID>` contains no resources
    (virtual machines, networks, floating IPs, images, secrets or public-keys) anymore. Otherwise
    the delete is rejected with a conflict and the remaining resources are named in the
    error-message. The resources of other projects are not checked.

    To prevent, that a new resource is created in the default-project during the delete, all users
    with the role `admin` or `member` in the default-project are made observers first, which
    invalidates their tokens. After a short wait for requests, which were still running, the
    default-project is checked again. If it contains resources now, the old roles are restored and
    the delete is rejected as well.

    Then the user is deleted together with its default-project, including its quota and the
    assignments of all of its users. The user is removed from all other projects, but these
    projects are never deleted, also if the user was the only user there.

!!! info "Changing the passphrase"

    Every user can change its own passphrase by providing the old and the new passphrase. An admin
    can set a new passphrase for any user without the old passphrase. The new passphrase is used
    for all future logins, already existing tokens stay valid.

=== "CLI"

    ```bash
    ainarictl user create -n <NAME> [--is_admin] <USER_ID>
    ainarictl user list
    ainarictl user get <USER_ID>
    ainarictl user delete <USER_ID>
    ainarictl user set_project_role <USER_ID> <PROJECT_ID> <PROJECT_ROLE>

    ainarictl user change_passphrase_admin <USER_ID>

    # every user, not only admins: own projects and the role in each of them
    ainarictl user list_user_projects

    # every user, not only admins: change the own passphrase
    ainarictl user change_passphrase
    ```

    `<PROJECT_ROLE>` is one of `admin`, `member` or `observer`. `set_project_role` only changes the
    role of a user, who was already added to the project.

    Without `-p <PASSPHRASE>` the passphrase is requested interactively, where a new passphrase has
    to be entered two times. For `change_passphrase` the old passphrase is requested as well, if
    not given by `--old_passphrase <OLD_PASSPHRASE>`. These flags should only be used for automated
    testing, because the passphrase is visible in the command-line and the shell-history.

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

    user.set_project_role(context, "my_user", "my_project", "observer")
    user.change_passphrase_admin(context, "my_user", "new-passphrase")

    # every user, not only admins: own projects and the role in each of them
    user.list_user_projects(context)
    # {"projects": [{"project_id": "default-my_user", "project_role": "admin"}, ...]}

    # every user, not only admins: change the own passphrase
    user.change_passphrase(context, "old-passphrase", "new-passphrase")
    ```

## Quotas

Maximum number of resources per project. The resources of all users of a project count against the
same quota. Every project gets a quota with a limit of 10 for each resource, when it is created.
Every user can see the quota of the project of the current login, only admins can see and set the
quotas of other projects.

=== "CLI"

    ```bash
    # quota of the project of the current login
    ainarictl quota show

    # admin only
    ainarictl quota list
    ainarictl quota get <PROJECT_ID>
    ainarictl quota set <PROJECT_ID> \
        --max_virtual_machine <N> --max_image <N> --max_secret <N> \
        --max_network <N> --max_floating_ip <N>
    ```

    example:

    ```bash
    ainarictl quota list

    ┌─────────────────┬───────────┬─────────────┬────────────┬─────────────────────┬──────────────┐
    │ MAX FLOATING IP │ MAX IMAGE │ MAX NETWORK │ MAX SECRET │ MAX VIRTUAL MACHINE │ PROJECT ID   │
    ├─────────────────┼───────────┼─────────────┼────────────┼─────────────────────┼──────────────┤
    │ 10              │ 10        │ 10          │ 10         │ 10                  │ default-asdf │
    │ 2               │ 5         │ 2           │ 5          │ 5                   │ my_project   │
    └─────────────────┴───────────┴─────────────┴────────────┴─────────────────────┴──────────────┘
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import quota

    quota.get_own_quota(context)

    # admin only
    quota.list_quotas(context)    # {"quotas": [...]}
    quota.get_quota(context, "my_project")
    quota.set_quota(context, "my_project",
                    5,    # max_virtual_machine
                    5,    # max_image
                    5,    # max_secret
                    2,    # max_network
                    2)    # max_floating_ip
    ```
