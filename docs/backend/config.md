# Configs of Ainari

!!! info

    example config-file can be found in the repository under `example_configs/ainari/`

## Bento

### without group

#### `debug`

- *Description*: Enable or disable the debug-logging
- *Type*: bool

### `storage`

#### `dataset_location`

- *Description*: Location on the server, where uploaded datasets should be stored.
- *Type*: string

#### `checkpoint_location`

- *Description*: Location on the server, where checkpoint-files should be stored.
- *Type*: string

#### `tempfile_location`

- *Description*: Location on the server, where temporary-files should be stored, for example while uploading a new dataset.
- *Type*: string

### `api`

#### `ip` 

- *Description*: IP of the intereface, where the server should listen on.
- *Type*: string

#### `port`

- *Description*: Port-number, where the server should listen on.
- *Type*: integer
- *Default*: 11416

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

- *Description*: In case of a self-signed certificate this has to be set to true, to ignore failing checks of the tls-certificate.
- *Type*: bool
- *Default*: false



## Sakura

### without group

#### `debug`

- *Description*: Enable or disable the debug-logging
- *Type*: bool

### `processing`

#### `max_number_of_threads`

- *Description*: Maximum number of threads to use as worker-threads. If 0, then all available threads are used.
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

- *Description*: In case of a self-signed certificate this has to be set to true, to ignore failing checks of the tls-certificate.
- *Type*: bool
- *Default*: false


### `bento`

#### `address` 

- *Description*: Address of the server
- *Type*: string

#### `port`

- *Description*: Port-number where the server is listening.
- *Type*: integer
- *Default*: 11416

#### `insecure`

- *Description*: In case of a self-signed certificate this has to be set to true, to ignore failing checks of the tls-certificate.
- *Type*: bool
- *Default*: false



## Miko

### without group

#### `debug`

- *Type*: bool
- *Description*: Enable or disable the debug-logging

### `auth`

#### `token_key_path`

- *Description*: Already existing file-path on the server, where the file with the key is located, which should be used to sign and validate JWT-token
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
