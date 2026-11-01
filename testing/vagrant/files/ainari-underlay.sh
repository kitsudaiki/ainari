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
# Moves the interface of the underlay-network of vagrant into the pod of the gateway on this
# virtual machine, so the gateways exchange the traffic of the virtual machines of ainari over
# their own network and not over the pod-network of flannel. It runs as the service
# ainari-underlay on the virtual machine ainari-torii (pod of torii-public) and on the
# sakura-hosts (pod of sakura, whose torii owns the TAP-devices).
#
# The whole interface is moved into the network-namespace of the pod. When the pod is gone, the
# kernel moves it back into the namespace of the virtual machine, so the service moves it into the
# next pod again. It is found by its mac-address, because its name changes on the way.
#
# Settings, given by the playbook in /etc/default/ainari-underlay:
#   UNDERLAY_APP      label 'app' of the pod, which gets the interface
#   UNDERLAY_MAC      mac-address of the interface of the underlay
#   UNDERLAY_ADDRESS  address of the gateway on the underlay in CIDR-notation
#   UNDERLAY_MTU      mtu of the underlay
#   UNDERLAY_IFACE    name of the interface within the pod (torii.underlay.iface)

set -u

: "${UNDERLAY_APP:?}" "${UNDERLAY_MAC:?}" "${UNDERLAY_ADDRESS:?}" "${UNDERLAY_MTU:?}" "${UNDERLAY_IFACE:?}"
NAMESPACE="ainari"

CRICTL=(k3s crictl)

# name of the interface with the mac-address of the underlay in the namespace of this virtual
# machine, empty, if it is within a pod
host_iface() {
    ip -o link show | awk -v mac="$UNDERLAY_MAC" '$0 ~ "link/ether " mac { sub(":$", "", $2); sub("@.*", "", $2); print $2 }'
}

while true; do
    IFACE="$(host_iface)"
    if [ -n "$IFACE" ]; then
        SANDBOX_ID="$("${CRICTL[@]}" pods --namespace "$NAMESPACE" --label "app=$UNDERLAY_APP" \
            --state ready -q 2> /dev/null | head -n 1)"
        if [ -n "$SANDBOX_ID" ]; then
            SANDBOX_PID="$("${CRICTL[@]}" inspectp -o go-template --template '{{.info.pid}}' \
                "$SANDBOX_ID" 2> /dev/null)"
            if [ -n "$SANDBOX_PID" ] && [ "$SANDBOX_PID" != "0" ]; then
                echo "Moving $IFACE into the pod of $UNDERLAY_APP (pid $SANDBOX_PID) ..."
                ip link set "$IFACE" down
                if ! ip link set "$IFACE" netns "$SANDBOX_PID"; then
                    echo "Failed to move the underlay into the pod, retrying ..."
                elif nsenter -t "$SANDBOX_PID" -n ip link set "$IFACE" name "$UNDERLAY_IFACE" \
                    && nsenter -t "$SANDBOX_PID" -n ip link set "$UNDERLAY_IFACE" mtu "$UNDERLAY_MTU" \
                    && nsenter -t "$SANDBOX_PID" -n ip addr replace "$UNDERLAY_ADDRESS" dev "$UNDERLAY_IFACE" \
                    && nsenter -t "$SANDBOX_PID" -n ip link set "$UNDERLAY_IFACE" up; then
                    echo "The gateway has its underlay $UNDERLAY_ADDRESS."
                else
                    # back into this virtual machine, so it is moved and configured again
                    echo "Failed to configure the underlay within the pod, retrying ..."
                    POD_IFACE="$(nsenter -t "$SANDBOX_PID" -n bash -c "$(declare -f host_iface); UNDERLAY_MAC=$UNDERLAY_MAC host_iface")"
                    [ -n "$POD_IFACE" ] && nsenter -t "$SANDBOX_PID" -n ip link set "$POD_IFACE" netns 1
                fi
            fi
        fi
    fi
    sleep 2
done
