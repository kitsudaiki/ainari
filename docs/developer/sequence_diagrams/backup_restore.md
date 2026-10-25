# Restore of a backup

Isekai restores a backup, which was created by [the backup of a project or a virtual
machine](backup_create.md), into its own project or into another one. The restore never changes
the package of the backup: the disks and images are copied out of it to normal onsen-hosts, so the
same backup can be restored again and again.

The request first verifies the backup and calculates the plan of the restore, before anything is
changed. The restore itself runs as job in the background, while the target-project is locked.

## Components

| Component | Role |
|---|---|
| **isekai** | Verifies the manifest and decrypts the metadata, calculates the plan, locks the target-project and runs the job, which calls all other components. |
| **miko** | Holds the lock of the target-project, creates a missing project and gives an admin a token for the target-project. |
| **ryokan** | Reads the files of the package, copies the disks and images out of it to normal onsen-hosts and registers them as images of the project. |
| **onsen** | The backup-target, which holds the package, and the normal onsen-hosts, which get the copies. |
| **omamori** | Unwraps the payloads of the secrets with the wrap-key of the backup and stores them encrypted with its own key. Imports the public-keys. |
| **hanami** | Imports the networks, network-filters and floating IPs and reserves the virtual machines with their original UUID and addresses, if they are free. |
| **sakura** | Creates a new virtual machine from the restored disk or replaces the root-disk of an existing one. |

```mermaid
flowchart LR
    user["user"]
    isekai["isekai<br/>plan, job"]
    miko["miko<br/>lock, project, token"]
    ryokan["ryokan<br/>package, copies, images"]
    backup["onsen<br/>backup-target"]
    onsen["onsen<br/>normal host"]
    omamori["omamori<br/>secrets, public-keys"]
    hanami["hanami<br/>networks, filters, reservation"]
    sakura["sakura<br/>create, replace disk"]

    user -- "POST /v1alpha/backup/{uuid}/restore" --> isekai
    isekai -- "lock, import, token" --> miko
    isekai -- "files, copies, import" --> ryokan
    ryokan -- "read" --> backup
    ryokan -- "write copies" --> onsen
    isekai -- "import" --> omamori
    isekai -- "import, reserve" --> hanami
    isekai -- "tasks" --> sakura
    sakura -- "download disk" --> onsen
```

## Options

| Option | Effect |
|---|---|
| `force` | Existing resources with the same UUID are overwritten. Without it, a resource, whose UUID is already used, gets a new UUID. |
| `prune` | Only together with `force` and only for a backup of a project: the resources of the project, which are not part of the backup, are deleted. |
| `target_project_id` | Restores into another project. All resources get new UUIDs there, because their UUIDs are usually still used by the project of the backup. A missing project is created, which only an admin can do. |
| `start_vms` | Starts all restored virtual machines. Without it, a virtual machine, which was stopped at the time of the backup, is stopped again. |
| `include_dependencies` | Only for a backup of a virtual machine: a missing network is created from the backup. |
| `dry_run` | Only returns the plan, without changing anything. |

## Verification and plan

Everything up to the plan only reads. A wrong passphrase or a manipulated metadata-file is
rejected with `400`, before the project is locked or changed.

```mermaid
sequenceDiagram
    actor U as user
    participant I as isekai
    participant DB as isekai database
    participant R as ryokan
    participant M as miko
    participant O as omamori
    participant H as hanami

    U->>I: POST /v1alpha/backup/{uuid}/restore<br/>{passphrase, force, prune, target_project_id,<br/>start_vms, include_dependencies, dry_run}
    I->>I: validate the input (400),<br/>only admins of the project (403)
    I->>DB: backup (404), must be READY (409)
    I->>I: target-project and the allowed options:<br/>prune only with force, another project<br/>only for admins (400, 403)
    I->>R: GET /backup/{uuid}/file/internal?path=manifest.json
    R-->>I: manifest
    I->>I: derive the keys with the salt of the manifest
    I->>I: check the HMAC of the manifest<br/>(wrong passphrase: 400)
    loop every metadata-file of the manifest
        I->>R: GET /backup/{uuid}/file/internal?path=...
        R-->>I: encrypted file
        I->>I: check size and sha256, decrypt (400)
    end
    par current state of the target-project
        I->>M: GET /project/{project_id}/export/internal
        I->>O: POST /project/{project_id}/export/internal {wrap_key}
        I->>R: GET /project/{project_id}/export/internal
        I->>H: GET /project/{project_id}/export/internal
    end
    I->>I: calculate the plan: id-map, overwritten,<br/>deleted (prune), quota, warnings
    alt dry_run
        I-->>U: 201 {plan, job: null}
    end
```

The plan decides the UUID of every resource of the backup:

- with `force` every UUID is kept and an existing resource is overwritten,
- without `force` a resource keeps its UUID, if it is free in the project, otherwise it gets a new
  one, which is stored in the id-map,
- restored into another project, every resource gets a new UUID.

All references between the resources, like the network of a virtual machine or the secret of an
image, are rewritten with the id-map. The quota of the project after the restore is calculated as
well, a restore, which would exceed it, is rejected with `409`.

## Start of the restore

```mermaid
sequenceDiagram
    actor U as user
    participant I as isekai
    participant DB as isekai database
    participant M as miko

    opt the target-project doesn't exist
        I->>M: POST /project/{project_id}/import/internal<br/>{name and quota of the backup}
    end
    I->>M: POST /project/{project_id}/lock/internal<br/>{job_uuid, reason: "restore of backup <uuid>"}
    alt locked by a failed restore of the same backup and force
        M-->>I: 409
        I->>M: DELETE /project/{project_id}/lock/internal {failed job}
        I->>M: POST /project/{project_id}/lock/internal {job_uuid}
    else locked by another job or an admin
        M-->>I: 409
        I-->>U: 409 Conflict
    end
    opt restore into another project
        I->>M: PUT /v1alpha/token/project/admin {project_id}
        M-->>I: token of the admin for the target-project
    end
    I->>DB: job (QUEUED)
    I->>I: add the job with the keys and the plan to the queue
    I-->>U: 201 {plan, job}
```

The other components act on the project of the token, for example hanami reserves a new virtual
machine within it. So a restore into another project gets a token of the admin for this project,
also if the admin is not assigned to it.

A failed restore keeps the project locked. Only a second restore of the same backup with `force`
takes over its lock, because only the job, which holds a lock, can remove it. Every other lock,
like the one of a running backup or of an admin, is never taken over.

## The job

The job checks the disks and images of the package against the manifest first, so a manipulated
package changes nothing. If this check fails, the project is unlocked again, because it wasn't
changed yet.

```mermaid
sequenceDiagram
    participant J as restore-job
    participant R as ryokan
    participant M as miko

    loop every disk and image of the manifest
        J->>R: GET /backup/{uuid}/file_info/internal?path=...
        R-->>J: size and sha256
        J->>J: compare with the manifest
    end
    alt a file is missing or changed
        J->>M: DELETE /project/{project_id}/lock/internal {job_uuid}
        J->>J: job ERROR
    else
        Note over J: backup of a project or of a virtual machine, see below
        J->>M: DELETE /project/{project_id}/lock/internal {job_uuid}
        J->>J: job FINISHED with the id-map
    end
```

### Restore of a project

The resources are restored in the order of their dependencies.

```mermaid
sequenceDiagram
    participant J as restore-job
    participant M as miko
    participant O as omamori
    participant R as ryokan
    participant H as hanami

    opt force and the project existed before
        J->>M: POST /project/{project_id}/import/internal<br/>{name, quota, force}
    end
    opt prune
        loop resources of the plan in reverse dependency-order
            J->>H: DELETE floating IPs, network-filters, VMs, networks
            J->>R: DELETE images
            J->>O: DELETE public-keys and secrets
        end
    end
    J->>O: POST /project/{project_id}/import/internal<br/>{wrap_key, secrets, public_keys, force, id_map}
    O->>O: unwrap all payloads and check all conflicts,<br/>before anything is stored
    opt images were included in the backup
        loop every image
            J->>R: POST /backup/{uuid}/copy/internal<br/>{OUT_OF_BACKUP, images/<image_uuid>.enc, new file-UUID}
            R-->>J: onsen-address, path and size of the copy
        end
        J->>R: POST /project/{project_id}/import/internal<br/>{images with the locations of their copies, force, id_map}
    end
    J->>H: POST /project/{project_id}/import/internal<br/>{networks, force, id_map}
    loop every VM
        Note over J,H: restore of the virtual machine, see below
    end
    J->>H: POST /project/{project_id}/import/internal<br/>{network_filters, floating_ips, force, id_map}
    H->>H: new floating IPs out of the pool,<br/>attached to the restored VMs
    loop every VM, which was stopped at the time of the backup
        J->>J: stop the VM again, unless start_vms
    end
```

Floating IPs are always allocated again from the pool of the target. Their old address is only a
preference, so the restore can also be imported into another deployment.

### Restore of a virtual machine

A virtual machine of a backup of a project, and the one of a backup of a virtual machine, is
restored from its disk in the package:

```mermaid
sequenceDiagram
    participant J as restore-job
    participant R as ryokan
    participant O as omamori
    participant H as hanami
    participant S as sakura

    J->>R: POST /backup/{uuid}/copy/internal<br/>{OUT_OF_BACKUP, disks/<vm_uuid>.enc, new image-UUID}
    R-->>J: onsen-address, path and size of the copy
    J->>O: POST /project/{project_id}/import/internal<br/>{secret of the disk as secret of the image}
    J->>R: POST /project/{project_id}/import/internal<br/>{snapshot-image at the location of the copy}
    alt the VM exists with the same disk-size and is overwritten
        J->>H: GET /virtual_machine/{uuid}/host/internal
        J->>S: POST /virtual_machine/{uuid}/backup_restore/internal<br/>{backup_uuid, image_uuid}
        S->>S: task BackupRestore: replace the root-disk<br/>with the snapshot
    else new VM, or other disk-size (deleted before)
        J->>H: POST /v1alpha/virtual_machine/internal<br/>{uuid, cores, memory, disk_size, network,<br/>internal_ip, mac_address}
        alt UUID or addresses are taken
            H-->>J: 409
            J->>H: again without the addresses, then without the UUID
        end
        H-->>J: reserved VM
        J->>H: GET /virtual_machine/{uuid}/host/internal
        J->>S: create the VM {image_uuid, public_key_uuid}
        S->>S: download the snapshot, decrypt it,<br/>new seed-image, boot
    end
    loop until the task is done
        J->>S: GET /v1alpha/task/{task_uuid}
    end
    J->>R: DELETE the temporary snapshot-image
    J->>J: store the new UUID of the VM in the id-map, if it changed
```

The snapshot-image is only needed to create the root-disk, so it is deleted again afterwards. A
new virtual machine gets a new seed-image, so its cloud-init runs with the restored public-key.

For a backup of a virtual machine, its public-key and its network are restored first, if they
don't exist in the target-project anymore. They get a new UUID, if their UUID is used by another
project. A missing network is only created, if `include_dependencies` is set. Afterwards the
network-filters of the virtual machine are imported and its power-state is applied like for a
project.

## Failure of the job

The steps are idempotent, so a failed restore can be repeated. The job ends in `ERROR` with the
step, which failed, and its message. The resources, which were already restored, stay, and the
project stays locked, so nobody works on a half restored project.

```mermaid
stateDiagram-v2
    [*] --> locked: restore started
    locked --> unlocked: job FINISHED
    locked --> unlocked: files of the package changed (nothing restored)
    locked --> locked_failed: job ERROR after the first change
    locked_failed --> locked: restore of the same backup with force<br/>takes over the lock
    locked_failed --> unlocked: admin removes the lock<br/>(DELETE /project/{id}/lock/admin)
    locked: locked by the restore-job
    locked_failed: locked by the failed job
    unlocked: unlocked
```

A restart of isekai sets an active restore-job to `ERROR`. Its project stays locked like after
every failed restore.
