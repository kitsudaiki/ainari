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
# Starts the setup of testing/vagrant: four virtual machines with nested virtualization and a
# kubernetes-cluster (k3s), on which ansible deploys the helm-chart of deploy/k8s/ainari. The
# images are built on the host and copied into the virtual machines.
#
# Adding the route towards the floating ip-addresses on the host needs root, so the script asks
# for it with sudo.
#
# Usage:
#   ./scripts/setup_vagrant_stack.sh          start the virtual machines and deploy ainari
#   ./scripts/setup_vagrant_stack.sh --down   destroy the virtual machines and remove the route

set -e

# The whole script is one block, which bash reads completely, before it runs it. Otherwise bash
# reads the script piece by piece while running it, and a change of the file during a run would
# make it continue at a random position of the new content.
{

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VAGRANT_DIR="$PROJECT_DIR/testing/vagrant"
WORK_DIR="$PROJECT_DIR/temporary_files/vagrant"
IMAGE_DIR="$WORK_DIR/images"

# the virtual machine, which ansible is started from, after all of them are up
LAST_MACHINE="ainari-sakura-2"

# The floating ip-addresses are served by the gateway at the edge, which runs on the virtual
# machine ainari-torii. The host reaches them over the private network of vagrant.
FLOATING_IP_CIDR="10.0.0.0/24"
TORII_VM_ADDRESS="192.168.56.11"
MGMT_VM_ADDRESS="192.168.56.10"

# CA, which signs the certificates of all components, see scripts/create_local_ca.sh
CA_CERT="$WORK_DIR/ainari-vagrant-ca.crt"
CA_KEY="$WORK_DIR/ainari-vagrant-ca.key"

IMAGES=(miko omamori ryokan onsen hanami sakura torii dashboard)

SUDO=""
if [ "$EUID" -ne 0 ]; then
    SUDO="sudo"
fi

for tool in docker vagrant ansible-playbook openssl; do
    if ! command -v "$tool" > /dev/null 2>&1; then
        echo "'$tool' not found, but it is required for the vagrant-setup."
        exit 1
    fi
done
if ! vagrant plugin list 2> /dev/null | grep -q '^vagrant-libvirt '; then
    echo "The vagrant-plugin 'vagrant-libvirt' is required for the vagrant-setup."
    exit 1
fi

cd "$VAGRANT_DIR"

if [ "$1" == "--down" ]; then
    vagrant destroy --force
    $SUDO ip route delete "$FLOATING_IP_CIDR" via "$TORII_VM_ADDRESS" > /dev/null 2>&1 || true
    echo "Virtual machines destroyed and the route removed."
    exit 0
fi

# ---------------------------------------------------------------------------------------------
# check that nothing else on the host owns the floating ip-addresses
# ---------------------------------------------------------------------------------------------
# The docker-compose setup and the kind-setup put the first floating ip-address on an interface
# of the host, which would steal the traffic of the floating ip-addresses.
FIP_PREFIX="$(echo "$FLOATING_IP_CIDR" | cut -d. -f1-3)."
CONFLICTS="$(ip -o -4 addr show | awk '{print $2, $4}' | grep " ${FIP_PREFIX}" | awk '{print $1}' | sort -u)"

if [ -n "$CONFLICTS" ]; then
    echo "The following interfaces of the host already have an address in $FLOATING_IP_CIDR:"
    for iface in $CONFLICTS; do
        echo "    $iface"
    done
    echo "They collide with the floating ip-addresses of the setup. Stop the other setup with"
    echo "'make down local' or 'make down kind' or remove them, for example with"
    echo "    sudo ip link delete <interface>"
    exit 1
fi

# ---------------------------------------------------------------------------------------------
# build the images and the CA
# ---------------------------------------------------------------------------------------------
# Always rebuild first: starting with stale images silently runs a different version than the one
# in this working tree.
"$PROJECT_DIR/scripts/build_local_images.sh"

echo "Saving the images for the virtual machines ..."
mkdir -p "$IMAGE_DIR"
for image in "${IMAGES[@]}"; do
    docker save -o "$IMAGE_DIR/$image.tar" "ainari/$image:local"
done

"$PROJECT_DIR/scripts/create_local_ca.sh" "$CA_CERT" "$CA_KEY" "ainari vagrant-setup CA" \
    "permitted;IP:192.168.56.0/255.255.255.0,permitted;DNS:cluster.local"

# ---------------------------------------------------------------------------------------------
# start the virtual machines and deploy ainari
# ---------------------------------------------------------------------------------------------
echo "Starting the virtual machines ..."
vagrant up --no-provision

# Ansible runs once for all virtual machines and is attached to the last one, see the Vagrantfile.
echo "Installing kubernetes and ainari with ansible ..."
vagrant provision "$LAST_MACHINE"

# ---------------------------------------------------------------------------------------------
# connect the host to the floating ip-addresses
# ---------------------------------------------------------------------------------------------
echo "Adding the route towards $FLOATING_IP_CIDR over ainari-torii ..."
$SUDO ip route replace "$FLOATING_IP_CIDR" via "$TORII_VM_ADDRESS"

echo ""
echo "The stack is up. The api is reachable over https at:"
echo "    miko       https://$MGMT_VM_ADDRESS:11417"
echo "    hanami     https://$MGMT_VM_ADDRESS:11418"
echo "    ryokan     https://$MGMT_VM_ADDRESS:11416"
echo "    omamori    https://$MGMT_VM_ADDRESS:11421"
echo "    torii      https://$TORII_VM_ADDRESS:11419"
echo "    dashboard  https://$MGMT_VM_ADDRESS:11422"
echo ""
echo "All certificates are signed by the CA $CA_CERT"
echo "It stays the same over all runs, so it only has to be trusted once, see"
echo "testing/local_stack/Readme.md."
echo ""
echo "The cluster can be inspected with"
echo "    kubectl --kubeconfig $WORK_DIR/kubeconfig --namespace ainari get pods -o wide"
echo "and the virtual machines with 'vagrant ssh <name>' in $VAGRANT_DIR."
echo ""
echo "Now the test can be started as a normal user:"
echo "    AINARI_MIKO_ADDRESS=https://$MGMT_VM_ADDRESS:11417 python3 testing/local_stack/vm_lifecycle_test.py"

exit 0
}
