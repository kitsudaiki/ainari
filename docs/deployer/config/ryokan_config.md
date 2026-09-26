# Ryokan

The config is read from `/etc/ainari/ryokan.toml`. The path can be overwritten with the
environment-variable `CONFIG_FILE`.

## Options

### Root Configuration

| Parameter               | Type    | Default    | Description                                                                                        |
| ----------------------- | ------- | ---------- | -------------------------------------------------------------------------------------------------- |
| `debug`                 | boolean | _required_ | Enables debug mode for detailed logging and troubleshooting.                                       |
| `log_path`              | string  | `"/var/log/"` | Path to the directory where log files will be stored. Currently not evaluated by the service.   |
| `skip_tls_verification` | boolean | `false`    | Set true to skip validation of https-connections, for example in case of self-singed certificates. |

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

### `storage` Configuration

| Parameter           | Type   | Default    | Description                                     |
| ------------------- | ------ | ---------- | ----------------------------------------------- |
| `tempfile_location` | string | _required_ | Directory where temporary files will be stored. |

## Example

!!! info

    example config-file can be found in the repository under `example_configs/ainari/`

```toml
--8<-- "example_configs/ainari/ryokan.toml"
```
