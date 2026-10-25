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
# Starts the same setup as testing/local_stack/setup_local_stack.sh, but on a kind-cluster (kubernetes in
# docker), and connects the host to it. The stack is deployed by the operator of deploy/operator
# from the Ainari-resource deploy/operator/config/samples/ainari_v1alpha1_ainari_kind.yaml. The
# images of the components and of the operator are built with docker and loaded into the cluster.
#
# With --zarf, the images are not loaded into the cluster, but deployed with the offline-package of
# zarf (scripts/create_zarf_package.sh), which also contains cert-manager, Envoy Gateway and the
# operator. The cluster is initialized with the init-package of zarf and the zarf-agent rewrites
# the images of all pods towards the registry of zarf within the cluster, like in an
# offline-installation. A cluster, which was set up in the other mode before, is created again.
#
# The network-setup needs root, so the script asks for it with sudo: the veth-pair towards the
# gateway at the edge is moved into the network-namespace of its pod and the host has to forward
# and masquerade the traffic of the virtual machines.
#
# Usage:
#   ./testing/kind/setup_kind_stack.sh          start the cluster and connect the host to it
#   ./testing/kind/setup_kind_stack.sh --zarf   the same, but deploy with the offline-package of zarf
#   ./testing/kind/setup_kind_stack.sh --down   delete the cluster and remove the veth-pair again
#
# The binary of kind can be given with the environment-variable KIND.

set -e

# The whole script is one block, which bash reads completely, before it runs it. Otherwise bash
# reads the script piece by piece while running it, and a change of the file during a run would
# make it continue at a random position of the new content.
{

DOWN=false
USE_ZARF=false
for arg in "$@"; do
    case "$arg" in
        --down) DOWN=true ;;
        --zarf) USE_ZARF=true ;;
        *) echo "Usage: $0 [--down|--zarf]"; exit 1 ;;
    esac
done

CLUSTER_NAME="ainari"
NODE="${CLUSTER_NAME}-control-plane"
CONTEXT="kind-${CLUSTER_NAME}"
NAMESPACE="ainari"
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
    ainari/izakaya:local
    ainari/omikuji:local
    ainari/isekai:local
    ainari/ryokan:local
    ainari/onsen:local
    ainari/hanami:local
    ainari/sakura:local
    ainari/torii:local
    ainari/dashboard:local
    ainari/operator:local
)

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CLUSTER_CONFIG="$PROJECT_DIR/testing/kind/cluster.yaml"

# The operator runs in a namespace of its own and deploys the stack of the Ainari-resource.
OPERATOR_DIR="$PROJECT_DIR/deploy/operator"
OPERATOR_IMAGE="ainari/operator:local"
OPERATOR_NAMESPACE="ainari-system"
AINARI_RESOURCE="$OPERATOR_DIR/config/samples/ainari_v1alpha1_ainari_kind.yaml"

# CA, which signs the certificates of all components. It is created once and kept over all runs,
# so it only has to be added to the trust-store of the host once. Its name-constraints limit it
# to the names of the setup, so it can't be misused for any other host, even if its key leaks.
CA_DIR="$PROJECT_DIR/temporary_files/kind"
CA_CERT="$CA_DIR/ainari-kind-ca.crt"
CA_KEY="$CA_DIR/ainari-kind-ca.key"
CA_SECRET="ainari-kind-ca"

# Offline-package of zarf for --zarf. It refers to the images with the names of docker-hub, so the
# Ainari-resource uses them as well, for example kitsudaiki/miko:local instead of ainari/miko:local.
ZARF_DIR="$PROJECT_DIR/temporary_files/zarf"
ZARF_BIN="$ZARF_DIR/zarf"
ZARF_PACKAGE="$ZARF_DIR/zarf-package-ainari-amd64-local.tar.zst"
# zarf has no option for the context, so it gets a kubeconfig of the cluster alone
ZARF_KUBECONFIG="$CA_DIR/zarf-kubeconfig"

SUDO=""
if [ "$EUID" -ne 0 ]; then
    SUDO="sudo"
fi

KUBECTL=(kubectl --context "$CONTEXT" --namespace "$NAMESPACE")

for tool in docker "$KIND" kubectl openssl; do
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

# The ports of a kind-cluster are only published, when it is created, so a cluster, which misses
# a port of the config, like the one of a new component, has to be created again.
cluster_has_all_ports() {
    local port
    for port in $(sed -n 's/.*containerPort: \([0-9]*\).*/\1/p' "$CLUSTER_CONFIG"); do
        docker port "$NODE" "$port/tcp" > /dev/null 2>&1 || return 1
    done
}

# A cluster, which was initialized with zarf, has the namespace 'zarf'
cluster_has_zarf() {
    kubectl --context "$CONTEXT" get namespace zarf > /dev/null 2>&1
}

# The api-server warns about the formats 'int32' and 'int64' in the schemas of the CRDs of
# cert-manager, see below, so these warnings are filtered out.
zarf_cmd() {
    KUBECONFIG="$ZARF_KUBECONFIG" "$ZARF_BIN" "$@" --no-color \
        2> >(grep -v 'unrecognized format' >&2)
}

# removes the rules for the traffic of the virtual machines. net.ipv4.ip_forward stays enabled,
# because docker needs it for its own networks as well.
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

if [ "$DOWN" == true ]; then
    if [ -n "$DEFAULT_IF" ]; then
        remove_nat_rules "$DEFAULT_IF"
    fi
    if cluster_exists; then
        "$KIND" delete cluster --name "$CLUSTER_NAME"
    fi
    docker network rm "$DOCKER_NETWORK" > /dev/null 2>&1 || true
    rm -f "$ZARF_KUBECONFIG"
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
# also the case, while the docker-compose setup of testing/local_stack/setup_local_stack.sh is running.
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
# in this working tree. The debian-based images are used, because they are easier to debug.
KVM_GID="$KVM_GID" "$PROJECT_DIR/testing/kind/build_local_images.sh" debian
echo "Building the image of the operator ..."
docker build -f "$PROJECT_DIR/dockerfiles/nix_based/Dockerfile_operator" -t "$OPERATOR_IMAGE" "$PROJECT_DIR"

if [ "$USE_ZARF" == true ]; then
    # the package of the images, which were just built
    "$PROJECT_DIR/scripts/create_zarf_package.sh" --local --no-build
fi

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

# The zarf-agent rewrites the images of all pods towards the registry of zarf, so a cluster with
# zarf can't run the images loaded by kind, and a cluster without zarf already has cert-manager,
# which zarf can't take over. So the cluster is created again, if the mode changes.
if cluster_exists && [ "$USE_ZARF" != "$(cluster_has_zarf && echo true || echo false)" ]; then
    echo "The kind-cluster $CLUSTER_NAME was set up in the other mode, so it is created again ..."
    "$KIND" delete cluster --name "$CLUSTER_NAME"
fi

if cluster_exists && ! cluster_has_all_ports; then
    echo "The kind-cluster $CLUSTER_NAME misses ports of $CLUSTER_CONFIG, so it is created again ..."
    "$KIND" delete cluster --name "$CLUSTER_NAME"
fi

if ! cluster_exists; then
    echo "Creating the kind-cluster $CLUSTER_NAME ..."
    KIND_EXPERIMENTAL_DOCKER_NETWORK="$DOCKER_NETWORK" \
        "$KIND" create cluster --config "$CLUSTER_CONFIG" --wait 120s
fi

# The node creates its own /dev/kvm, which belongs to the kvm-group of the node and not to the
# one of the host. Sakura runs as a normal user and needs that group to open the virtual machines.
NODE_KVM_GID="$(docker exec "$NODE" stat -c '%g' /dev/kvm)"

if [ "$USE_ZARF" == true ]; then
    mkdir -p "$CA_DIR"
    "$KIND" get kubeconfig --name "$CLUSTER_NAME" > "$ZARF_KUBECONFIG"
    # The init-package deploys the registry and the zarf-agent once and stays over all runs. Zarf
    # starts its registry with an image of a running pod, so the pods of kube-system have to be up.
    if ! kubectl --context "$CONTEXT" --namespace zarf get secret zarf-state > /dev/null 2>&1; then
        kubectl --context "$CONTEXT" --namespace kube-system wait pod --all \
            --for=condition=Ready --timeout=180s > /dev/null
        echo "Initializing the cluster with the init-package of zarf ..."
        (cd "$ZARF_DIR" && zarf_cmd init "$(ls zarf-init-amd64-*.tar.zst)" --confirm)
    fi
else
    echo "Installing cert-manager $CERT_MANAGER_VERSION ..."
    # The api-server doesn't know the formats 'int32' and 'int64' in the schemas of the CRDs of
    # cert-manager. It accepts the CRDs anyway, but warns about every one of them, so these
    # warnings are filtered out.
    kubectl --context "$CONTEXT" apply -f \
        "https://github.com/cert-manager/cert-manager/releases/download/$CERT_MANAGER_VERSION/cert-manager.yaml" \
        2> >(grep -v 'unrecognized format "int\(32\|64\)"' >&2) > /dev/null
    kubectl --context "$CONTEXT" --namespace cert-manager wait deployment --all \
        --for=condition=Available --timeout=300s

    echo "Loading the images into the cluster ..."
    "$KIND" load docker-image --name "$CLUSTER_NAME" "${IMAGES[@]}"
fi

# ---------------------------------------------------------------------------------------------
# deploy the stack
# ---------------------------------------------------------------------------------------------
# The setup keeps no state on purpose, like the docker-compose one: every run starts with empty
# databases and the new images, so a test never sees the virtual machines of a previous run.
# Deleting the namespace removes the Ainari-resource with all components and the secrets, which
# the operator generated.
if "${KUBECTL[@]}" get namespace "$NAMESPACE" > /dev/null 2>&1; then
    echo "Removing the previous namespace $NAMESPACE ..."
    kubectl --context "$CONTEXT" delete namespace "$NAMESPACE" --wait
fi
# the sakura-hosts and their gateways keep their data in a directory of the node (see
# sakura.hostDataPath), which outlives the namespace
docker exec "$NODE" rm -rf /etc/ainari

"$PROJECT_DIR/testing/kind/create_local_ca.sh" "$CA_CERT" "$CA_KEY" "ainari kind-setup CA" \
    "permitted;IP:127.0.0.1/255.255.255.255,permitted;DNS:localhost,permitted;DNS:cluster.local"

"${KUBECTL[@]}" create namespace "$NAMESPACE" > /dev/null
"${KUBECTL[@]}" create secret tls "$CA_SECRET" --cert "$CA_CERT" --key "$CA_KEY" > /dev/null

if [ "$USE_ZARF" == true ]; then
    # The package without its Ainari-resource, because the one of the kind-setup is used. Its
    # variables are only used by this resource, but the ones without default have to be set anyway.
    echo "Deploying the zarf-package with cert-manager, Envoy Gateway and the operator ..."
    zarf_cmd package deploy "$ZARF_PACKAGE" --confirm --components=-ainari \
        --set-variables "FLOATING_IP_CIDR=$FLOATING_IP_CIDR,UPLINK_NEXT_HOP=${HOST_ADDRESS%/*}"
    # Zarf only creates the secret for its registry in the namespaces of the package
    kubectl --context "$CONTEXT" --namespace "$OPERATOR_NAMESPACE" get secret private-registry -o yaml \
        | sed '/namespace:/d;/resourceVersion:/d;/uid:/d;/creationTimestamp:/d' \
        | "${KUBECTL[@]}" apply -f - > /dev/null
    # The image of the operator always has the same tag and the pull-policy IfNotPresent, so the
    # old one is removed from the node, before the operator is restarted below.
    # crictl ignores a filter on the repository together with -q, so it is filtered with awk.
    OLD_OPERATOR_IMAGES="$(docker exec "$NODE" crictl images \
        | awk '$1 == "127.0.0.1:31999/kitsudaiki/ainari_operator" {print $3}')"
    for image in $OLD_OPERATOR_IMAGES; do
        docker exec "$NODE" crictl rmi "$image" > /dev/null 2>&1 || true
    done
    # the images of the package have the names of docker-hub
    RESOURCE_IMAGES=(-e 's|image: ainari/dashboard:local|image: kitsudaiki/ainari_dashboard:local|'
                     -e 's|image: ainari/\(.*\):local|image: kitsudaiki/\1:local|')
else
    echo "Deploying the operator ..."
    kubectl --context "$CONTEXT" kustomize "$OPERATOR_DIR/config/default" \
        | sed "s|kitsudaiki/ainari_operator:develop|$OPERATOR_IMAGE|" \
        | kubectl --context "$CONTEXT" apply --server-side --force-conflicts -f - > /dev/null
    RESOURCE_IMAGES=()
fi
kubectl --context "$CONTEXT" wait crd/ainaris.ainari.cloud \
    --for=condition=Established --timeout=60s > /dev/null
# The image always has the same tag, so a running operator is restarted to get the new one.
kubectl --context "$CONTEXT" --namespace "$OPERATOR_NAMESPACE" \
    rollout restart deployment/ainari-operator > /dev/null
kubectl --context "$CONTEXT" --namespace "$OPERATOR_NAMESPACE" \
    rollout status deployment/ainari-operator --timeout=120s

# The operator generates all keys and passwords, but takes over a secret, which already exists,
# and never replaces it. So the passphrase of the test-user, which the cli and the tests use, is
# created before.
"${KUBECTL[@]}" create secret generic miko-admin --from-literal=passphrase=asdfasdf > /dev/null

echo "Creating the Ainari-resource ..."
sed -e "s/^\( *kvmGid:\).*/\1 $NODE_KVM_GID/" "${RESOURCE_IMAGES[@]}" "$AINARI_RESOURCE" \
    | "${KUBECTL[@]}" apply -f - > /dev/null

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
# the operator reports the readiness of all components in the status of the resource
if ! "${KUBECTL[@]}" wait ainari/ainari --for=condition=Ready --timeout=900s; then
    "${KUBECTL[@]}" get ainari/ainari -o jsonpath='{.status.conditions[?(@.type=="Ready")].message}{"\n"}'
    exit 1
fi

# ---------------------------------------------------------------------------------------------
# mark the onsen of the backups as backup-target, so isekai stores the backups there
# ---------------------------------------------------------------------------------------------
echo "Marking the onsen of the backups as backup-target ..."
"$PROJECT_DIR/testing/local_stack/mark_backup_targets.sh" \
    "https://127.0.0.1:11417" "https://127.0.0.1:11416" "$CA_CERT" || true

echo ""
echo "The stack is up. The api is reachable over https at:"
echo "    miko     https://127.0.0.1:11417"
echo "    hanami   https://127.0.0.1:11418"
echo "    ryokan   https://127.0.0.1:11416"
echo "    omamori  https://127.0.0.1:11421"
echo "    omikuji  https://127.0.0.1:11424"
echo "    isekai   https://127.0.0.1:11425"
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
echo "    kubectl --context $CONTEXT --namespace $NAMESPACE get ainari ainari -o yaml"
echo "    kubectl --context $CONTEXT --namespace $OPERATOR_NAMESPACE logs deployment/ainari-operator"
echo ""
echo "The two sakura-hosts (the pods sakura-0 and sakura-1, each with its own torii) register"
echo "themselves in hanami, as soon as it is up."
echo ""
echo "Now the test can be started as a normal user:"
echo "    AINARI_MIKO_ADDRESS=https://127.0.0.1:11417 python3 testing/ainari_test/vm_lifecycle_test.py"
echo "and the cli with 'ainarictl --insecure' and AINARI_ADDRESS=https://127.0.0.1:11417"

exit 0
}
