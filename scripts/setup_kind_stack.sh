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
# Starts the same setup as scripts/setup_local_stack.sh, but on a kind-cluster (kubernetes in
# docker) with the helm-chart of deploy/k8s/ainari, and connects the host to it. The images are
# built with docker and loaded into the cluster.
#
# The network-setup needs root, so the script asks for it with sudo: the veth-pair towards the
# gateway at the edge is moved into the network-namespace of its pod and the host has to forward
# and masquerade the traffic of the virtual machines.
#
# Usage:
#   ./scripts/setup_kind_stack.sh          start the cluster and connect the host to it
#   ./scripts/setup_kind_stack.sh --down   delete the cluster and remove the veth-pair again
#
# The binary of kind can be given with the environment-variable KIND.

set -e

# The whole script is one block, which bash reads completely, before it runs it. Otherwise bash
# reads the script piece by piece while running it, and a change of the file during a run would
# make it continue at a random position of the new content.
{

CLUSTER_NAME="ainari"
NODE="${CLUSTER_NAME}-control-plane"
CONTEXT="kind-${CLUSTER_NAME}"
NAMESPACE="ainari"
RELEASE="ainari"
# the components talk https to each other with self-signed certificates of cert-manager
CERT_MANAGER_VERSION="v1.18.2"
KIND="${KIND:-kind}"

# The overlay adds a header to every packet, so the underlay needs a bigger mtu. The pods get the
# mtu of the network of the node, so the cluster runs in a docker-network of its own.
DOCKER_NETWORK="ainari-kind"
DOCKER_NETWORK_MTU=1600

# range of the floating ip-addresses, which is configured in the values of hanami. The host owns
# the first address of that range and is the next hop of the gateway at the edge.
FLOATING_IP_CIDR="10.0.0.0/24"
HOST_ADDRESS="10.0.0.1/24"
# address of the gateway on the other end of the veth-pair. The floating ip-addresses are served
# on that interface, so it sits in the same subnet as the host.
GATEWAY_ADDRESS="10.0.0.254/24"
# The end of the veth-pair on the host. It is named differently than the one of the
# docker-compose setup, so the one setup never removes the interface of the other one.
HOST_IFACE="veth-kind"
# The end of the veth-pair in the pod of the gateway at the edge, which waits for it
GATEWAY_IFACE="veth-gw"

IMAGES=(
    ainari/miko:local
    ainari/omamori:local
    ainari/ryokan:local
    ainari/onsen:local
    ainari/hanami:local
    ainari/sakura:local
    ainari/torii:local
    ainari/dashboard:local
)

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CHART_DIR="$PROJECT_DIR/deploy/k8s/ainari"
KIND_DIR="$PROJECT_DIR/deploy/k8s/kind"

# CA, which signs the certificates of all components. It is created once and kept over all runs,
# so it only has to be added to the trust-store of the host once. Its name-constraints limit it
# to the names of the setup, so it can't be misused for any other host, even if its key leaks.
CA_DIR="$PROJECT_DIR/temporary_files/kind"
CA_CERT="$CA_DIR/ainari-kind-ca.crt"
CA_KEY="$CA_DIR/ainari-kind-ca.key"
CA_SECRET="ainari-kind-ca"

SUDO=""
if [ "$EUID" -ne 0 ]; then
    SUDO="sudo"
fi

KUBECTL=(kubectl --context "$CONTEXT" --namespace "$NAMESPACE")
HELM=(helm --kube-context "$CONTEXT" --namespace "$NAMESPACE")

for tool in docker "$KIND" kubectl helm openssl; do
    if ! command -v "$tool" > /dev/null 2>&1; then
        echo "'$tool' not found, but it is required for the kind-setup."
        exit 1
    fi
done

cd "$PROJECT_DIR"

# ---------------------------------------------------------------------------------------------
# helper
# ---------------------------------------------------------------------------------------------
cluster_exists() {
    "$KIND" get clusters 2> /dev/null | grep -qx "$CLUSTER_NAME"
}

remove_nat_rules() {
    local default_if="$1"
    $SUDO iptables -t nat -D POSTROUTING -s "$FLOATING_IP_CIDR" -o "$default_if" -j MASQUERADE 2>/dev/null || true
    $SUDO iptables -D FORWARD -s "$FLOATING_IP_CIDR" -j ACCEPT 2>/dev/null || true
    $SUDO iptables -D FORWARD -d "$FLOATING_IP_CIDR" -j ACCEPT 2>/dev/null || true
}

DEFAULT_IF=$(ip route show default | awk '/default/ {print $5}' | head -n 1)

# ---------------------------------------------------------------------------------------------
# cleanup of a previous run
# ---------------------------------------------------------------------------------------------
echo "Cleaning up a previous setup ..."
$SUDO ip link delete "$HOST_IFACE" > /dev/null 2>&1 || true

if [ "$1" == "--down" ]; then
    if [ -n "$DEFAULT_IF" ]; then
        remove_nat_rules "$DEFAULT_IF"
    fi
    if cluster_exists; then
        "$KIND" delete cluster --name "$CLUSTER_NAME"
    fi
    docker network rm "$DOCKER_NETWORK" > /dev/null 2>&1 || true
    echo "Cluster deleted and the veth-pair removed."
    exit 0
fi

# /dev/kvm belongs to a group, whose id differs between hosts. The images are built with the one
# of the host, like in the docker-compose setup.
KVM_GID="$(stat -c '%g' /dev/kvm 2>/dev/null || echo "")"
if [ -z "$KVM_GID" ]; then
    echo "/dev/kvm not found. The host needs kvm to run the virtual machines."
    exit 1
fi
export KVM_GID

# ---------------------------------------------------------------------------------------------
# check that nothing else on the host owns the floating ip-addresses
# ---------------------------------------------------------------------------------------------
# Another interface in the same subnet steals the traffic of the floating ip-addresses. This is
# also the case, while the docker-compose setup of scripts/setup_local_stack.sh is running.
FIP_PREFIX="$(echo "$FLOATING_IP_CIDR" | cut -d. -f1-3)."
CONFLICTS="$(ip -o -4 addr show | awk '{print $2, $4}' | grep " ${FIP_PREFIX}" | awk '{print $1}' | sort -u)"

if [ -n "$CONFLICTS" ]; then
    echo "The following interfaces of the host already have an address in $FLOATING_IP_CIDR:"
    for iface in $CONFLICTS; do
        echo "    $iface"
    done
    echo "They collide with the floating ip-addresses of the setup. Stop the docker-compose setup"
    echo "with 'make down local' or remove them, for example with"
    echo "    sudo ip link delete <interface>"
    exit 1
fi

# ---------------------------------------------------------------------------------------------
# build the images
# ---------------------------------------------------------------------------------------------
# Always rebuild first: starting with stale images silently runs a different version than the one
# in this working tree.
KVM_GID="$KVM_GID" "$PROJECT_DIR/scripts/build_local_images.sh"

# ---------------------------------------------------------------------------------------------
# create the cluster
# ---------------------------------------------------------------------------------------------
if ! docker network inspect "$DOCKER_NETWORK" > /dev/null 2>&1; then
    echo "Creating the docker-network $DOCKER_NETWORK with mtu $DOCKER_NETWORK_MTU ..."
    docker network create \
        --driver bridge \
        --opt "com.docker.network.driver.mtu=$DOCKER_NETWORK_MTU" \
        "$DOCKER_NETWORK" > /dev/null
fi

if ! cluster_exists; then
    echo "Creating the kind-cluster $CLUSTER_NAME ..."
    KIND_EXPERIMENTAL_DOCKER_NETWORK="$DOCKER_NETWORK" \
        "$KIND" create cluster --config "$KIND_DIR/cluster.yaml" --wait 120s
fi

# The node creates its own /dev/kvm, which belongs to the kvm-group of the node and not to the
# one of the host. Sakura runs as a normal user and needs that group to open the virtual machines.
NODE_KVM_GID="$(docker exec "$NODE" stat -c '%g' /dev/kvm)"

echo "Installing cert-manager $CERT_MANAGER_VERSION ..."
# The api-server doesn't know the formats 'int32' and 'int64' in the schemas of the CRDs of
# cert-manager. It accepts the CRDs anyway, but warns about every one of them, so these warnings
# are filtered out.
kubectl --context "$CONTEXT" apply -f \
    "https://github.com/cert-manager/cert-manager/releases/download/$CERT_MANAGER_VERSION/cert-manager.yaml" \
    2> >(grep -v 'unrecognized format "int\(32\|64\)"' >&2) > /dev/null
kubectl --context "$CONTEXT" --namespace cert-manager wait deployment --all \
    --for=condition=Available --timeout=300s

echo "Loading the images into the cluster ..."
"$KIND" load docker-image --name "$CLUSTER_NAME" "${IMAGES[@]}"

# ---------------------------------------------------------------------------------------------
# deploy the helm-chart
# ---------------------------------------------------------------------------------------------
# The setup keeps no state on purpose, like the docker-compose one: every run starts with empty
# databases and the new images, so a test never sees the virtual machines of a previous run.
if "${HELM[@]}" status "$RELEASE" > /dev/null 2>&1; then
    echo "Removing the previous deployment ..."
    "${HELM[@]}" uninstall "$RELEASE" --wait
fi
if "${KUBECTL[@]}" get namespace "$NAMESPACE" > /dev/null 2>&1; then
    kubectl --context "$CONTEXT" delete namespace "$NAMESPACE" --wait
fi

"$PROJECT_DIR/scripts/create_local_ca.sh" "$CA_CERT" "$CA_KEY" "ainari kind-setup CA" \
    "permitted;IP:127.0.0.1/255.255.255.255,permitted;DNS:localhost,permitted;DNS:cluster.local"

"${KUBECTL[@]}" create namespace "$NAMESPACE" > /dev/null
"${KUBECTL[@]}" create secret tls "$CA_SECRET" --cert "$CA_CERT" --key "$CA_KEY" > /dev/null

echo "Deploying the helm-chart ..."
"${HELM[@]}" install "$RELEASE" "$CHART_DIR" \
    --values "$KIND_DIR/values.yaml" \
    --set "sakura.kvm_gid=$NODE_KVM_GID"

# ---------------------------------------------------------------------------------------------
# connect the host to the gateway at the edge of the network
# ---------------------------------------------------------------------------------------------
echo "Waiting for the pod of torii-public to come up ..."
SANDBOX_ID=""
for _ in $(seq 1 300); do
    SANDBOX_ID="$(docker exec "$NODE" crictl pods \
        --namespace "$NAMESPACE" --label app=torii-public --state ready -q | head -n 1)"
    [ -z "$SANDBOX_ID" ] || break
    sleep 1
done
if [ -z "$SANDBOX_ID" ]; then
    echo "The pod of torii-public did not come up. See"
    echo "    kubectl --context $CONTEXT --namespace $NAMESPACE describe pod -l app=torii-public"
    exit 1
fi

# The pod runs within the node, which is a container itself. So the veth-pair is created on the
# host, one end is moved into the network-namespace of the node and from there into the one of the
# pod. The pid of the pod is only known within the node.
NODE_PID="$(docker inspect -f '{{.State.Pid}}' "$NODE")"
SANDBOX_PID="$(docker exec "$NODE" crictl inspectp -o go-template --template '{{.info.pid}}' "$SANDBOX_ID")"

echo "Injecting $GATEWAY_IFACE into the pod of torii-public (pid $SANDBOX_PID within $NODE) ..."
docker exec "$NODE" ip link delete "$GATEWAY_IFACE" > /dev/null 2>&1 || true
$SUDO ip link add "$HOST_IFACE" type veth peer name "$GATEWAY_IFACE"

# The host only reaches the floating ip-addresses. The internal addresses of the virtual machines
# stay behind the gateway, which translates them.
$SUDO ip addr add "$HOST_ADDRESS" dev "$HOST_IFACE"
$SUDO ip link set "$HOST_IFACE" up

# No checksum offloading: the datapath of the gateway rewrites addresses with incremental
# checksum updates, which are only correct on complete checksums. The gateway does the same on
# its end, as soon as it sees the interface.
$SUDO ethtool -K "$HOST_IFACE" tx off rx off > /dev/null 2>&1 || true

$SUDO ip link set "$GATEWAY_IFACE" netns "$NODE_PID"
docker exec "$NODE" ip link set "$GATEWAY_IFACE" netns "$SANDBOX_PID"
docker exec "$NODE" nsenter -t "$SANDBOX_PID" -n ip addr add "$GATEWAY_ADDRESS" dev "$GATEWAY_IFACE"
docker exec "$NODE" nsenter -t "$SANDBOX_PID" -n ip link set "$GATEWAY_IFACE" up

# ---------------------------------------------------------------------------------------------
# let the virtual machines reach the internet through the host
# ---------------------------------------------------------------------------------------------
echo "Enabling forwarding and NAT for $FLOATING_IP_CIDR ..."
$SUDO sysctl -w net.ipv4.ip_forward=1 > /dev/null

if [ -n "$DEFAULT_IF" ]; then
    # remove the rules of a previous run first, so they are not added twice
    remove_nat_rules "$DEFAULT_IF"

    $SUDO iptables -t nat -A POSTROUTING -s "$FLOATING_IP_CIDR" -o "$DEFAULT_IF" -j MASQUERADE
    $SUDO iptables -A FORWARD -s "$FLOATING_IP_CIDR" -j ACCEPT
    $SUDO iptables -A FORWARD -d "$FLOATING_IP_CIDR" -j ACCEPT
else
    echo "WARNING: no default route found on the host, so the virtual machines have no internet."
fi

# ---------------------------------------------------------------------------------------------
# wait for the components
# ---------------------------------------------------------------------------------------------
echo "Waiting for the components to become ready ..."
for resource in deployment/miko deployment/omamori deployment/ryokan deployment/hanami \
                deployment/torii-public deployment/dashboard statefulset/onsen statefulset/sakura; do
    "${KUBECTL[@]}" rollout status "$resource" --timeout=600s
done

echo ""
echo "The stack is up. The api is reachable over https at:"
echo "    miko     https://127.0.0.1:11417"
echo "    hanami   https://127.0.0.1:11418"
echo "    ryokan   https://127.0.0.1:11416"
echo "    omamori  https://127.0.0.1:11421"
echo "    torii    https://127.0.0.1:11419"
echo ""
echo "The dashboard is reachable at https://127.0.0.1:11422."
echo ""
echo "All certificates are signed by the CA $CA_CERT"
echo "It stays the same over all runs, so it only has to be trusted once, see"
echo "testing/local_stack/Readme.md. Otherwise the certificates have to be accepted in the browser"
echo "for every port and the clients have to skip their verification."
echo ""
echo "The cluster can be inspected with"
echo "    kubectl --context $CONTEXT --namespace $NAMESPACE get pods"
echo ""
echo "The two sakura-hosts (the pods sakura-0 and sakura-1, each with its own torii) register"
echo "themselves in hanami, as soon as it is up."
echo ""
echo "Now the test can be started as a normal user:"
echo "    AINARI_MIKO_ADDRESS=https://127.0.0.1:11417 python3 testing/local_stack/vm_lifecycle_test.py"
echo "and the cli with 'ainarictl --insecure' and AINARI_ADDRESS=https://127.0.0.1:11417"

exit 0
}
