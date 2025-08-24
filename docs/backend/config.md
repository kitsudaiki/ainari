# Configs of Ainari

!!! info

    example config-file can be found in the repository under `example_configs/ainari/hanami.toml`

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

### `max_number_of_threads`

- *Description*: Maximum number of threads to use as worker-threads. If 0, then all available threads are used.
- *Type*: int
- *Default*: 0
- *Restriction*: Positive value

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
