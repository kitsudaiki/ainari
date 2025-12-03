# Omamori

## Options

### Root Configuration

| Parameter               | Type    | Default      | Description                                                                                        |
| ----------------------- | ------- | ------------ | -------------------------------------------------------------------------------------------------- |
| `debug`                 | boolean | `true`       | Enables debug mode for detailed logging and troubleshooting.                                       |
| `log_path`              | string  | `"/var/log"` | Path to the directory where log files will be stored.                                              |
| `skip_tls_verification` | boolean | `true`       | Set true to skip validation of https-connections, for example in case of self-singed certificates. |

### `api` Configuration

| Parameter          | Type    | Default     | Description                               |
| ------------------ | ------- | ----------- | ----------------------------------------- |
| `public_ip`        | string  | `"0.0.0.0"` | IP address for public API access.         |
| `public_port`      | integer | `11421`     | Port for public API access.               |
| `internal_ip`      | string  | `"0.0.0.0"` | IP address for internal API access.       |
| `internal_port`    | integer | `10421`     | Port for internal API access.             |

### `miko` Configuration

| Parameter | Type   | Default                    | Description                  |
| --------- | ------ | -------------------------- | ---------------------------- |
| `address` | string | `"http://127.0.0.1:11417"` | Address of the Miko service. |

### `database` Configuration

| Parameter   | Type   | Default                    | Description                |
| ----------- | ------ | -------------------------- | -------------------------- |
| `file_path` | string | `"/etc/ainari/omamori_db"` | Path to the database file. |

### `simple_crypto` Configuration

| Parameter | Type   | Default                                          | Description                                                        |
| --------- | ------ | ------------------------------------------------ | ------------------------------------------------------------------ |
| `key_b64` | string | `"q9vN4CjOQm5wKzyzjZtS7t4oQp8oQK1JvU5xgq8vFzE="` | Base64 encoded encryption key for simple cryptographic operations. |

## Example

!!! info

    example config-file can be found in the repository under `example_configs/ainari/`

```toml
--8<-- "example_configs/ainari/omamori.toml"
```
