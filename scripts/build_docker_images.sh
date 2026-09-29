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

# Builds the images of all components with the tag 'local_test' and saves them in
# temporary_files/ainari_docker_files.tar.
#
# Usage:
#   ./scripts/build_docker_images.sh [debian|nix]
#
# The argument selects the Dockerfiles of dockerfiles/debian_based or dockerfiles/nix_based and
# defaults to 'nix', like the images of the CI.

# stop at the first failed build, so the tar-file never contains an older image with the same tag
set -e

BASE="${1:-nix}"
case "$BASE" in
    debian|nix) ;;
    *) echo "Usage: $0 [debian|nix]"; exit 1 ;;
esac
DOCKERFILES="dockerfiles/${BASE}_based"

# the images of the components are built on this image, which Dockerfile_services uses by default
docker build -f "$DOCKERFILES/Dockerfile_build_base" -t "kitsudaiki/ainari_build_base_$BASE:0.5.0" .

for target in hanami miko omamori onsen ryokan sakura; do
    docker build -f "$DOCKERFILES/Dockerfile_services" --target "$target" \
        -t "kitsudaiki/$target:local_test" .
done
docker build -f "$DOCKERFILES/Dockerfile_torii" -t kitsudaiki/torii:local_test .
docker build -f "$DOCKERFILES/Dockerfile_dashboard" -t kitsudaiki/ainari_dashboard:local_test .

mkdir -p temporary_files
docker save -o temporary_files/ainari_docker_files.tar \
    kitsudaiki/hanami:local_test \
    kitsudaiki/miko:local_test \
    kitsudaiki/omamori:local_test \
    kitsudaiki/onsen:local_test \
    kitsudaiki/ryokan:local_test \
    kitsudaiki/sakura:local_test \
    kitsudaiki/torii:local_test \
    kitsudaiki/ainari_dashboard:local_test
