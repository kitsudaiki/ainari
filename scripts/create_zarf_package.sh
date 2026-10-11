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
# Creates the offline-installation of the ainari-stack with zarf (deploy/zarf/zarf.yaml). The output
# directory contains everything to deploy the stack on a kubernetes without access to the internet:
#
#   zarf                                       binary of zarf for the target-architecture
#   zarf-init-<arch>-<zarf-version>.tar.zst    init-package of zarf with the registry and the agent
#   zarf-package-ainari-<arch>-<version>.tar.zst
#
# Usage:
#   ./scripts/create_zarf_package.sh [options]
#
# Options:
#   --version VERSION   tag of the images of docker-hub, for example '0.21.0' (default 'develop')
#   --local             use the locally built images of this working tree instead of the ones of
#                       docker-hub, they are built before, with the tag 'local' (only amd64)
#   --no-build          together with --local: use the existing local images without building them
#   --arch ARCH         target-architecture, 'amd64' or 'arm64' (default 'amd64')
#   --output DIR        output directory (default temporary_files/zarf)
#
# Zarf is downloaded into temporary_files/bin, if it is not installed. Another binary can be given
# with ZARF_BINARY, but it must have the version of ZARF_VERSION, because of the init-package. See
# docs/developer/scripts.md and docs/deployer/installation/offline_installation.md.

set -e

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# the version of the binary and of the init-package have to match
ZARF_VERSION="v0.87.0"
ZARF_RELEASE_URL="https://github.com/zarf-dev/zarf/releases/download/$ZARF_VERSION"

VERSION="develop"
LOCAL=false
BUILD=true
ARCH="amd64"
OUTPUT_DIR="$PROJECT_DIR/temporary_files/zarf"

while [ $# -gt 0 ]; do
    case "$1" in
        --version) VERSION="$2"; shift 2 ;;
        --local) LOCAL=true; shift ;;
        --no-build) BUILD=false; shift ;;
        --arch) ARCH="$2"; shift 2 ;;
        --output) OUTPUT_DIR="$2"; shift 2 ;;
        *) sed -n '/^# Usage:/,/^# Zarf is/p' "$0" | sed 's/^# \{0,1\}//' | head -n -1; exit 1 ;;
    esac
done

case "$ARCH" in
    amd64|arm64) ;;
    *) echo "Unsupported architecture '$ARCH', expected amd64 or arm64."; exit 1 ;;
esac
if [ "$LOCAL" == true ] && [ "$ARCH" != amd64 ]; then
    echo "The local images are only built for amd64."
    exit 1
fi

# downloads a file of the release of zarf, if it doesn't exist yet
download() {
    local file="$1"
    local target="$2"
    if [ ! -f "$target" ]; then
        echo "Downloading $file of zarf $ZARF_VERSION ..."
        mkdir -p "$(dirname "$target")"
        curl -fsSLo "$target.part" "$ZARF_RELEASE_URL/$file"
        mv "$target.part" "$target"
    fi
}

# the binary of zarf for this host, which creates the package
ZARF="${ZARF_BINARY:-$(command -v zarf 2> /dev/null || echo "$PROJECT_DIR/temporary_files/bin/zarf")}"
if [ ! -x "$ZARF" ]; then
    download "zarf_${ZARF_VERSION}_Linux_amd64" "$ZARF"
    chmod +x "$ZARF"
fi
if [ "$("$ZARF" version)" != "$ZARF_VERSION" ]; then
    echo "zarf $ZARF has the version $("$ZARF" version), but $ZARF_VERSION is required."
    exit 1
fi

# the binary of zarf and the init-package for the target, which is offline
mkdir -p "$OUTPUT_DIR"
download "zarf_${ZARF_VERSION}_Linux_$ARCH" "$OUTPUT_DIR/zarf"
chmod +x "$OUTPUT_DIR/zarf"
download "zarf-init-$ARCH-$ZARF_VERSION.tar.zst" "$OUTPUT_DIR/zarf-init-$ARCH-$ZARF_VERSION.tar.zst"

if [ "$LOCAL" == true ]; then
    VERSION="local"
    if [ "$BUILD" == true ]; then
        # the nix-based images are the same as the ones of the CI
        "$PROJECT_DIR/testing/kind/build_local_images.sh" nix
        docker build -f "$PROJECT_DIR/dockerfiles/nix_based/Dockerfile_operator" \
            -t ainari/operator:local "$PROJECT_DIR"
    fi
    # The package refers to the names of docker-hub, so the local images get these names as well.
    # Zarf takes them from the local docker-daemon, because they don't exist on docker-hub.
    for component in miko hanami ryokan omamori omikuji izakaya onsen sakura torii; do
        docker tag "ainari/$component:local" "kitsudaiki/$component:local"
    done
    docker tag ainari/dashboard:local kitsudaiki/ainari_dashboard:local
    docker tag ainari/operator:local kitsudaiki/ainari_operator:local
fi

echo "Creating the zarf-package of the version $VERSION for $ARCH ..."
rm -f "$OUTPUT_DIR/zarf-package-ainari-$ARCH-$VERSION.tar.zst"
"$ZARF" package create "$PROJECT_DIR/deploy/zarf" --output "$OUTPUT_DIR" --confirm \
    --architecture "$ARCH" --set "AINARI_VERSION=$VERSION" --skip-sbom

echo "The offline-installation is in $OUTPUT_DIR:"
ls -lh "$OUTPUT_DIR"
