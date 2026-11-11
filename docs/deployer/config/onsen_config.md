# Onsen

The config is read from `/etc/ainari/onsen.toml`. The path can be overwritten with the
environment-variable `CONFIG_FILE`.

## Options

### Root Configuration

| Parameter               | Type    | Default    | Description                                                                                        |
| ----------------------- | ------- | ---------- | -------------------------------------------------------------------------------------------------- |
| `debug`                 | boolean | _required_ | Enables debug mode for detailed logging and troubleshooting.                                       |
| `log_type` | string | `"stdout"` | Target of the logs: `"stdout"` or `"log_file"`. |
| `log_path` | string | `"/var/log/"` | Directory of the log-file `<service>.log`, if `log_type` is `"log_file"`. |
| `skip_tls_verification` | boolean | `false`    | Set true to skip validation of https-connections, for example in case of self-singed certificates. |
| `address`               | string  | _required_ | Address of the internal api of the onsen-host itself, which is registered at ryokan and where it is reached by ryokan and sakura, for example `https://onsen-0.onsen.ainari.svc.cluster.local:8443`. |

### `api` Configuration

All endpoints, which store and read the files, are internal and only reachable over the internal
port. The public port only provides the version, the ready-status and the openapi-specification.

| Parameter       | Type    | Default    | Description                         |
| --------------- | ------- | ---------- | ----------------------------------- |
| `public_ip`     | string  | _required_ | IP address for public API access.   |
| `public_port`   | integer | _required_ | Port for public API access.         |
| `internal_ip`   | string  | _required_ | IP address for internal API access. |
| `internal_port` | integer | _required_ | Port for internal API access.       |

### `miko` Configuration

| Parameter | Type   | Default    | Description                  |
| --------- | ------ | ---------- | ---------------------------- |
| `address` | string | _required_ | Address of the Miko service. |

### `storage` Configuration

Every onsen uses exactly one storage-backend, which is selected by `backend`. The section of the
selected backend must exist, otherwise the onsen stops at the startup with an error.

| Parameter  | Type   | Default   | Description                                                                                      |
| ---------- | ------ | --------- | ------------------------------------------------------------------------------------------------ |
| `backend`  | string | `"local"` | Storage-backend of this onsen. Only `"local"` is available at the moment.                         |
| `location` | string | –         | **Deprecated**, use `[storage.local] location` instead. It is still read and mapped to the local backend, but can't be combined with the section `[storage.local]`. |

### `storage.local` Configuration

Local storage-backend, which stores the files on the local disk of the onsen-host.

The volumes, which the sakura-hosts attach as disks of the virtual machines, are sparse files in
the subdirectory `volumes` of `location`. This directory is reserved for the volumes and can't be
accessed over the file-API. The onsen doesn't export it itself, so it doesn't need
root-permissions: the deployer exports it read-write over NFS to the network of the sakura-hosts
(for example in `/etc/exports`) and sets `nfs_server` and `nfs_export`. Without both values the
onsen doesn't support volumes. The content of the volumes is always encrypted by the sakura-hosts,
so the export only contains cipher-text. This backend has no redundancy and is meant for testing.

| Parameter    | Type   | Default    | Description                                                                 |
| ------------ | ------ | ---------- | --------------------------------------------------------------------------- |
| `location`   | string | _required_ | Directory where files will be stored. It is created, if it doesn't exist.    |
| `nfs_server` | string | –          | Address of the NFS-server, which exports `<location>/volumes` to the sakura-hosts. Can only be set together with `nfs_export`. |
| `nfs_export` | string | –          | Path of the NFS-export of `<location>/volumes` on the NFS-server, which the sakura-hosts mount. Can only be set together with `nfs_server`. |

## Example

!!! info

    example config-file can be found in the repository under `example_configs/ainari/`

```toml
--8<-- "example_configs/ainari/onsen.toml"
```
