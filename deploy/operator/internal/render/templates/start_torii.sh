#!/bin/bash
#
# Prepares a gateway in the pod and starts it afterwards with the start-script of the image.
set -e

export CONFIG_FILE=/tmp/torii.toml
cp "$CONFIG_TEMPLATE" "$CONFIG_FILE"

# The interface of an underlay, which is not the pod-network, is moved into the pod from the
# outside. Without it the eBPF-programs have nothing to attach to and the gateway has no address,
# which hanami could ask for, so it waits until the interface is there and carries its address.
if [ -n "$WAIT_FOR_UNDERLAY" ]; then
    echo "Waiting for the underlay $WAIT_FOR_UNDERLAY ..."
    while ! ip -4 addr show "$WAIT_FOR_UNDERLAY" 2>/dev/null | grep -q "inet "; do sleep 1; done
    ip link set "$WAIT_FOR_UNDERLAY" up
    echo "The underlay $WAIT_FOR_UNDERLAY is there."
fi

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
