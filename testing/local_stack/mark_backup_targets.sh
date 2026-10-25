#!/bin/bash
#
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
#
# Marks the onsen-hosts of the backups as backup-targets in ryokan, so isekai stores its backups on
# them and ryokan never puts normal images or snapshots there. The onsen-hosts register themselves
# shortly after their start, so they are awaited. They are found by their names, which are the
# names of their pods in kubernetes (onsen-backup-0, ...), or by their addresses, which contain the
# name of their container or service (http://onsen-backup:50051, ...). A host, which is already
# marked, stays marked.
#
# Usage:
#   ./testing/local_stack/mark_backup_targets.sh <miko-address> <ryokan-address> [<ca-certificate>]
#
# The admin-user is read from AINARI_USER and AINARI_PASSPHRASE (default asdf/asdfasdf). Without a
# CA-certificate, the addresses are expected to be plain http.

set -e

MIKO_ADDRESS="$1"
RYOKAN_ADDRESS="$2"
CA_CERT="$3"
if [ -z "$MIKO_ADDRESS" ] || [ -z "$RYOKAN_ADDRESS" ]; then
    echo "Usage: $0 <miko-address> <ryokan-address> [<ca-certificate>]"
    exit 2
fi

CURL=(curl -sf)
if [ -n "$CA_CERT" ]; then
    CURL+=(--cacert "$CA_CERT")
fi

mark_backup_targets() {
    local token host_uuids host_uuid
    token="$("${CURL[@]}" -X POST "$MIKO_ADDRESS/v1alpha/token" \
        -d "token_format=jwt" -d "grant_type=client_credentials" \
        -d "client_id=${AINARI_USER:-asdf}" -d "client_secret=${AINARI_PASSPHRASE:-asdfasdf}" \
        | python3 -c 'import json, sys; print(json.load(sys.stdin)["access_token"])')" || return 1
    host_uuids="$("${CURL[@]}" -H "Authorization: Bearer $token" "$RYOKAN_ADDRESS/v1alpha/host/admin" \
        | python3 -c 'import json, sys
hosts = [h for h in json.load(sys.stdin)["hosts"]
         if h["name"].startswith("onsen-backup-") or "//onsen-backup" in h["host_address"]]
if not hosts:
    sys.exit(1)
print("\n".join(h["uuid"] for h in hosts))')" || return 1
    for host_uuid in $host_uuids; do
        "${CURL[@]}" -X PUT -H "Authorization: Bearer $token" -H "Content-Type: application/json" \
            -d '{"is_backup_target": true}' \
            "$RYOKAN_ADDRESS/v1alpha/host/$host_uuid/backup_target/admin" > /dev/null || return 1
        echo "Marked the onsen-host $host_uuid as backup-target"
    done
}

for _ in $(seq 1 60); do
    if mark_backup_targets 2> /dev/null; then
        exit 0
    fi
    sleep 2
done
echo "WARNING: the onsen-hosts of the backups could not be marked as backup-targets, so isekai"
echo "         can't create backups."
exit 1
