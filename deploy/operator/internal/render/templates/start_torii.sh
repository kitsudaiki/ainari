#!/bin/bash
#
# Prepares a gateway in the pod and starts it afterwards with the start-script of the image.
set -e

export CONFIG_FILE=/tmp/torii.toml
cp "$CONFIG_TEMPLATE" "$CONFIG_FILE"

# A gateway in front of a sakura-host sends everything, which is not for one of its virtual
# machines, to the gateway at the edge of the network.
if [ -n "$DEFAULT_GATEWAY_HOST" ]; then
    echo "Resolving the gateway at the edge $DEFAULT_GATEWAY_HOST ..."
    GATEWAY_IP=""
    while [ -z "$GATEWAY_IP" ]; do
        GATEWAY_IP="$(getent ahostsv4 "$DEFAULT_GATEWAY_HOST" | awk 'NR == 1 {print $1}')"
        [ -n "$GATEWAY_IP" ] || sleep 1
    done
    echo "The gateway at the edge has the address $GATEWAY_IP"
    sed -i "s/@DEFAULT_GATEWAY_IP@/$GATEWAY_IP/g" "$CONFIG_FILE"
fi

exec /app/start.sh
