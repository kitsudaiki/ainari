# Sakura

## Options

### Root Configuration

| Parameter               | Type    | Default                    | Description                                                                                        |
| ----------------------- | ------- | -------------------------- | -------------------------------------------------------------------------------------------------- |
| `debug`                 | boolean | `true`                     | Enables debug mode for detailed logging and troubleshooting.                                       |
| `log_path`              | string  | `"/var/log"`               | Path to the directory where log files will be stored.                                              |
| `skip_tls_verification` | boolean | `true`                     | Set true to skip validation of https-connections, for example in case of self-singed certificates. |
| `address`               | string  | `"http://127.0.0.1:11420"` | Address of the sakura-host itself, where it can be reached from hanami and torii.                  |

### `api` Configuration

| Parameter       | Type    | Default     | Description                         |
| --------------- | ------- | ----------- | ----------------------------------- |
| `public_ip`     | string  | `"0.0.0.0"` | IP address for public API access.   |
| `public_port`   | integer | `11420`     | Port for public API access.         |
| `internal_ip`   | string  | `"0.0.0.0"` | IP address for internal API access. |
| `internal_port` | integer | `10420`     | Port for internal API access.       |

### `database` Configuration

| Parameter   | Type   | Default                   | Description                |
| ----------- | ------ | ------------------------- | -------------------------- |
| `file_path` | string | `"/etc/ainari/sakura_db"` | Path to the database file. |

### `miko` Configuration

| Parameter | Type   | Default                    | Description                  |
| --------- | ------ | -------------------------- | ---------------------------- |
| `address` | string | `"http://127.0.0.1:11417"` | Address of the Miko service. |

### `processing` Configuration

| Parameter               | Type    | Default | Description                                                                 |
| ----------------------- | ------- | ------- | --------------------------------------------------------------------------- |
| `max_number_of_threads` | integer | `0`     | Maximum number of threads for processing. A value of 0 means no limitation. |

### `storage` Configuration

| Parameter           | Type   | Default         | Description                                     |
| ------------------- | ------ | --------------- | ----------------------------------------------- |
| `tempfile_location` | string | `"/tmp/sakura"` | Directory where temporary files will be stored. |

### `host` Configuration

Resources, which are reserved for the host itself, for example for the operating-system. They are
subtracted from the resources of the host, before these are reported to hanami, so they are not
available for virtual machines. The whole section is optional.

| Parameter         | Type    | Default | Description                                  |
| ----------------- | ------- | ------- | -------------------------------------------- |
| `reserved_cores`  | integer | `0`     | Number of cpu-threads reserved for the host. |
| `reserved_memory` | integer | `0`     | Memory in MiB reserved for the host.         |
| `reserved_disk`   | integer | `0`     | Disk-space in GiB reserved for the host.     |

## Example

!!! info

    example config-file can be found in the repository under `example_configs/ainari/`

```toml
--8<-- "example_configs/ainari/hanami.toml"
```
