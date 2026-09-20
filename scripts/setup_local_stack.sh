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
# Starts the local docker-compose setup and connects the host to it. Everything in here needs
# root: the veth-pair towards the gateway is moved into the network-namespace of a container and
# the host has to forward and masquerade the traffic of the virtual machines.
#
# Usage:
#   sudo ./scripts/setup_local_stack.sh          start the stack and connect the host to it
#   sudo ./scripts/setup_local_stack.sh --down   stop the stack and remove the veth-pair again

set -e

# range of the floating ip-addresses, which is configured in the config of hanami. The host owns
# the first address of that range and is the next hop of the gateway at the edge.
FLOATING_IP_CIDR="10.0.0.0/24"
HOST_ADDRESS="10.0.0.1/24"
# address of the gateway on the other end of the veth-pair. The floating ip-addresses are served
# on that interface, so it sits in the same subnet as the host.
GATEWAY_ADDRESS="10.0.0.254/24"
# the gateway at the edge of the network, which the veth-pair is injected into
GATEWAY_CONTAINER="torii-public"

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# /dev/kvm belongs to a group, whose id differs between hosts. Sakura runs as a normal user, so
# it needs that group to open the virtual machines.
KVM_GID="$(stat -c '%g' /dev/kvm 2>/dev/null || echo "")"
if [ -z "$KVM_GID" ]; then
    echo "/dev/kvm not found. The host needs kvm to run the virtual machines."
    exit 1
fi
export KVM_GID

if [ "$EUID" -ne 0 ]; then
    echo "Please run as root (e.g. sudo ./scripts/setup_local_stack.sh)"
    exit 1
fi

cd "$PROJECT_DIR"

# ---------------------------------------------------------------------------------------------
# cleanup of a previous run
# ---------------------------------------------------------------------------------------------
echo "Cleaning up a previous setup ..."
ip link delete veth-host > /dev/null 2>&1 || true

if [ "$1" == "--down" ]; then
    docker compose down --remove-orphans
    echo "Stack stopped and the veth-pair removed."
    exit 0
fi

docker compose down --remove-orphans > /dev/null 2>&1 || true

# ---------------------------------------------------------------------------------------------
# check that nothing else on the host owns the floating ip-addresses
# ---------------------------------------------------------------------------------------------
# Another interface in the same subnet steals the traffic of the floating ip-addresses and, if it
# carries the address of the gateway, the host even ignores the arp-requests of the gateway.
# 'uplink0' of scripts/setup_single_node_uplink.sh is such an interface.
FIP_PREFIX="$(echo "$FLOATING_IP_CIDR" | cut -d. -f1-3)."
CONFLICTS="$(ip -o -4 addr show | awk '{print $2, $4}' | grep " ${FIP_PREFIX}" | awk '{print $1}' | sort -u)"

if [ -n "$CONFLICTS" ]; then
    echo "The following interfaces of the host already have an address in $FLOATING_IP_CIDR:"
    for iface in $CONFLICTS; do
        echo "    $iface"
    done
    echo "They collide with the floating ip-addresses of the setup. Remove them, for example with"
    echo "    sudo ip link delete <interface>"
    echo "or set another range in FLOATING_IP_CIDR of this script and in the config of hanami."
    exit 1
fi

# ---------------------------------------------------------------------------------------------
# start the stack
# ---------------------------------------------------------------------------------------------
# Always rebuild first: starting with stale images silently runs a different version than the one
# in this working tree.
echo "Building the images ..."
docker compose build

echo "Starting the containers ..."
docker compose up -d

echo "Waiting for $GATEWAY_CONTAINER to come up ..."
while ! docker inspect -f '{{.State.Pid}}' "$GATEWAY_CONTAINER" > /dev/null 2>&1 || \
      [ "$(docker inspect -f '{{.State.Pid}}' "$GATEWAY_CONTAINER")" == "0" ]; do
    sleep 1
done

PID=$(docker inspect -f '{{.State.Pid}}' "$GATEWAY_CONTAINER")

# ---------------------------------------------------------------------------------------------
# connect the host to the gateway at the edge of the network
# ---------------------------------------------------------------------------------------------
echo "Injecting veth-gw into $GATEWAY_CONTAINER (pid $PID) ..."
ip link add veth-host type veth peer name veth-gw

# The host only reaches the floating ip-addresses. The internal addresses of the virtual machines
# stay behind the gateway, which translates them.
ip addr add "$HOST_ADDRESS" dev veth-host
ip link set veth-host up

# No checksum offloading: the datapath of the gateway rewrites addresses with incremental
# checksum updates, which are only correct on complete checksums.
ethtool -K veth-host tx off rx off > /dev/null 2>&1 || true

ip link set veth-gw netns "$PID"
nsenter -t "$PID" -n ip addr add "$GATEWAY_ADDRESS" dev veth-gw
nsenter -t "$PID" -n ip link set veth-gw up
nsenter -t "$PID" -n ethtool -K veth-gw tx off rx off > /dev/null 2>&1 || true

# ---------------------------------------------------------------------------------------------
# let the virtual machines reach the internet through the host
# ---------------------------------------------------------------------------------------------
echo "Enabling forwarding and NAT for $FLOATING_IP_CIDR ..."
sysctl -w net.ipv4.ip_forward=1 > /dev/null

DEFAULT_IF=$(ip route show default | awk '/default/ {print $5}' | head -n 1)

if [ -n "$DEFAULT_IF" ]; then
    # remove the rules of a previous run first, so they are not added twice
    iptables -t nat -D POSTROUTING -s "$FLOATING_IP_CIDR" -o "$DEFAULT_IF" -j MASQUERADE 2>/dev/null || true
    iptables -D FORWARD -s "$FLOATING_IP_CIDR" -j ACCEPT 2>/dev/null || true
    iptables -D FORWARD -d "$FLOATING_IP_CIDR" -j ACCEPT 2>/dev/null || true

    iptables -t nat -A POSTROUTING -s "$FLOATING_IP_CIDR" -o "$DEFAULT_IF" -j MASQUERADE
    iptables -A FORWARD -s "$FLOATING_IP_CIDR" -j ACCEPT
    iptables -A FORWARD -d "$FLOATING_IP_CIDR" -j ACCEPT
else
    echo "WARNING: no default route found on the host, so the virtual machines have no internet."
fi

echo ""
echo "The stack is up. The api is reachable at:"
echo "    miko     http://127.0.0.1:11417"
echo "    hanami   http://127.0.0.1:11418"
echo "    ryokan   http://127.0.0.1:11416"
echo "    omamori  http://127.0.0.1:11421"
echo "    torii    http://127.0.0.1:11419"
echo ""
echo "Now the test can be started as a normal user:"
echo "    python3 testing/local_stack/vm_lifecycle_test.py"
