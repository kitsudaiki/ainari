#!/bin/sh

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

# Builds the root-filesystem of an image out of a runtime-environment of dockerfiles/nix_based/nix,
# which is copied into an image, which is built 'FROM scratch'. So the image contains nothing else
# than the packages of the environment and the files, which the Dockerfile adds to the
# root-filesystem.
#
#   make_rootfs.sh <ROOTFS> <RUNTIME> [<USER>]
#
# ROOTFS   directory of the root-filesystem. Files, which the Dockerfile puts there before, like
#          the compiled binaries, are kept.
# RUNTIME  name of the runtime-environment within dockerfiles/nix_based/nix/packages.nix, like
#          'runtime-miko'
# USER     optional user with the id 1000, who gets a home-directory
#
# The compiled binaries are linked against libraries of the nix-store. If a file of the rootfs
# references a path of the store, which is not part of the runtime-environment, the build fails,
# because the image would miss this path. Hashes, which are replaced by remove-references-to
# (eeee...), are no valid hashes of the store and so are not matched.

set -eu

ROOTFS="$1"
RUNTIME="$2"
USER_NAME="${3:-}"

FLAKE_DIR="$(dirname "$(readlink -f "$0")")"

ENV_PATH="$(nix build --no-link --print-out-paths "path:$FLAKE_DIR#$RUNTIME")"
CLOSURE="$(nix-store --query --requisites "$ENV_PATH")"

mkdir -p "$ROOTFS"

# check the references of the files, which the Dockerfile has added
REFERENCES="$(find "$ROOTFS" -type f -exec grep -aohE '/nix/store/[0-9a-df-np-sv-z]{32}-[a-zA-Z0-9+._?=-]+' {} + \
                  | sort -u)" || true
MISSING=""
for reference in $REFERENCES; do
    if ! printf '%s\n' "$CLOSURE" | grep -qxF "$reference"; then
        MISSING="$MISSING $reference"
    fi
done
if [ -n "$MISSING" ]; then
    echo "ERROR: files of the image reference paths, which are not part of '$RUNTIME':" >&2
    printf '    %s\n' $MISSING >&2
    exit 1
fi

# the whole closure of the runtime-environment
mkdir -p "$ROOTFS/nix/store"
for path in $CLOSURE; do
    cp -a "$path" "$ROOTFS/nix/store/"
done

# merged /usr, the executables of the environment are linked into /usr/bin
mkdir -p "$ROOTFS/usr/bin" "$ROOTFS/usr/local/bin" "$ROOTFS/usr/local/share"
ln -sfn usr/bin "$ROOTFS/bin"
ln -sfn usr/bin "$ROOTFS/sbin"
ln -sfn bin "$ROOTFS/usr/sbin"
for executable in "$ENV_PATH"/bin/*; do
    ln -sfn "$executable" "$ROOTFS/usr/bin/$(basename "$executable")"
done

# configs of the environment, like the certificates in /etc/ssl
mkdir -p "$ROOTFS/etc"
if [ -d "$ENV_PATH/etc" ]; then
    cp -a "$ENV_PATH/etc/." "$ROOTFS/etc/"
fi

# A setuid-bit is not possible within the nix-store, so sudo is copied out of it
if [ -e "$ENV_PATH/bin/sudo" ]; then
    rm "$ROOTFS/usr/bin/sudo"
    cp "$(readlink -f "$ENV_PATH/bin/sudo")" "$ROOTFS/usr/bin/sudo"
    chmod 4755 "$ROOTFS/usr/bin/sudo"
    # the sample-configs of sudo are replaced by an empty config, which only includes the rules
    # of the Dockerfile in /etc/sudoers.d
    rm -rf "$ROOTFS/etc/sudoers" "$ROOTFS/etc/sudoers.d"
    mkdir -p "$ROOTFS/etc/sudoers.d"
    echo "@includedir /etc/sudoers.d" > "$ROOTFS/etc/sudoers"
    chmod 0440 "$ROOTFS/etc/sudoers"
fi

mkdir -p "$ROOTFS/tmp" "$ROOTFS/var/tmp" "$ROOTFS/run" "$ROOTFS/root" "$ROOTFS/home"
chmod 1777 "$ROOTFS/tmp" "$ROOTFS/var/tmp"
chmod 0700 "$ROOTFS/root"

cat > "$ROOTFS/etc/passwd" <<EOF
root:x:0:0:root:/root:/bin/bash
nobody:x:65534:65534:nobody:/nonexistent:/bin/false
EOF
cat > "$ROOTFS/etc/group" <<EOF
root:x:0:
nogroup:x:65534:
EOF
cat > "$ROOTFS/etc/shadow" <<EOF
root:!:1::::::
nobody:!:1::::::
EOF
chmod 0640 "$ROOTFS/etc/shadow"
cat > "$ROOTFS/etc/nsswitch.conf" <<EOF
passwd: files
group: files
shadow: files
hosts: files dns
networks: files
protocols: files
services: files
EOF

if [ -n "$USER_NAME" ]; then
    echo "$USER_NAME:x:1000:1000::/home/$USER_NAME:/bin/bash" >> "$ROOTFS/etc/passwd"
    echo "$USER_NAME:x:1000:" >> "$ROOTFS/etc/group"
    echo "$USER_NAME:!:1::::::" >> "$ROOTFS/etc/shadow"
    mkdir -p "$ROOTFS/home/$USER_NAME"
    chown 1000:1000 "$ROOTFS/home/$USER_NAME"
fi

# list of all packages of the image
printf '%s\n' "$CLOSURE" | cut -d- -f2- | sort > "$ROOTFS/etc/nix-packages.txt"
