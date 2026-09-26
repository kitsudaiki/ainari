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
# Builds the images of all components for the kubernetes-based local setups (kind and vagrant) with
# the tag 'local'. They are the same images as the ones of the docker-compose setup, with the
# faster 'local'-profile of cargo, but built without docker compose.
#
# Usage:
#   ./scripts/build_local_images.sh
#
# The id of the group of /dev/kvm, which sakura is built with, can be given with KVM_GID. It
# defaults to the one of the host. The pods get the id of their node anyway, see sakura.kvm_gid
# in the values of the helm-chart.

set -e

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_DIR"

KVM_GID="${KVM_GID:-$(stat -c '%g' /dev/kvm 2> /dev/null || echo 993)}"

echo "Building the images ..."
for target in miko omamori ryokan onsen hanami; do
    docker build -f dockerfiles/Dockerfile_services --target "$target" \
        --build-arg CARGO_PROFILE=local -t "ainari/$target:local" .
done
docker build -f dockerfiles/Dockerfile_services --target sakura \
    --build-arg CARGO_PROFILE=local --build-arg "KVM_GID=$KVM_GID" -t ainari/sakura:local .
docker build -f dockerfiles/Dockerfile_torii \
    --build-arg CARGO_PROFILE=local -t ainari/torii:local .
docker build -f dockerfiles/Dockerfile_dashboard -t ainari/dashboard:local .
