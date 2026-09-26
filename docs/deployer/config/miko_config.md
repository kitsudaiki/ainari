# Miko

The config is read from `/etc/ainari/miko.toml`. The path can be overwritten with the
environment-variable `CONFIG_FILE`.

## Options

### Root Configuration

| Parameter  | Type    | Default    | Description                                                                                   |
| ---------- | ------- | ---------- | --------------------------------------------------------------------------------------------- |
| `debug`    | boolean | _required_ | Enables debug mode for detailed logging and troubleshooting.                                  |
| `log_path` | string  | `"/var/log/"` | Path to the directory where log files will be stored. Currently not evaluated by the service. |

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

### `auth` Configuration

| Parameter           | Type    | Default    | Description                                                                                      |
| ------------------- | ------- | ---------- | ------------------------------------------------------------------------------------------------ |
| `token_key_path`    | string  | _required_ | Path to the file containing the key, which signs the tokens. See [Token-Key](token_key.md).      |
| `token_expire_time` | integer | _required_ | Token expiration time in seconds.                                                                |

### `endpoints` Configuration

All four endpoints are required. Each of them has the same two parameters:

| Parameter          | Type   | Default    | Description                           |
| ------------------ | ------ | ---------- | ------------------------------------- |
| `public_address`   | string | _required_ | Public address of the service.        |
| `internal_address` | string | _required_ | Internal address of the service.      |

| Section              | Service                    |
| -------------------- | -------------------------- |
| `endpoints.hanami`   | [Hanami](hanami_config.md) |
| `endpoints.ryokan`   | [Ryokan](ryokan_config.md) |
| `endpoints.torii`    | [Torii](torii_config.md)   |
| `endpoints.omamori`  | [Omamori](omamori_config.md) |

## Example

!!! info

    example config-file can be found in the repository under `example_configs/ainari/`

```toml
--8<-- "example_configs/ainari/miko.toml"
```
