# Onsen

!!! info

    The server is not the fastest, so it may take a few seconds, until the swagger-ui with the docu
    appears.

!!! info

    This documeation was generated from the source-code to provide a maximum of consistency.

## Onsen

All endpoints, which store and read the files, are internal and only called by ryokan and the
sakura-hosts with the internal api-key. Like for all other components, every request needs the
token of a user, which is validated by miko.

The volumes, which the sakura-hosts attach as disks, are addressed by their name within the
storage-backend (`backend_id`). Only ryokan manages them, the onsen never knows the UUID of a
volume in ryokan and never sees plain data, because the content is encrypted by the sakura-hosts.

[OpenAPI-spec of onsen](open_api_docu_onsen.json){ .swagger-ui-spec }
