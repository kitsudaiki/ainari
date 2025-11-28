# Onsen

## Options

### Root Configuration

| Parameter               | Type    | Default                    | Description                                                                                        |
| ----------------------- | ------- | -------------------------- | -------------------------------------------------------------------------------------------------- |
| `debug`                 | boolean | `true`                     | Enables debug mode for detailed logging and troubleshooting.                                       |
| `log_path`              | string  | `"/var/log"`               | Path to the directory where log files will be stored.                                              |
| `skip_tls_verification` | boolean | `true`                     | Set true to skip validation of https-connections, for example in case of self-singed certificates. |
| `address`               | string  | `"http://127.0.0.1:50051"` | Address of the onsen-host itself, where it can be reached from the ryokan and sakura.              |

### `api` Configuration

| Parameter          | Type    | Default     | Description                               |
| ------------------ | ------- | ----------- | ----------------------------------------- |
| `public_ip`        | string  | `"0.0.0.0"` | IP address for public API access.         |
| `public_port`      | integer | `11422`     | Port for public API access.               |
| `internal_ip`      | string  | `"0.0.0.0"` | IP address for internal API access.       |
| `internal_port`    | integer | `10422`     | Port for internal API access.             |
| `internal_api_key` | string  | `"asdf"`    | API key required for internal API access. |

### `miko` Configuration

| Parameter | Type   | Default                    | Description                  |
| --------- | ------ | -------------------------- | ---------------------------- |
| `address` | string | `"http://127.0.0.1:11417"` | Address of the Miko service. |

### `storage` Configuration

| Parameter  | Type   | Default        | Description                           |
| ---------- | ------ | -------------- | ------------------------------------- |
| `location` | string | `"/tmp/onsen"` | Directory where files will be stored. |

### `ryokan` Configuration

| Parameter         | Type   | Default                   | Description                          |
| ----------------- | ------ | ------------------------- | ------------------------------------ |
| `registation_key` | string | `"test-registration-key"` | Registration key for Ryokan service. |

## Example

!!! info

    example config-file can be found in the repository under `example_configs/ainari/`

```toml
--8<-- "example_configs/ainari/onsen.toml"
```
