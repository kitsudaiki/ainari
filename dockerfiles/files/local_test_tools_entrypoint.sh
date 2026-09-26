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
# Entrypoint of the image of dockerfiles/Dockerfile_local_test_tools.
#
# The repository is mounted from the host, so everything, which the setups create in it, would
# belong to root. With HOST_UID and HOST_GID the command runs as a user with the ids of the user of
# the host instead. This user gets the groups of the sockets of docker and libvirt and of /dev/kvm,
# and sudo without a password, because the setup-scripts use sudo for the network-setup of the
# host.

set -e

HOST_UID="${HOST_UID:-0}"
HOST_GID="${HOST_GID:-0}"

if [ "$HOST_UID" == "0" ]; then
    exec "$@"
fi

# group and user with the ids of the host. An existing group or user with the same id is reused,
# because the groups below are given to the user with this id.
if ! getent group "$HOST_GID" > /dev/null; then
    groupadd --gid "$HOST_GID" tester
fi
USER_NAME="$(getent passwd "$HOST_UID" | cut -d: -f1)"
if [ -z "$USER_NAME" ]; then
    USER_NAME="tester"
    useradd --uid "$HOST_UID" --gid "$HOST_GID" --home-dir "/home/$USER_NAME" --create-home \
        --shell /bin/bash "$USER_NAME"
fi
USER_HOME="$(getent passwd "$HOST_UID" | cut -d: -f6)"
mkdir -p "$USER_HOME"

# access to the sockets and devices of the host, whose group-ids differ between hosts
for path in /var/run/docker.sock /var/run/libvirt/libvirt-sock /dev/kvm; do
    if [ -e "$path" ]; then
        gid="$(stat -c '%g' "$path")"
        if [ "$gid" != "0" ]; then
            if ! getent group "$gid" > /dev/null; then
                groupadd --gid "$gid" "host-$(basename "$path" | tr -c 'a-z0-9\n' '-')"
            fi
            usermod --append --groups "$gid" "$USER_NAME"
        fi
    fi
done

echo "$USER_NAME ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/host-user
chmod 0440 /etc/sudoers.d/host-user

# the plugins of vagrant are installed in the image, the boxes are downloaded into the same place
chown -R "$HOST_UID:$HOST_GID" "$VAGRANT_HOME" "$USER_HOME"

exec setpriv --reuid "$HOST_UID" --regid "$HOST_GID" --init-groups \
    env HOME="$USER_HOME" USER="$USER_NAME" "$@"
