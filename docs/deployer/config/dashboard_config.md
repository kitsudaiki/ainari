# Dashboard

The dashboard loads its config at startup from `/config.json` of its own web-server. In the
docker-compose setup of `src/dashboard/` the file `/etc/ainari/dashboard_config.json` is mounted
there, in the kubernetes deployment it is created by the operator.

## Options

| Parameter | Type   | Default    | Description                                                                                             |
| --------- | ------ | ---------- | ------------------------------------------------------------------------------------------------------- |
| `apiUrl`  | string | _required_ | Public address of the [Miko](miko_config.md) service. The browser talks to it directly, so it has to be reachable from the machine of the user. |

## Example

!!! info

    example config-file can be found in the repository under `example_configs/ainari/`

```json
--8<-- "example_configs/ainari/dashboard_config.json"
```
