#!/bin/bash

# Copyright 2022 Tobias Anker
#
# Licensed under the Apache License, Version 2.0 (the "License")
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#    http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

curl 127.0.0.1:11417/openapi.json | jq > ./docs/user/rest_api/open_api_docu_miko.json
curl 127.0.0.1:11416/openapi.json | jq > ./docs/user/rest_api/open_api_docu_ryokan.json
curl 127.0.0.1:11418/openapi.json | jq > ./docs/user/rest_api/open_api_docu_hanami.json
curl 127.0.0.1:11419/openapi.json | jq > ./docs/user/rest_api/open_api_docu_torii.json
curl 127.0.0.1:11421/openapi.json | jq > ./docs/user/rest_api/open_api_docu_omamori.json
curl 127.0.0.1:11420/openapi.json | jq > ./docs/user/rest_api/open_api_docu_sakura.json