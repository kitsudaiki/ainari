# Token-Key

The token-key is a plain file, whose whole content is used by [Miko](miko_config.md) as secret key
to sign and verify the tokens of the users. Its path is configured with `auth.token_key_path` in
the config of Miko, by default `/etc/ainari/token_key`. If the file can not be read, Miko exits.

All tokens become invalid, when the content of the file is changed.

!!! warning

    The key of the example is public and only meant for local testing. Always use a long random
    key for a deployment, for example created with

    ```bash
    openssl rand -base64 64 | tr -d '\n' > /etc/ainari/token_key
    ```

## Example

!!! info

    example file can be found in the repository under `example_configs/ainari/`

```text
--8<-- "example_configs/ainari/token_key"
```
