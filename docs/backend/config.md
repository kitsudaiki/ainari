# Configs of OpenHanami

!!! info

    example config-file can be found in the repository under `example_configs/openhanami/hanami.toml`

## without group

### `debug`

- *Type*: bool
- *Description*: Enable or disable the debug-logging

## `storage`-group

### `dataset_location`

- *Description*: Location on the server, where uploaded datasets should be stored.
- *Type*: string

### `checkpoint_location`

- *Description*: Location on the server, where checkpoint-files should be stored.
- *Type*: string

### `tempfile_location`

- *Description*: Location on the server, where temporary-files should be stored, for example while uploading a new dataset.
- *Type*: string

## `processing`-group

### `use_of_free_memory`

- *Description*: Amount of memory to pre-allocate for the backend on the server. It means the percentage of the momory, so 1.0 means 100% of available memory.
- *Type*: float
- *Restriction*: Value must be between `0.01` and `0.9`

## `auth`-group

### `token_key_path`

- *Description*: Already existing file-path on the server, where the file with the key is located, which should be used to sign and validate JWT-token
- *Type*: string

### `token_expire_time`

- *Description*: Amount of time, which new created JWT-token should be valid, in seconds.
- *Type*: integer

## `api`-group

### `ip` 

- *Description*: IP of the intereface, where the server should listen on.
- *Type*: string

### `port` = 11418

- *Description*: Port-number, where the server should listen on.
- *Type*: integer

## `database`-group

### `file_path`

- *Description*: File-path, where the sqlite should store its data.
- *Type*: string
