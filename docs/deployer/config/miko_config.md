# Miko

## Options

### without group

#### `debug`

- *Type*: bool
- *Description*: Enable or disable the debug-logging

### `auth`

#### `token_key_path`

- *Description*: Already existing file-path on the server, where the file with the key is located,
    which should be used to sign and validate JWT-token
- *Type*: string

#### `token_expire_time`

- *Description*: Amount of time, which new created JWT-token should be valid, in seconds.
- *Type*: integer

### `api`

#### `ip`

- *Description*: IP of the intereface, where the server should listen on.
- *Type*: string

#### `port` = 11418

- *Description*: Port-number, where the server should listen on.
- *Type*: integer

### `database`

#### `file_path`

- *Description*: File-path, where the sqlite should store its data.
- *Type*: string

## Example

!!! info

    example config-file can be found in the repository under `example_configs/ainari/`

```toml
--8<-- "example_configs/ainari/miko.toml"
```
