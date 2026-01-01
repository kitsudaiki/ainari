# Ryokan

## Options

### Root Configuration

| Parameter               | Type    | Default      | Description                                                                                        |
| ----------------------- | ------- | ------------ | -------------------------------------------------------------------------------------------------- |
| `debug`                 | boolean | `true`       | Enables debug mode for detailed logging and troubleshooting.                                       |
| `log_path`              | string  | `"/var/log"` | Path to the directory where log files will be stored.                                              |
| `skip_tls_verification` | boolean | `true`       | Set true to skip validation of https-connections, for example in case of self-singed certificates. |

### `api` Configuration

| Parameter       | Type    | Default     | Description                         |
| --------------- | ------- | ----------- | ----------------------------------- |
| `public_ip`     | string  | `"0.0.0.0"` | IP address for public API access.   |
| `public_port`   | integer | `11416`     | Port for public API access.         |
| `internal_ip`   | string  | `"0.0.0.0"` | IP address for internal API access. |
| `internal_port` | integer | `10416`     | Port for internal API access.       |

### `database` Configuration

| Parameter   | Type   | Default                   | Description                |
| ----------- | ------ | ------------------------- | -------------------------- |
| `file_path` | string | `"/etc/ainari/ryokan_db"` | Path to the database file. |

### `miko` Configuration

| Parameter | Type   | Default                    | Description                  |
| --------- | ------ | -------------------------- | ---------------------------- |
| `address` | string | `"http://127.0.0.1:11417"` | Address of the Miko service. |

### `storage` Configuration

| Parameter           | Type   | Default         | Description                                     |
| ------------------- | ------ | --------------- | ----------------------------------------------- |
| `tempfile_location` | string | `"/tmp/ryokan"` | Directory where temporary files will be stored. |

## Example

!!! info

    example config-file can be found in the repository under `example_configs/ainari/`

```toml
--8<-- "example_configs/ainari/hanami.toml"
```
