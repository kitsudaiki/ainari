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
# Creates a CA for a local setup, if it doesn't exist yet. The CA signs the certificates of all
# components of the setup and stays the same over all runs, so it only has to be added to the
# trust-store of the host once. Its name-constraints limit it to the names of the setup, so it
# can't be misused for any other host, even if its key leaks.
#
# Usage:
#   ./scripts/create_local_ca.sh <cert-file> <key-file> <common-name> <permitted-names>
#
#   <permitted-names> are the name-constraints, for example
#   "permitted;IP:127.0.0.1/255.255.255.255,permitted;DNS:localhost,permitted;DNS:cluster.local"

set -e

CA_CERT="$1"
CA_KEY="$2"
COMMON_NAME="$3"
PERMITTED="$4"

if [ -z "$PERMITTED" ]; then
    echo "Usage: $0 <cert-file> <key-file> <common-name> <permitted-names>"
    exit 1
fi

if [ -f "$CA_CERT" ] && [ -f "$CA_KEY" ]; then
    exit 0
fi

echo "Creating the CA $CA_CERT ..."
mkdir -p "$(dirname "$CA_CERT")" "$(dirname "$CA_KEY")"
(
    umask 077
    openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes \
        -keyout "$CA_KEY" -out "$CA_CERT" -days 3650 \
        -subj "/CN=$COMMON_NAME" \
        -addext "basicConstraints=critical,CA:TRUE,pathlen:0" \
        -addext "keyUsage=critical,keyCertSign,cRLSign" \
        -addext "nameConstraints=critical,$PERMITTED" \
        2> /dev/null
)
chmod 644 "$CA_CERT"
