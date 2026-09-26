# Onsen

The config is read from `/etc/ainari/onsen.toml`. The path can be overwritten with the
environment-variable `CONFIG_FILE`.

## Options

### Root Configuration

| Parameter               | Type    | Default    | Description                                                                                        |
| ----------------------- | ------- | ---------- | -------------------------------------------------------------------------------------------------- |
| `debug`                 | boolean | _required_ | Enables debug mode for detailed logging and troubleshooting.                                       |
| `log_path`              | string  | `"/var/log/"` | Path to the directory where log files will be stored. Currently not evaluated by the service.   |
| `skip_tls_verification` | boolean | `false`    | Set true to skip validation of https-connections, for example in case of self-singed certificates. |
| `address`               | string  | _required_ | Address of the onsen-host itself, where it can be reached from the ryokan and sakura.              |

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
