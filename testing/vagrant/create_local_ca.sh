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
# Creates a CA for a local setup, if it doesn't exist yet or doesn't match the given parameters.
# The CA signs the certificates of all components of the setup and stays the same over all runs,
# so it only has to be added to the trust-store of the host once. Its name-constraints limit it to
# the names of the setup, so it can't be misused for any other host, even if its key leaks.
#
# Usage:
#   ./testing/vagrant/create_local_ca.sh <cert-file> <key-file> <common-name> <permitted-names>
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

# name-constraints in the form of the output of openssl, one per line, like "Permitted: DNS:localhost"
expected_constraints() {
    local entry
    local IFS=","
    for entry in $PERMITTED; do
        case "$entry" in
            permitted\;*) echo "Permitted: ${entry#permitted;}" ;;
            excluded\;*)  echo "Excluded: ${entry#excluded;}" ;;
        esac
    done | sort
}

actual_constraints() {
    local line
    local section=""
    openssl x509 -in "$CA_CERT" -noout -ext nameConstraints 2> /dev/null | while read -r line; do
        case "$line" in
            "X509v3 Name Constraints"*) ;;
            Permitted:|Excluded:) section="$line" ;;
            *) echo "$section $line" ;;
        esac
    done | sort
}

# An existing CA is kept, as long as it matches the parameters and its key, so it only has to be
# trusted once. Otherwise it is replaced, because the setup would not work with it.
if [ -f "$CA_CERT" ] && [ -f "$CA_KEY" ]; then
    REASON=""
    if [ "$(openssl x509 -in "$CA_CERT" -noout -subject -nameopt RFC2253 2> /dev/null)" != "subject=CN=$COMMON_NAME" ]; then
        REASON="its common-name is not '$COMMON_NAME'"
    elif [ "$(actual_constraints)" != "$(expected_constraints)" ]; then
        REASON="its name-constraints are not '$PERMITTED'"
    elif [ "$(openssl x509 -in "$CA_CERT" -noout -pubkey 2> /dev/null)" != \
           "$(openssl pkey -in "$CA_KEY" -pubout 2> /dev/null)" ]; then
        REASON="it doesn't belong to the key $CA_KEY"
    fi

    if [ -z "$REASON" ]; then
        exit 0
    fi
    echo "Replacing the CA $CA_CERT, because $REASON."
    echo "WARNING: the new CA has to be added to the trust-store of the host again."
    rm -f "$CA_CERT" "$CA_KEY"
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
