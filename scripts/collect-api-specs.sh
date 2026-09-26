#!/bin/bash

# Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>
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

set -euo pipefail

OUTPUT_DIR="./docs/user/rest_api"

# Sakura shares the network-namespace of the torii in front of it and has no published port in
# the local docker-compose setup, so it is only reachable at the address of its gateway
# 'torii-vmm' on the docker-network (see docker-compose.yml)
SAKURA_ADDRESS="${SAKURA_ADDRESS:-172.30.0.20:11420}"

# downloads the openapi-spec of a component. The target-file is only overwritten, when the
# download was successful, so an unreachable component doesn't leave an empty spec behind.
# All 'operationId'-fields are removed, because the secret-scanner of github reports them as
# false-positives. They are removed with jq instead of deleting their lines, so the json stays
# valid, even when an 'operationId' is the last field of its object.
collect_spec() {
    local name="$1"
    local address="$2"
    local spec

    if ! spec=$(curl -sf -m 10 "http://${address}/openapi.json" \
                | jq 'walk(if type == "object" then del(.operationId) else . end)'); then
        echo "failed to get the openapi-spec of ${name} from ${address}" >&2
        return 1
    fi
    echo "${spec}" > "${OUTPUT_DIR}/open_api_docu_${name}.json"
    echo "collected openapi-spec of ${name}"
}

collect_spec miko 127.0.0.1:11417
collect_spec ryokan 127.0.0.1:11416
collect_spec hanami 127.0.0.1:11418
collect_spec torii 127.0.0.1:11419
collect_spec omamori 127.0.0.1:11421
collect_spec sakura "${SAKURA_ADDRESS}"
