# Hanami-Testing

INI-style config for the python-SDK tests, which run against a local setup (see
[development](../../developer/development.md)). It is expected at
`/etc/ainari/hanami_testing.conf`.

## Options

### `connection` Section

| Parameter         | Type   | Default    | Description                                                        |
| ----------------- | ------ | ---------- | ------------------------------------------------------------------ |
| `miko_address`    | string | _required_ | Address of the [Miko](miko_config.md) service to log in.           |
| `test_user`       | string | _required_ | ID of the user used by the tests, which has to exist in Miko.      |
| `test_passphrase` | string | _required_ | Passphrase of the test-user.                                       |

The default user and passphrase match the admin-user, which is created at the initial start with
`AINARI_ADMIN_ID=asdf` and `AINARI_ADMIN_PASSPHRASE=asdfasdf`.

### `test_data` Section

Paths to the four files of the MNIST dataset.

| Parameter        | Type   | Default    | Description                            |
| ---------------- | ------ | ---------- | -------------------------------------- |
| `train_inputs`   | string | _required_ | Path to the training images.           |
| `train_labels`   | string | _required_ | Path to the training labels.           |
| `request_inputs` | string | _required_ | Path to the test images.               |
| `request_labels` | string | _required_ | Path to the test labels.               |

## Example

!!! info

    example config-file can be found in the repository under `example_configs/ainari/`

```ini
--8<-- "example_configs/ainari/hanami_testing.conf"
```
