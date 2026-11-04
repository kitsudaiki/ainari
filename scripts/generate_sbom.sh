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

# Generates the SBOMs of all docker-images of dockerfiles/nix_based/. Every part of an image is
# described by the tool, which knows it best, and the parts are merged into one SBOM per image:
#
#   nix runtime-environment   sbomnix out of the flake (dockerfiles/nix_based/nix/packages.nix),
#                             with versions, licenses, patches, CPEs and purls
#   rust-binaries             cargo-cyclonedx out of Cargo.lock, for the target of the platform.
#                             The torii additionally contains the eBPF-programs of torii-ebpf.
#   operator                  syft out of the go build-info of the binary, which is built for this
#   dashboard                 syft out of src/dashboard/app/package-lock.json, without the
#                             dev-dependencies, because the image only contains the built bundle
#   docs                      only the runtime-environment, the site itself has no dependencies
#
# The parts are merged hierarchically with cyclonedx-cli, so every part stays a component with its
# own dependency-graph. None of the images has to be built before. Nix doesn't have to be installed,
# the script runs within the image of nix, which also pins all tools by the flake.
#
#   ./scripts/generate_sbom.sh [--platform linux/amd64|linux/arm64] [<IMAGE>...]
#
# Without images the SBOMs of all images are generated. For another platform than the one of the
# host, docker needs QEMU (see docs/developer/build_base_image.md).
#
# Result in temporary_files/sbom/<PLATFORM>/:
#   <IMAGE>.cdx.json    CycloneDX of the whole image
#   <IMAGE>.spdx.json   SPDX of the whole image, converted out of the CycloneDX
#   versions.csv        overview of all images: image,ecosystem,package,version
#   parts/              the SBOMs of the single tools, which are merged, and the CSV of sbomnix
#
# The nix-store of the container is kept in the docker-volume 'ainari-sbom-nix', the downloaded
# crates and go-modules in 'ainari-sbom-cache', so they are only downloaded once. They are removed
# with 'docker volume rm ainari-sbom-nix ainari-sbom-cache'.

set -euo pipefail

# the same image of nix like in the Dockerfiles
NIX_IMAGE="nixos/nix:2.35.2@sha256:7a007c766426c1877758ddc5cb87a965ac131fc78c582ce0083d922d51ae945c"
ALL_IMAGES="miko omamori izakaya omikuji isekai hanami ryokan onsen sakura torii dashboard docs operator"

PLATFORM="linux/$(uname -m | sed -e 's/x86_64/amd64/' -e 's/aarch64/arm64/')"
IMAGES=""
while [ $# -gt 0 ]; do
    case "$1" in
        --platform) PLATFORM="$2"; shift 2 ;;
        *) IMAGES="$IMAGES $1"; shift ;;
    esac
done
IMAGES="${IMAGES:-$ALL_IMAGES}"

case "$PLATFORM" in
    linux/amd64) RUST_TARGET="x86_64-unknown-linux-gnu" ;;
    linux/arm64) RUST_TARGET="aarch64-unknown-linux-gnu" ;;
    *) echo "ERROR: unsupported platform '$PLATFORM'" >&2; exit 1 ;;
esac

REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="$REPO_DIR/temporary_files/sbom/${PLATFORM//\//-}"
# the version of the workspace is the version of the images
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$REPO_DIR/Cargo.toml" | head -n 1)"
mkdir -p "$OUT_DIR"

docker run --rm -i --platform "$PLATFORM" \
    -v ainari-sbom-nix:/nix \
    -v ainari-sbom-cache:/cache \
    -v "$REPO_DIR:/repo:ro" \
    -v "$OUT_DIR:/out" \
    -e IMAGES="$IMAGES" \
    -e VERSION="$VERSION" \
    -e RUST_TARGET="$RUST_TARGET" \
    -e HOST_IDS="$(id -u):$(id -g)" \
    "$NIX_IMAGE" \
    bash -s <<'EOF'
set -euo pipefail
# filter-syscalls is not possible with the emulation of QEMU for another platform
export NIX_CONFIG="experimental-features = nix-command flakes
filter-syscalls = false"
export CARGO_HOME=/cache/cargo GOMODCACHE=/cache/go/mod GOCACHE=/cache/go/build
# like in dockerfiles/nix_based/Dockerfile_operator
export GOTOOLCHAIN=local GOFLAGS=-mod=readonly CGO_ENABLED=0
# syft only describes, what the image contains
export SYFT_JAVASCRIPT_INCLUDE_DEV_DEPENDENCIES=false SYFT_CHECK_FOR_APP_UPDATE=false
export DOTNET_CLI_TELEMETRY_OPTOUT=1

run() {
    nix develop path:/repo/dockerfiles/nix_based/nix#sbom --command "$@"
}

# cargo-cyclonedx writes its SBOMs next to the Cargo.toml of the crates, so it gets a copy of the
# workspace instead of the read-only repository
mkdir -p /work/workspace /out/parts
cp /repo/Cargo.toml /repo/Cargo.lock /work/workspace/
tar -C /repo --exclude=node_modules -cf - src | tar -C /work/workspace -xf -

# crate, out of which the rust-SBOM is generated, with its target
cargo_part() {
    local crate="$1" target="$2" out="$3"
    run cargo cyclonedx --manifest-path "/work/workspace/src/binaries/$crate/Cargo.toml" \
        --format json --spec-version 1.5 --target "$target" \
        --override-filename sbom -q
    mv "/work/workspace/src/binaries/$crate/sbom.json" "$out"
}

echo "image,ecosystem,package,version" > /out/versions.csv
for image in $IMAGES; do
    echo "=== $image"
    parts=""

    run sbomnix "path:/repo/dockerfiles/nix_based/nix#runtime-$image" \
        --cdx "/out/parts/$image.nix.cdx.json" \
        --spdx "/out/parts/$image.nix.spdx.json" \
        --csv "/out/parts/$image.nix.csv"
    parts="$parts /out/parts/$image.nix.cdx.json"

    case "$image" in
        torii)
            cargo_part torii "$RUST_TARGET" /out/parts/torii.cargo.cdx.json
            cargo_part torii-ebpf bpfel-unknown-none /out/parts/torii.cargo-ebpf.cdx.json
            parts="$parts /out/parts/torii.cargo.cdx.json /out/parts/torii.cargo-ebpf.cdx.json"
            ;;
        operator)
            (cd /repo/deploy/operator && \
                run go build -trimpath -buildvcs=false -ldflags="-s -w" -o /work/operator ./cmd)
            run syft scan file:/work/operator --source-name operator --source-version "$VERSION" \
                -q -o "cyclonedx-json@1.6=/out/parts/operator.syft.cdx.json"
            parts="$parts /out/parts/operator.syft.cdx.json"
            ;;
        dashboard)
            run syft scan file:/repo/src/dashboard/app/package-lock.json \
                --source-name dashboard --source-version "$VERSION" \
                -q -o "cyclonedx-json@1.6=/out/parts/dashboard.syft.cdx.json"
            parts="$parts /out/parts/dashboard.syft.cdx.json"
            ;;
        docs)
            ;;
        *)
            cargo_part "$image" "$RUST_TARGET" "/out/parts/$image.cargo.cdx.json"
            parts="$parts /out/parts/$image.cargo.cdx.json"
            ;;
    esac

    # shellcheck disable=SC2086
    run cyclonedx merge --hierarchical --name "$image" --version "$VERSION" \
        --input-files $parts --output-version v1_6 --output-file "/out/$image.cdx.json"
    # The conversion to SPDX only takes the top-level components, so it is done out of a flat
    # merge, which lists the components of all parts side by side.
    # shellcheck disable=SC2086
    run cyclonedx merge --input-files $parts --output-version v1_6 \
        --output-file "/work/$image.flat.cdx.json"
    run cyclonedx convert --input-file "/work/$image.flat.cdx.json" \
        --output-format spdxjson --output-file "/out/$image.spdx.json"

    # All packages of the parts with the type of their purl as ecosystem. The parts themselves and
    # the nix runtime-environment are not packages of the image.
    run jq -r --arg image "$image" '
        [.components[]?.components[]? | recurse(.components[]?)]
        | .[]
        | select((.version // "") != "" and (.name | startswith("ainari-runtime-") | not))
        | [$image, ((.purl // "") | capture("^pkg:(?<t>[^/]+)/").t // ""), .name, .version]
        | join(",")' "/out/$image.cdx.json" | sort -u >> /out/versions.csv
done

chown -R "$HOST_IDS" /out
EOF

echo "SBOMs written to $OUT_DIR"
