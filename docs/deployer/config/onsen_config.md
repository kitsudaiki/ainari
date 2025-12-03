# Onsen

## Options

### Root Configuration

| Parameter               | Type    | Default                    | Description                                                                                        |
| ----------------------- | ------- | -------------------------- | -------------------------------------------------------------------------------------------------- |
| `debug`                 | boolean | `true`                     | Enables debug mode for detailed logging and troubleshooting.                                       |
| `log_path`              | string  | `"/var/log"`               | Path to the directory where log files will be stored.                                              |
| `skip_tls_verification` | boolean | `true`                     | Set true to skip validation of https-connections, for example in case of self-singed certificates. |
| `address`               | string  | `"http://127.0.0.1:50051"` | Address of the onsen-host itself, where it can be reached from the ryokan and sakura.              |

### `miko` Configuration

| Parameter | Type   | Default                    | Description                  |
| --------- | ------ | -------------------------- | ---------------------------- |
| `address` | string | `"http://127.0.0.1:11417"` | Address of the Miko service. |

### `storage` Configuration

| Parameter  | Type   | Default        | Description                           |
| ---------- | ------ | -------------- | ------------------------------------- |
| `location` | string | `"/tmp/onsen"` | Directory where files will be stored. |

## Example

!!! info

    example config-file can be found in the repository under `example_configs/ainari/`

```toml
--8<-- "example_configs/ainari/onsen.toml"
```
