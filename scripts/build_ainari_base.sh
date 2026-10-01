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
# Builds the base-images with the toolchain of the rust-components for linux/amd64 and linux/arm64
# and pushes them to Docker Hub:
#
#   kitsudaiki/ainari_build_base_nix:<VERSION>      dockerfiles/nix_based/Dockerfile_build_base
#   kitsudaiki/ainari_build_base_debian:<VERSION>   dockerfiles/debian_based/Dockerfile_build_base
#
# Usage:
#   ./scripts/build_ainari_base.sh <VERSION>
#
# The platform, which is not the one of the host, is built with the emulation of QEMU, which is
# registered by this script. See docs/developer/build_base_image.md.

set -euo pipefail

VERSION="${1:-}"
if [ -z "$VERSION" ]; then
    echo "Usage: $0 <VERSION>"
    exit 1
fi

BUILDER="multi-builder"
BINFMT_IMAGE="tonistiigi/binfmt:latest"
PLATFORMS="linux/amd64,linux/arm64"

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_DIR"

# ---------------------------------------------------------------------------------------------
# QEMU for the other platform
# ---------------------------------------------------------------------------------------------
case "$(uname -m)" in
    x86_64)        EMULATED_ARCH="arm64"; QEMU_NAME="qemu-aarch64" ;;
    aarch64|arm64) EMULATED_ARCH="amd64"; QEMU_NAME="qemu-x86_64" ;;
    *) echo "Unsupported architecture of the host: $(uname -m)"; exit 1 ;;
esac

docker pull "$BINFMT_IMAGE"

# QEMU has to be at least version 10.1. Older versions don't support the ioctl TCGETS2, which the
# glibc of nixpkgs uses, and the nix-based build fails with 'Inappropriate ioctl for device'.
QEMU_VERSION="$(docker run --rm "$BINFMT_IMAGE" --version 2>&1 \
    | grep -o 'qemu/v[0-9.]*' | cut -dv -f2)"
QEMU_MAJOR="$(echo "$QEMU_VERSION" | cut -d. -f1)"
QEMU_MINOR="$(echo "$QEMU_VERSION" | cut -d. -f2)"
if [ "$QEMU_MAJOR" -lt 10 ] || { [ "$QEMU_MAJOR" -eq 10 ] && [ "$QEMU_MINOR" -lt 1 ]; }; then
    echo "QEMU of $BINFMT_IMAGE is version $QEMU_VERSION, but at least 10.1 is required."
    exit 1
fi

# The old registration is removed before, because it keeps using the binary of the old QEMU.
echo "Registering QEMU $QEMU_VERSION for $EMULATED_ARCH ..."
docker run --privileged --rm "$BINFMT_IMAGE" --uninstall "$QEMU_NAME" > /dev/null
docker run --privileged --rm "$BINFMT_IMAGE" --install "$EMULATED_ARCH" > /dev/null

# ---------------------------------------------------------------------------------------------
# buildx-builder for multiple platforms
# ---------------------------------------------------------------------------------------------
# An existing builder is restarted, because it detects the platforms only at its start.
if docker buildx inspect "$BUILDER" > /dev/null 2>&1; then
    docker buildx stop "$BUILDER"
else
    docker buildx create --name "$BUILDER" > /dev/null
fi

BUILDER_PLATFORMS="$(docker buildx inspect "$BUILDER" --bootstrap | grep '^Platforms:')"
for platform in ${PLATFORMS//,/ }; do
    if ! echo "$BUILDER_PLATFORMS" | grep -q "$platform"; then
        echo "The builder $BUILDER doesn't support $platform: $BUILDER_PLATFORMS"
        exit 1
    fi
done

# ---------------------------------------------------------------------------------------------
# build and push the base-images
# ---------------------------------------------------------------------------------------------
# uses the stored credentials, if already logged in, and asks for them otherwise
docker login

for base in nix debian; do
    echo "Building kitsudaiki/ainari_build_base_${base}:${VERSION} for $PLATFORMS ..."
    docker buildx build \
        --builder "$BUILDER" \
        --platform "$PLATFORMS" \
        -f "dockerfiles/${base}_based/Dockerfile_build_base" \
        -t "kitsudaiki/ainari_build_base_${base}:${VERSION}" \
        --push .
done

echo "Pushed kitsudaiki/ainari_build_base_nix:${VERSION} and" \
     "kitsudaiki/ainari_build_base_debian:${VERSION}."
echo "The Dockerfile_services of both variants and the scripts of scripts/ still use their old" \
     "tag, until it is changed there."
