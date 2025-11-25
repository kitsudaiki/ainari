# Sakura

## Options

### without group

#### `debug`

- *Description*: Enable or disable the debug-logging
- *Type*: bool

### `processing`

#### `max_number_of_threads`

- *Description*: Maximum number of threads to use as worker-threads. If 0, then all available
    threads are used.
- *Type*: int
- *Default*: 0
- *Restriction*: Positive value

### `api`

#### `ip`

- *Description*: IP of the intereface, where the server should listen on.
- *Type*: string

#### `port`

- *Description*: Port-number, where the server should listen on.
- *Type*: integer
- *Default*: 11418

### `database`

#### `file_path`

- *Description*: File-path, where the sqlite should store its data.
- *Type*: string

### `miko`

#### `address`

- *Description*: Address of the server
- *Type*: string

#### `port`

- *Description*: Port-number where the server is listening.
- *Type*: integer
- *Default*: 11417

#### `insecure`

- *Description*: In case of a self-signed certificate this has to be set to true, to ignore failing
    checks of the tls-certificate.
- *Type*: bool
- *Default*: false

### `ryokan`

#### `address`

- *Description*: Address of the server
- *Type*: string

#### `port`

- *Description*: Port-number where the server is listening.
- *Type*: integer
- *Default*: 11416

#### `insecure`

- *Description*: In case of a self-signed certificate this has to be set to true, to ignore failing
    checks of the tls-certificate.
- *Type*: bool
- *Default*: false

## Example

!!! info

    example config-file can be found in the repository under `example_configs/ainari/`

```toml
--8<-- "example_configs/ainari/hanami.toml"
```
