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
# Sets the version of all components in the repository to the same value:
#   - the rust-workspace (Cargo.toml, which all crates inherit, and Cargo.lock)
#   - the helm-chart (version and appVersion)
#   - the python-sdk (setup.py and __init__.py)
#   - the cli (ainarictl)
#   - the dashboard (package.json)
#
# Usage:
#   ./scripts/update_version.sh 0.10.0

set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Usage: $0 <version>    (for example: $0 0.10.0)" >&2
    exit 1
fi

VERSION="${1#v}"
if ! [[ "${VERSION}" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
    echo "'${VERSION}' is not a valid semantic version, like 0.10.0 or 0.10.0-rc1" >&2
    exit 1
fi

cd "$(dirname "$0")/.."

# replaces the first match of a sed-expression within a file and fails, if nothing was replaced,
# so a changed file-layout doesn't get lost silently
replace() {
    local file="$1"
    local pattern="$2"
    local replacement="$3"

    if ! grep -qE "${pattern}" "${file}"; then
        echo "no version found in ${file}" >&2
        exit 1
    fi
    sed -i -E "0,/${pattern}/s//${replacement}/" "${file}"
    echo "updated ${file}"
}

# rust-workspace: only the version within [workspace.package], which is inherited by all crates
sed -i -E "/^\[workspace\.package\]/,/^\[/ s/^version = \".*\"/version = \"${VERSION}\"/" Cargo.toml
grep -qE "^version = \"${VERSION}\"" Cargo.toml || { echo "no version found in Cargo.toml" >&2; exit 1; }
echo "updated Cargo.toml"
if command -v cargo > /dev/null; then
    cargo update --workspace --offline --quiet
    echo "updated Cargo.lock"
else
    echo "cargo not found, Cargo.lock is updated with the next build" >&2
fi

# helm-chart
replace deploy/k8s/ainari/Chart.yaml '^version: .*' "version: ${VERSION}"
replace deploy/k8s/ainari/Chart.yaml '^appVersion: .*' "appVersion: \"${VERSION}\""

# python-sdk. The version in setup.py is only the default, the ci overwrites it with the tag.
replace src/sdk/python/ainari_sdk/setup.py \
    "os\.getenv\('PYTHON_PACKAGE_VERSION', '[^']*'\)" \
    "os.getenv('PYTHON_PACKAGE_VERSION', '${VERSION}')"
replace src/sdk/python/ainari_sdk/ainari_sdk/__init__.py \
    '^__version__ = ".*"' "__version__ = \"${VERSION}\""

# cli
replace src/cli/ainarictl/main.go '^var version = ".*"' "var version = \"${VERSION}\""

# dashboard
replace src/dashboard/app/package.json '^  "version": ".*",' "  \"version\": \"${VERSION}\","

echo "all versions set to ${VERSION}"
