# Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>
#
# Licensed under the Apache License, Version 2.0 (the "License");
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

"""
Settings of the tested setup. The defaults match the local docker-compose setup and every one of
them can be overwritten by an environment-variable.
"""

import os
from dataclasses import dataclass, field

from . import REPO_DIR


def _env_int(name: str, default: int) -> int:
    return int(os.getenv(name, str(default)))


@dataclass
class Config:
    # the addresses and credentials of the setup
    miko_address: str = field(
        default_factory=lambda: os.getenv("AINARI_MIKO_ADDRESS", "http://127.0.0.1:11417"))
    user_id: str = field(default_factory=lambda: os.getenv("AINARI_USER", "asdf"))
    passphrase: str = field(default_factory=lambda: os.getenv("AINARI_PASSPHRASE", "asdfasdf"))

    # number of sakura-hosts of the setup. Every one of them registers itself in hanami, when it
    # starts, and a host, which is not registered yet, can not get a virtual machine.
    number_of_sakura_hosts: int = field(
        default_factory=lambda: _env_int("AINARI_SAKURA_HOSTS", 2))

    # the network of the virtual machines. The gateway of the virtual machines is the first
    # address of this subnet and is served by the torii, not by a real interface.
    network_subnet: str = field(
        default_factory=lambda: os.getenv("AINARI_NETWORK_SUBNET", "192.168.100.1/24"))

    # the virtual machines, which are created by the test. They share their image, their public
    # key and their network, so they only differ in the addresses, which hanami assigns to them.
    number_of_virtual_machines: int = field(
        default_factory=lambda: _env_int("AINARI_VIRTUAL_MACHINES", 2))
    number_of_cores: int = 2
    memory_size: int = 2 * 1024  # MiB
    disk_size: int = 10  # GiB

    # ubuntu-cloud-image, which becomes the boot-disk of the virtual machines
    image_url: str = field(default_factory=lambda: os.getenv(
        "AINARI_IMAGE_URL",
        "https://cloud-images.ubuntu.com/noble/current/noble-server-cloudimg-amd64.img"))
    # the cloud-image brings this user, the generated ssh-key is deployed for it
    vm_user: str = "ubuntu"

    # own directory, so the generated ssh-key doesn't replace the one of prepare_resources.py,
    # whose virtual machines are kept for manual tests
    work_dir: str = field(default_factory=lambda: os.getenv(
        "AINARI_TEST_WORK_DIR", os.path.join(REPO_DIR, "temporary_files", "ainari_test")))

    # Network-namespace, out of which the floating ip-addresses are reachable, if they are not
    # reachable from the host itself. This is the case for the single-node setup of VSCode, which
    # serves them only towards the namespace 'torii-outside'. The ssh-commands are then executed
    # with sudo within this namespace. Empty, if the host reaches the floating ip-addresses
    # directly.
    ssh_netns: str = field(default_factory=lambda: os.getenv("AINARI_SSH_NETNS", ""))

    # timeouts in seconds
    host_registration_timeout: int = field(
        default_factory=lambda: _env_int("AINARI_HOST_REGISTRATION_TIMEOUT", 300))
    vm_create_timeout: int = field(
        default_factory=lambda: _env_int("AINARI_VM_CREATE_TIMEOUT", 600))
    ssh_timeout: int = field(default_factory=lambda: _env_int("AINARI_SSH_TIMEOUT", 300))
    task_timeout: int = field(default_factory=lambda: _env_int("AINARI_TASK_TIMEOUT", 600))
    delete_timeout: int = field(default_factory=lambda: _env_int("AINARI_DELETE_TIMEOUT", 180))
    # a migration shuts the virtual machine down, transfers its disk and boots it again
    migration_timeout: int = field(
        default_factory=lambda: _env_int("AINARI_MIGRATION_TIMEOUT", 900))

    @property
    def image_path(self) -> str:
        return os.path.join(self.work_dir, os.path.basename(self.image_url))

    @property
    def ssh_key_path(self) -> str:
        return os.path.join(self.work_dir, "id_ed25519")
