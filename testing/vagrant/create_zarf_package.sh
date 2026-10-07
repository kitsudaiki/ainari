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
# Creates the offline-package of the vagrant-setup with zarf (testing/vagrant/zarf/zarf.yaml) in
# temporary_files/vagrant/zarf. Next to the package, the directory contains the binary of zarf and
# its init-package, so it has everything to deploy ainari on a kubernetes-cluster without access to
# the internet:
#
#   zarf init zarf-init-amd64-<version>.tar.zst --confirm
#   zarf package deploy zarf-package-ainari-vagrant-amd64-<version>.tar.zst --confirm \
#       --set-variables KVM_GID=<gid> \
#       --set-variables CA_CRT=<base64 of the cert> --set-variables CA_KEY=<base64 of the key>
#
# setup_vagrant_stack.sh --zarf calls this script and the playbook deploys the package, so it only
# has to be called directly to create the package without starting the setup.
#
# Usage:
#   ./testing/vagrant/create_zarf_package.sh               build the images and create the package
#   ./testing/vagrant/create_zarf_package.sh --no-build    create the package of the existing images
#
# Zarf is downloaded into temporary_files/bin, if it is not installed. Another binary can be given
# with ZARF.

set -e

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT_DIR="$PROJECT_DIR/temporary_files/vagrant/zarf"

# the version of the binary and of the init-package have to match
ZARF_VERSION="v0.87.0"
ZARF_RELEASE_URL="https://github.com/zarf-dev/zarf/releases/download/$ZARF_VERSION"
ZARF="${ZARF:-$(command -v zarf 2> /dev/null || echo "$PROJECT_DIR/temporary_files/bin/zarf")}"
INIT_PACKAGE="zarf-init-amd64-$ZARF_VERSION.tar.zst"

if [ ! -x "$ZARF" ]; then
    echo "Downloading zarf $ZARF_VERSION ..."
    mkdir -p "$(dirname "$ZARF")"
    curl -fsSLo "$ZARF" "$ZARF_RELEASE_URL/zarf_${ZARF_VERSION}_Linux_amd64"
    chmod +x "$ZARF"
fi
if [ "$("$ZARF" version)" != "$ZARF_VERSION" ]; then
    echo "zarf $ZARF has the version $("$ZARF" version), but $ZARF_VERSION is required."
    exit 1
fi

mkdir -p "$OUTPUT_DIR"
if [ ! -f "$OUTPUT_DIR/$INIT_PACKAGE" ]; then
    echo "Downloading the init-package of zarf $ZARF_VERSION ..."
    curl -fsSLo "$OUTPUT_DIR/$INIT_PACKAGE.part" "$ZARF_RELEASE_URL/$INIT_PACKAGE"
    mv "$OUTPUT_DIR/$INIT_PACKAGE.part" "$OUTPUT_DIR/$INIT_PACKAGE"
fi
# the binary for the virtual machines, which is the same as the one of the host
cp "$ZARF" "$OUTPUT_DIR/zarf"

# Zarf takes the images ainari/<component>:local from the local docker-daemon. The nix-based
# images are used, like in the setup without zarf.
if [ "$1" != "--no-build" ]; then
    "$PROJECT_DIR/testing/vagrant/build_local_images.sh" nix
fi

echo "Creating the zarf-package ..."
rm -f "$OUTPUT_DIR"/zarf-package-ainari-vagrant-*.tar.zst
"$ZARF" package create "$PROJECT_DIR/testing/vagrant/zarf" --output "$OUTPUT_DIR" --confirm \
    --skip-sbom

echo "The package and everything to deploy it is in $OUTPUT_DIR"
