# Sakura

The config is read from `/etc/ainari/sakura.toml`. The path can be overwritten with the
environment-variable `CONFIG_FILE`.

## Options

### Root Configuration

| Parameter               | Type    | Default    | Description                                                                                        |
| ----------------------- | ------- | ---------- | -------------------------------------------------------------------------------------------------- |
| `debug`                 | boolean | _required_ | Enables debug mode for detailed logging and troubleshooting.                                       |
| `log_path`              | string  | `"/var/log/"` | Path to the directory where log files will be stored. Currently not evaluated by the service.   |
| `skip_tls_verification` | boolean | `false`    | Set true to skip validation of https-connections, for example in case of self-singed certificates. |
| `address`               | string  | _required_ | Address of the sakura-host itself, where it can be reached from hanami and torii.                  |

### `api` Configuration

| Parameter       | Type    | Default    | Description                         |
| --------------- | ------- | ---------- | ----------------------------------- |
| `public_ip`     | string  | _required_ | IP address for public API access.   |
| `public_port`   | integer | _required_ | Port for public API access.         |
| `internal_ip`   | string  | _required_ | IP address for internal API access. |
| `internal_port` | integer | _required_ | Port for internal API access.       |

### `database` Configuration

| Parameter   | Type   | Default    | Description                |
| ----------- | ------ | ---------- | -------------------------- |
| `file_path` | string | _required_ | Path to the database file. |

### `miko` Configuration

| Parameter | Type   | Default    | Description                  |
| --------- | ------ | ---------- | ---------------------------- |
| `address` | string | _required_ | Address of the Miko service. |

### `processing` Configuration

| Parameter               | Type    | Default | Description                                                                 |
| ----------------------- | ------- | ------- | --------------------------------------------------------------------------- |
| `max_number_of_threads` | integer | `0`     | Maximum number of threads for processing. A value of 0 means no limitation. |

### `storage` Configuration

| Parameter               | Type   | Default    | Description                                                  |
| ----------------------- | ------ | ---------- | ------------------------------------------------------------ |
| `local_vm_storage_path` | string | _required_ | Directory where the files of the virtual machines are stored. |
| `tempfile_location`     | string | _required_ | Directory where temporary files will be stored.              |

### `host` Configuration

Resources, which are reserved for the host itself, for example for the operating-system. They are
subtracted from the resources of the host, before these are reported to hanami, so they are not
available for virtual machines. The whole section is optional.

| Parameter         | Type    | Default | Description                                  |
| ----------------- | ------- | ------- | -------------------------------------------- |
| `reserved_cores`  | integer | `0`     | Number of cpu-threads reserved for the host. |
| `reserved_memory` | integer | `0`     | Memory in MiB reserved for the host.         |
| `reserved_disk`   | integer | `0`     | Disk-space in GiB reserved for the host.     |

### `hypervisor` Configuration

The hypervisor, which runs the virtual machines. The whole section is optional.

| Parameter       | Type   | Default                             | Description                                       |
| --------------- | ------ | ----------------------------------- | ------------------------------------------------- |
| `binary_path`   | string | `"/usr/local/bin/cloud-hypervisor"` | Path of the cloud-hypervisor binary.              |
| `firmware_path` | string | `"/usr/local/share/CLOUDHV.fd"`     | Path of the firmware, which boots the virtual machines. |

## Example

!!! info

    example config-file can be found in the repository under `example_configs/ainari/`

```toml
--8<-- "example_configs/ainari/sakura.toml"
```
