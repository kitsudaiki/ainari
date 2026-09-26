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
# Connects the virtual machine ainari-torii to the gateway at the edge of the network, which runs
# in the pod of torii-public. It is the same, which scripts/setup_local_stack.sh and
# scripts/setup_kind_stack.sh do on the host, but runs as the service ainari-uplink within the
# virtual machine: a new pod of torii-public has a new network-namespace, so the service moves a
# new veth-pair into it, whenever the one of the previous pod is gone.
#
# The virtual machine is the next hop of the gateway and routes the floating ip-addresses between
# the gateway and the host, which reaches them over the private network of vagrant. The virtual
# machines of ainari reach the internet over the NAT of this virtual machine.

set -u

# the host-side of the veth-pair is the next hop of the gateway (torii.public.network in the values)
HOST_IFACE="veth-uplink"
HOST_ADDRESS="10.0.0.1/24"
GATEWAY_IFACE="veth-gw"
GATEWAY_ADDRESS="10.0.0.254/24"
FLOATING_IP_CIDR="10.0.0.0/24"
NAMESPACE="ainari"

CRICTL=(k3s crictl)

sysctl -q -w net.ipv4.ip_forward=1

# the virtual machines of ainari reach the internet over the default route of this virtual machine
DEFAULT_IF=$(ip route show default | awk '/default/ {print $5}' | head -n 1)
iptables -t nat -C POSTROUTING -s "$FLOATING_IP_CIDR" -o "$DEFAULT_IF" -j MASQUERADE 2> /dev/null \
    || iptables -t nat -A POSTROUTING -s "$FLOATING_IP_CIDR" -o "$DEFAULT_IF" -j MASQUERADE
iptables -C FORWARD -s "$FLOATING_IP_CIDR" -j ACCEPT 2> /dev/null \
    || iptables -I FORWARD -s "$FLOATING_IP_CIDR" -j ACCEPT
iptables -C FORWARD -d "$FLOATING_IP_CIDR" -j ACCEPT 2> /dev/null \
    || iptables -I FORWARD -d "$FLOATING_IP_CIDR" -j ACCEPT

while true; do
    # The veth-pair disappears together with the network-namespace of the pod, so a missing
    # host-side means, that the gateway of the current pod has no uplink yet.
    if ! ip link show "$HOST_IFACE" > /dev/null 2>&1; then
        SANDBOX_ID="$("${CRICTL[@]}" pods --namespace "$NAMESPACE" --label app=torii-public \
            --state ready -q 2> /dev/null | head -n 1)"
        if [ -n "$SANDBOX_ID" ]; then
            SANDBOX_PID="$("${CRICTL[@]}" inspectp -o go-template --template '{{.info.pid}}' \
                "$SANDBOX_ID" 2> /dev/null)"
            if [ -n "$SANDBOX_PID" ] && [ "$SANDBOX_PID" != "0" ]; then
                echo "Injecting $GATEWAY_IFACE into the pod of torii-public (pid $SANDBOX_PID) ..."
                ip link add "$HOST_IFACE" type veth peer name "$GATEWAY_IFACE"
                ip addr add "$HOST_ADDRESS" dev "$HOST_IFACE"
                # No checksum offloading: the datapath of the gateway rewrites addresses with
                # incremental checksum updates, which are only correct on complete checksums.
                ethtool -K "$HOST_IFACE" tx off rx off > /dev/null 2>&1 || true
                ip link set "$HOST_IFACE" up
                if ip link set "$GATEWAY_IFACE" netns "$SANDBOX_PID" \
                    && nsenter -t "$SANDBOX_PID" -n ip addr add "$GATEWAY_ADDRESS" dev "$GATEWAY_IFACE" \
                    && nsenter -t "$SANDBOX_PID" -n ip link set "$GATEWAY_IFACE" up; then
                    echo "The gateway has its uplink."
                else
                    echo "Failed to inject the uplink, retrying ..."
                    ip link delete "$HOST_IFACE" > /dev/null 2>&1 || true
                fi
            fi
        fi
    fi
    sleep 2
done
