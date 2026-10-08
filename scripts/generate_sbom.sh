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

# Generates the SBOMs of all docker-images of dockerfiles/nix_based/, which are built with nix.
# The SBOM of an image lists all packages of its runtime-environment of
# dockerfiles/nix_based/nix/packages.nix, which dockerfiles/nix_based/nix/make_rootfs.sh copies
# into the image, with their versions, licenses, patches, CPEs and purls. It is generated with
# sbomnix out of the flake, so the images don't have to be built before. Nix doesn't have to be
# installed, the script runs within the image of nix.
#
#   ./scripts/generate_sbom.sh [--platform linux/amd64|linux/arm64] [<IMAGE>...]
#
# Without images the SBOMs of all images are generated. For another platform than the one of the
# host, docker needs QEMU (see docs/developer/build_base_image.md).
#
# Result in temporary_files/sbom/<PLATFORM>/:
#   <IMAGE>.cdx.json    CycloneDX
#   <IMAGE>.spdx.json   SPDX
#   <IMAGE>.csv         all fields of sbomnix as table
#   versions.csv        overview of all images: image,package,version
#
# The nix-store of the container is kept in the docker-volume 'ainari-sbom-nix', so the packages
# are only downloaded once. It is removed with 'docker volume rm ainari-sbom-nix'.

set -euo pipefail

# the same image of nix like in the Dockerfiles
NIX_IMAGE="nixos/nix:2.35.2@sha256:7a007c766426c1877758ddc5cb87a965ac131fc78c582ce0083d922d51ae945c"
ALL_IMAGES="miko omamori hanami ryokan onsen sakura torii dashboard docs operator"

PLATFORM="linux/$(uname -m | sed -e 's/x86_64/amd64/' -e 's/aarch64/arm64/')"
IMAGES=""
while [ $# -gt 0 ]; do
    case "$1" in
        --platform) PLATFORM="$2"; shift 2 ;;
        *) IMAGES="$IMAGES $1"; shift ;;
    esac
done
IMAGES="${IMAGES:-$ALL_IMAGES}"

REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="$REPO_DIR/temporary_files/sbom/${PLATFORM//\//-}"
mkdir -p "$OUT_DIR"

docker run --rm --platform "$PLATFORM" \
    -v ainari-sbom-nix:/nix \
    -v "$REPO_DIR/dockerfiles/nix_based/nix:/flake:ro" \
    -v "$OUT_DIR:/out" \
    -e IMAGES="$IMAGES" \
    -e HOST_IDS="$(id -u):$(id -g)" \
    "$NIX_IMAGE" \
    sh -c '
        set -eu
        # filter-syscalls is not possible with the emulation of QEMU for another platform
        export NIX_CONFIG="experimental-features = nix-command flakes
filter-syscalls = false"
        echo "image,package,version" > /out/versions.csv
        for image in $IMAGES; do
            echo "=== $image"
            nix develop path:/flake#sbom --command sbomnix "path:/flake#runtime-$image" \
                --cdx "/out/$image.cdx.json" \
                --spdx "/out/$image.spdx.json" \
                --csv "/out/$image.csv"
            # name, pname and version are the first columns and contain no commas. The
            # environment itself is not a package of the image.
            tail -n +2 "/out/$image.csv" | cut -d, -f2,3 | tr -d "\"" \
                | grep -v "^ainari-runtime-" \
                | while read -r line; do echo "$image,$line"; done >> /out/versions.csv
        done
        chown -R "$HOST_IDS" /out
    '

echo "SBOMs written to $OUT_DIR"
