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

| Parameter  | Type   | Default    | Description                           |
| ---------- | ------ | ---------- | ------------------------------------- |
| `location` | string | _required_ | Directory where files will be stored. |

## Example

!!! info

    example config-file can be found in the repository under `example_configs/ainari/`

```toml
--8<-- "example_configs/ainari/onsen.toml"
```
