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

# Creates the uplink of the torii single node setup for local development
# (see example_configs/ainari/torii_single_node.toml).
#
# The "outside" is a network namespace connected to the host by a veth pair:
#
#   [ netns torii-outside ]  outside0 10.0.0.1/24   (uplink_next_hop)
#              |
#   [ host, torii ]          uplink0  10.0.0.254/24 (uplink_iface)
#
# The floating IPs (10.0.0.2 - 10.0.0.253) are reachable from inside the
# namespace only, for example:
#
#   sudo ip netns exec torii-outside ssh ubuntu@10.0.0.2
#
# On a real host no such namespace is needed: set uplink_iface to the physical
# NIC and uplink_next_hop to the router behind it instead.
#
#   sudo ./scripts/setup_single_node_uplink.sh        create (replaces an existing one)
#   sudo ./scripts/setup_single_node_uplink.sh down   remove it again

set -e

NETNS="torii-outside"
UPLINK="uplink0"
OUTSIDE="outside0"
UPLINK_CIDR="10.0.0.254/24"
OUTSIDE_CIDR="10.0.0.1/24"

if [ "$EUID" -ne 0 ]; then
    echo "Please run as root"
    exit 1
fi

# deleting one end of the veth pair removes the other one as well
ip link delete "$UPLINK" > /dev/null 2>&1 || true
ip netns delete "$NETNS" > /dev/null 2>&1 || true

if [ "$1" == "down" ]; then
    echo "Single node uplink removed"
    exit 0
fi

ip netns add "$NETNS"
ip link add "$UPLINK" type veth peer name "$OUTSIDE" netns "$NETNS"

ip addr add "$UPLINK_CIDR" dev "$UPLINK"
ip link set "$UPLINK" up

ip -n "$NETNS" addr add "$OUTSIDE_CIDR" dev "$OUTSIDE"
ip -n "$NETNS" link set "$OUTSIDE" up
ip -n "$NETNS" link set lo up

# The gateway rewrites addresses with incremental checksum updates, which are
# only correct on complete checksums, so offloading is switched off on both ends.
ethtool -K "$UPLINK" tx off rx off > /dev/null
ip netns exec "$NETNS" ethtool -K "$OUTSIDE" tx off rx off > /dev/null

echo "Single node uplink ready: $UPLINK ($UPLINK_CIDR) <-> $OUTSIDE ($OUTSIDE_CIDR in netns $NETNS)"
