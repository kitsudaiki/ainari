# Izakaya

The config is read from `/etc/ainari/izakaya.toml`. The path can be overwritten with the
environment-variable `CONFIG_FILE`.

## Options

### Root Configuration

| Parameter               | Type    | Default    | Description                                                                                        |
| ----------------------- | ------- | ---------- | -------------------------------------------------------------------------------------------------- |
| `debug`                 | boolean | _required_ | Enables debug mode for detailed logging and troubleshooting.                                       |
| `log_type` | string | `"stdout"` | Target of the logs: `"stdout"` or `"log_file"`. |
| `log_path` | string | `"/var/log/"` | Directory of the log-file `<service>.log`, if `log_type` is `"log_file"`. |
| `database_type` | string | _required_ | Type of the database, `"sqlite"` or `"mysql"`. It selects the group `sqlite` or `mysql`, which is used to connect to the database. |
| `skip_tls_verification` | boolean | `false`    | Set true to skip validation of https-connections, for example in case of self-singed certificates. |

### `api` Configuration

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

### `sqlite` Configuration

Required, if `database_type` is `"sqlite"`.

| Parameter   | Type   | Default    | Description                                                        |
| ----------- | ------ | ---------- | ------------------------------------------------------------------ |
| `file_path` | string | _required_ | Path to the database file, which is created, if it doesn't exist. |

### `mysql` Configuration

Required, if `database_type` is `"mysql"`. The password of the user is not part of the config, but
read from the env-variable `AINARI_MYSQL_PASSWORD`.

| Parameter  | Type    | Default    | Description                                                                          |
| ---------- | ------- | ---------- | ------------------------------------------------------------------------------------ |
| `host`     | string  | _required_ | Hostname or IP-address of the mysql-server.                                          |
| `port`     | integer | `3306`     | Port of the mysql-server.                                                            |
| `user`     | string  | _required_ | User to log into the mysql-server.                                                   |
| `database` | string  | _required_ | Name of the database. It has to exist already, but the tables are created by the service. |

### `mls` Configuration

| Parameter               | Type    | Default    | Description                                                                                                  |
| ----------------------- | ------- | ---------- | ------------------------------------------------------------------------------------------------------------ |
| `grant_public_key`      | string  | _required_ | Base64 encoded public key of [Hanami](hanami_config.md), which signs the membership-grants of the gateways. It belongs to the `MLS_GRANT_SIGNING_KEY` of Hanami, which logs this public key at its start. |
| `key_rotation_interval` | integer | `3600`     | Seconds between two key-rotations of a group, whose membership didn't change.                                |
| `member_timeout`        | integer | `120`      | Seconds without contact, after which a gateway is removed from its groups.                                   |

!!! warning

    The key of the example config belongs to the signing-key of the development-setup. Always use
    the public key of the own `MLS_GRANT_SIGNING_KEY` of Hanami for a deployment.

## Example

!!! info

    example config-file can be found in the repository under `example_configs/ainari/`

```toml
--8<-- "example_configs/ainari/izakaya.toml"
```
