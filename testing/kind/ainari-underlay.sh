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
# Underlay of the gateways in the kind-setup, so they exchange the traffic of the virtual machines
# over a network of their own and not over the pod-network of the cluster. It runs in the
# background within the kind-node, started by testing/kind/setup_kind_stack.sh.
#
# The underlay is the bridge br-underlay within the node. Every pod of a gateway gets a veth-pair
# towards it, whose end in the pod is named underlay0 (torii.underlay.iface in the
# Ainari-resource):
#
#   torii-public-*   172.31.0.10   the gateway at the edge (torii.underlay.edgeAddress)
#   sakura-N         172.31.0.2N   the gateway in front of the sakura-host N
#
# The veth-pair disappears together with the network-namespace of its pod, so a new pod, for
# example a restarted sakura-host, gets a new one. Its end in the pod gets a fixed mac-address,
# which is derived from its address, because the other gateways resolve the mac-address of a
# gateway only once, when they program their routes towards it. Hanami doesn't change these
# routes, when the address of a restarted host stays the same.

set -u

NAMESPACE="ainari"
BRIDGE="br-underlay"
NETWORK_PREFIX="172.31.0"
PREFIX_LENGTH=24
# the overlay adds a header to every packet, so the underlay needs a bigger mtu
MTU=1600
POD_IFACE="underlay0"
PID_FILE="/run/ainari-underlay.pid"

echo $$ > "$PID_FILE"

if ! ip link show "$BRIDGE" > /dev/null 2>&1; then
    ip link add "$BRIDGE" type bridge
fi
ip link set "$BRIDGE" mtu "$MTU" up

# The node loads br_netfilter for kubernetes, so the traffic between the ports of the bridge passes
# the FORWARD-chain of iptables as well.
iptables -C FORWARD -i "$BRIDGE" -o "$BRIDGE" -j ACCEPT 2> /dev/null \
    || iptables -I FORWARD -i "$BRIDGE" -o "$BRIDGE" -j ACCEPT

# Moves the end of a new veth-pair into the pod of a gateway.
#
# Arguments: name of the end in the node, pid of the pod, address of the gateway
connect_pod() {
    local host_iface="$1" pid="$2" address="$3"
    local peer="${host_iface}p"
    # locally administered unicast-address with the address of the gateway in the last byte
    local mac
    mac="$(printf '02:00:ac:1f:00:%02x' "$address")"

    echo "Connecting the pod with pid $pid as $NETWORK_PREFIX.$address over $host_iface ..."
    if ip link add "$host_iface" mtu "$MTU" type veth peer name "$peer" mtu "$MTU" address "$mac" \
        && ip link set "$host_iface" master "$BRIDGE" \
        && ip link set "$host_iface" up \
        && ip link set "$peer" netns "$pid" \
        && nsenter -t "$pid" -n ip link set "$peer" name "$POD_IFACE" \
        && nsenter -t "$pid" -n ip addr add "$NETWORK_PREFIX.$address/$PREFIX_LENGTH" dev "$POD_IFACE" \
        && nsenter -t "$pid" -n ip link set "$POD_IFACE" up; then
        echo "The pod has its underlay $NETWORK_PREFIX.$address."
    else
        # deleting one end removes the whole pair, also the one in the pod
        echo "Failed to connect the pod, retrying ..."
        ip link delete "$host_iface" > /dev/null 2>&1 || true
    fi
}

while true; do
    for sandbox in $(crictl pods --namespace "$NAMESPACE" --state ready -q 2> /dev/null); do
        read -r name pid < <(crictl inspectp -o go-template \
            --template '{{.status.metadata.name}} {{.info.pid}}' "$sandbox" 2> /dev/null)
        case "${name:-}" in
            torii-public-*)
                host_iface="ul-edge"
                address=10
                ;;
            sakura-[0-9])
                host_iface="ul-${name}"
                address="2${name#sakura-}"
                ;;
            *)
                continue
                ;;
        esac
        # the end in the node exists, as long as the pod has its underlay
        if [ -n "${pid:-}" ] && [ "$pid" != "0" ] && ! ip link show "$host_iface" > /dev/null 2>&1; then
            connect_pod "$host_iface" "$pid" "$address"
        fi
    done
    sleep 2
done
