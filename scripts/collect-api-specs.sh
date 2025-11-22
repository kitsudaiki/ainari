#!/bin/bash

curl 127.0.0.1:11417/openapi.json | jq > ./docs/frontend/open_api_docu_miko.json
curl 127.0.0.1:11416/openapi.json | jq > ./docs/frontend/open_api_docu_ryokan.json
curl 127.0.0.1:11418/openapi.json | jq > ./docs/frontend/open_api_docu_hanami.json
curl 127.0.0.1:11419/openapi.json | jq > ./docs/frontend/open_api_docu_torii.json
curl 127.0.0.1:11421/openapi.json | jq > ./docs/frontend/open_api_docu_omamori.json
curl 127.0.0.1:11420/openapi.json | jq > ./docs/frontend/open_api_docu_sakura.json