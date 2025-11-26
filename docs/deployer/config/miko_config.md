# Miko

## Options

### Root Configuration

| Parameter  | Type    | Default      | Description                                                  |
| ---------- | ------- | ------------ | ------------------------------------------------------------ |
| `debug`    | boolean | `true`       | Enables debug mode for detailed logging and troubleshooting. |
| `log_path` | string  | `"/var/log"` | Path to the directory where log files will be stored.        |

### `api` Configuration

| Parameter          | Type    | Default     | Description                               |
| ------------------ | ------- | ----------- | ----------------------------------------- |
| `public_ip`        | string  | `"0.0.0.0"` | IP address for public API access.         |
| `public_port`      | integer | `11417`     | Port for public API access.               |
| `internal_ip`      | string  | `"0.0.0.0"` | IP address for internal API access.       |
| `internal_port`    | integer | `10417`     | Port for internal API access.             |
| `internal_api_key` | string  | `"asdf"`    | API key required for internal API access. |

### `database` Configuration

| Parameter   | Type   | Default                 | Description                |
| ----------- | ------ | ----------------------- | -------------------------- |
| `file_path` | string | `"/etc/ainari/miko_db"` | Path to the database file. |

### `authentication` Configuration

| Parameter           | Type    | Default                   | Description                                                   |
| ------------------- | ------- | ------------------------- | ------------------------------------------------------------- |
| `token_key_path`    | string  | `"/etc/ainari/token_key"` | Path to the file containing the token key for authentication. |
| `token_expire_time` | integer | `3600`                    | Token expiration time in seconds (1 hour by default).         |

### `endpoints` Configuration

#### `hanami` Endpoints

| Parameter          | Type   | Default                    | Description                          |
| ------------------ | ------ | -------------------------- | ------------------------------------ |
| `public_address`   | string | `"http://127.0.0.1:11418"` | Public address for Hanami service.   |
| `internal_address` | string | `"http://127.0.0.1:10418"` | Internal address for Hanami service. |

#### `ryokan` Endpoints

| Parameter          | Type   | Default                    | Description                          |
| ------------------ | ------ | -------------------------- | ------------------------------------ |
| `public_address`   | string | `"http://127.0.0.1:11416"` | Public address for Ryokan service.   |
| `internal_address` | string | `"http://127.0.0.1:10416"` | Internal address for Ryokan service. |

#### `torii` Endpoints

| Parameter          | Type   | Default                    | Description                         |
| ------------------ | ------ | -------------------------- | ----------------------------------- |
| `public_address`   | string | `"http://127.0.0.1:11419"` | Public address for Torii service.   |
| `internal_address` | string | `"http://127.0.0.1:10419"` | Internal address for Torii service. |

#### `omamori` Endpoints

| Parameter          | Type   | Default                    | Description                           |
| ------------------ | ------ | -------------------------- | ------------------------------------- |
| `public_address`   | string | `"http://127.0.0.1:11421"` | Public address for Omamori service.   |
| `internal_address` | string | `"http://127.0.0.1:10421"` | Internal address for Omamori service. |

## Example

!!! info

    example config-file can be found in the repository under `example_configs/ainari/`

```toml
--8<-- "example_configs/ainari/miko.toml"
```
