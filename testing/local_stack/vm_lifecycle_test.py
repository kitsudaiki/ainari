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
End-to-end test of the local docker-compose setup.

It walks through the whole life-cycle of two virtual machines with the python-sdk:

    0. wait until all sakura-hosts of the setup are registered in hanami
    1. generate a ssh-key-pair and upload the public key to omamori
    2. download an ubuntu-cloud-image and upload it to ryokan
    3. create a network
    4. reserve the virtual machines in hanami
    5. create the reserved virtual machines on their sakura-hosts
    6. give every virtual machine its own floating ip-address
    7. log into every virtual machine over ssh with the generated key

Both virtual machines use the same image, the same public key and the same network and differ
only in their addresses. Hanami picks a sakura-host for every one of them, so they can land on
the same host or on different ones, which differs from run to run.

The stack has to run before this script is started:

    sudo ./scripts/setup_local_stack.sh
"""

import os
import subprocess
import sys
import time
import urllib.request
import uuid

# the sdk is used from the source-tree, so the test always runs against the current version
REPO_DIR = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.join(REPO_DIR, "src", "sdk", "python", "ainari_sdk"))

from ainari_sdk import floating_ip   # noqa: E402
from ainari_sdk import host          # noqa: E402
from ainari_sdk import image         # noqa: E402
from ainari_sdk import login         # noqa: E402
from ainari_sdk import network       # noqa: E402
from ainari_sdk import public_key    # noqa: E402
from ainari_sdk import virtual_machine  # noqa: E402

# the addresses and credentials of the local docker-compose setup
MIKO_ADDRESS = os.getenv("AINARI_MIKO_ADDRESS", "http://127.0.0.1:11417")
USER_ID = os.getenv("AINARI_USER", "asdf")
PASSPHRASE = os.getenv("AINARI_PASSPHRASE", "asdfasdf")

# the network of the virtual machine. The gateway of the virtual machines is the first address of
# this subnet and is served by the torii, not by a real interface.
NETWORK_SUBNET = "192.168.100.1/24"

# the virtual machines, which are created by the test. They share their image, their public key
# and their network, so they only differ in the addresses, which hanami assigns to them.
NUMBER_OF_VIRTUAL_MACHINES = int(os.getenv("AINARI_VIRTUAL_MACHINES", "2"))

NUMBER_OF_CORES = 2
MEMORY_SIZE = 2 * 1024  # MiB
DISK_SIZE = 10  # GiB

# ubuntu-cloud-image, which becomes the boot-disk of the virtual machine
IMAGE_URL = "https://cloud-images.ubuntu.com/noble/current/noble-server-cloudimg-amd64.img"
# the cloud-image brings this user, the generated ssh-key is deployed for it
VM_USER = "ubuntu"

WORK_DIR = os.path.join(REPO_DIR, "temporary_files", "local_stack_test")
IMAGE_PATH = os.path.join(WORK_DIR, "noble-server-cloudimg-amd64.img")
SSH_KEY_PATH = os.path.join(WORK_DIR, "id_ed25519")

# number of sakura-hosts of the docker-compose setup. Every one of them registers itself in
# hanami, when it starts, and a host, which is not registered yet, can not get a virtual machine.
NUMBER_OF_SAKURA_HOSTS = int(os.getenv("AINARI_SAKURA_HOSTS", "2"))
HOST_REGISTRATION_TIMEOUT = 300

# a virtual machine needs a while to boot, before it answers on ssh
VM_CREATE_TIMEOUT = 600
SSH_TIMEOUT = 300


def log(message: str):
    print(f"[test] {message}", flush=True)


def generate_ssh_key() -> str:
    """
    Generates a fresh ssh-key-pair and returns the public key in its one-line representation.
    """
    if os.path.exists(SSH_KEY_PATH):
        os.remove(SSH_KEY_PATH)
    if os.path.exists(f"{SSH_KEY_PATH}.pub"):
        os.remove(f"{SSH_KEY_PATH}.pub")

    subprocess.run(["ssh-keygen", "-t", "ed25519", "-N", "", "-C", "ainari-local-stack-test",
                    "-f", SSH_KEY_PATH],
                   check=True,
                   stdout=subprocess.DEVNULL)

    with open(f"{SSH_KEY_PATH}.pub", "r", encoding="utf-8") as file_handle:
        return file_handle.read().strip()


def download_cloud_image():
    """
    Downloads the ubuntu-cloud-image, if it is not already in the working-directory.
    """
    if os.path.exists(IMAGE_PATH) and os.path.getsize(IMAGE_PATH) > 0:
        log(f"cloud-image already downloaded: {IMAGE_PATH}")
        return

    log(f"downloading {IMAGE_URL} ...")
    urllib.request.urlretrieve(IMAGE_URL, IMAGE_PATH)
    log(f"downloaded {os.path.getsize(IMAGE_PATH)} bytes")


def wait_for_sakura_hosts(context) -> list:
    """
    Waits until all sakura-hosts of the setup are registered in hanami and returns them.

    A sakura, which is started before hanami, fails its registration and is restarted by docker,
    so the hosts are not there immediately after the stack is up.
    """
    end_time = time.time() + HOST_REGISTRATION_TIMEOUT
    hosts = []
    while time.time() < end_time:
        hosts = host.list_hosts(context)["hosts"]
        if len(hosts) >= NUMBER_OF_SAKURA_HOSTS:
            return hosts
        time.sleep(2.0)

    raise TimeoutError(f"only {len(hosts)} of {NUMBER_OF_SAKURA_HOSTS} sakura-hosts registered "
                       f"themselves within {HOST_REGISTRATION_TIMEOUT}s")


def reserve_and_create_virtual_machine(context,
                                       name: str,
                                       network_uuid: str,
                                       image_uuid: str,
                                       public_key_uuid: str) -> dict:
    """
    Reserves a virtual machine in hanami and starts its creation on the sakura-host, which hanami
    selected for it. The virtual machine is not booted yet, when this returns.
    """
    reserved = virtual_machine.reserve_virtual_machine(context,
                                                       name,
                                                       NUMBER_OF_CORES,
                                                       MEMORY_SIZE,
                                                       DISK_SIZE,
                                                       network_uuid)
    virtual_machine_data = {
        "name": name,
        "uuid": reserved["uuid"],
        "internal_ip": reserved["internal_ip"],
        "torii_port": reserved["torii_port"],
    }
    log(f"    {name}: {virtual_machine_data['uuid']} "
        f"({virtual_machine_data['internal_ip']}, proxy-port {virtual_machine_data['torii_port']})")

    virtual_machine.create_virtual_machine(context,
                                           virtual_machine_data["torii_port"],
                                           virtual_machine_data["uuid"],
                                           image_uuid,
                                           public_key_uuid)

    return virtual_machine_data


def wait_for_created_virtual_machine(context, virtual_machine_uuid: str) -> dict:
    """
    Waits until the sakura-host reports the virtual machine as running and returns its data.
    """
    end_time = time.time() + VM_CREATE_TIMEOUT
    while time.time() < end_time:
        virtual_machine_data = virtual_machine.get_virtual_machine(context, virtual_machine_uuid)
        vm_state = virtual_machine_data.get("vm_state")
        if vm_state == "RUNNING":
            return virtual_machine_data
        if vm_state == "ERROR":
            raise RuntimeError(f"creation of virtual machine '{virtual_machine_uuid}' failed")
        time.sleep(2.0)

    raise TimeoutError(
        f"virtual machine '{virtual_machine_uuid}' was not created within {VM_CREATE_TIMEOUT}s")


def run_over_ssh(floating_ip_address: str, command: str) -> str:
    """
    Runs a command in the virtual machine over ssh and returns its output.
    """
    ssh_command = [
        "ssh",
        "-i", SSH_KEY_PATH,
        "-o", "StrictHostKeyChecking=no",
        "-o", "UserKnownHostsFile=/dev/null",
        "-o", "ConnectTimeout=5",
        "-o", "LogLevel=ERROR",
        f"{VM_USER}@{floating_ip_address}",
        command,
    ]
    result = subprocess.run(ssh_command, capture_output=True, text=True, check=True)

    return result.stdout.strip()


def wait_for_ssh(floating_ip_address: str) -> str:
    """
    Waits until the virtual machine answers on ssh and returns the output of the first command.
    """
    end_time = time.time() + SSH_TIMEOUT
    last_error = ""
    while time.time() < end_time:
        try:
            return run_over_ssh(floating_ip_address, "hostname && uptime")
        except subprocess.CalledProcessError as error:
            last_error = error.stderr.strip()
        time.sleep(5.0)

    raise TimeoutError(f"no ssh-access to '{floating_ip_address}' "
                       f"within {SSH_TIMEOUT}s: {last_error}")


def main() -> int:
    os.makedirs(WORK_DIR, exist_ok=True)
    test_id = str(uuid.uuid4())[:8]

    log(f"login as '{USER_ID}' at {MIKO_ADDRESS}")
    context = login.request_context(MIKO_ADDRESS,
                                    USER_ID,
                                    PASSPHRASE,
                                    verify_connection=False)

    # 0. sakura-hosts
    log(f"waiting for the {NUMBER_OF_SAKURA_HOSTS} sakura-host(s) of the setup")
    for host_data in wait_for_sakura_hosts(context):
        log(f"    sakura-host '{host_data['name']}' at {host_data['host_address']}")

    # 1. ssh-key-pair
    log("generating ssh-key-pair and uploading the public key to omamori")
    public_key_content = generate_ssh_key()
    public_key_data = public_key.upload_public_key(context,
                                                   f"local-stack-test-{test_id}",
                                                   public_key_content)
    public_key_uuid = public_key_data["uuid"]
    log(f"public-key: {public_key_uuid}")

    # 2. cloud-image
    download_cloud_image()
    log("uploading the cloud-image to ryokan (this takes a while) ...")
    image_data = image.upload_disk_file(context, f"ubuntu-noble-{test_id}", IMAGE_PATH)
    image_uuid = image_data["uuid"]
    log(f"image: {image_uuid}")

    # 3. network
    log(f"creating network {NETWORK_SUBNET}")
    network_data = network.create_network(context, f"local-stack-test-{test_id}", NETWORK_SUBNET)
    network_uuid = network_data["uuid"]
    log(f"network: {network_uuid}")

    # 4. + 5. reserve the virtual machines and create them on their sakura-hosts
    log(f"reserving {NUMBER_OF_VIRTUAL_MACHINES} virtual machines with {NUMBER_OF_CORES} cores "
        f"and {MEMORY_SIZE} MiB memory each")
    virtual_machines = []
    for number in range(1, NUMBER_OF_VIRTUAL_MACHINES + 1):
        virtual_machines.append(
            reserve_and_create_virtual_machine(context,
                                               f"local-stack-test-{test_id}-{number}",
                                               network_uuid,
                                               image_uuid,
                                               public_key_uuid))

    # the virtual machines are installed and booted in parallel, so they are awaited together
    log("waiting until the virtual machines are created on their sakura-hosts ...")
    for virtual_machine_entry in virtual_machines:
        wait_for_created_virtual_machine(context, virtual_machine_entry["uuid"])
        log(f"    {virtual_machine_entry['name']} is created")

    # 6. floating ip-addresses
    log("creating a floating ip-address for every virtual machine")
    for virtual_machine_entry in virtual_machines:
        floating_ip_data = floating_ip.create_floating_ip(context,
                                                          f"{virtual_machine_entry['name']}-fip",
                                                          network_uuid,
                                                          virtual_machine_entry["internal_ip"])
        virtual_machine_entry["floating_ip"] = floating_ip_data["floating_ip"]
        log(f"    {virtual_machine_entry['floating_ip']} "
            f"-> {virtual_machine_entry['internal_ip']}")

    # 7. ssh
    for virtual_machine_entry in virtual_machines:
        log(f"waiting for ssh on {virtual_machine_entry['floating_ip']} ...")
        output = wait_for_ssh(virtual_machine_entry["floating_ip"])
        log(f"ssh-access to {virtual_machine_entry['name']} works:")
        for line in output.splitlines():
            log(f"    {line}")

    log("")
    log("SUCCESS")
    for virtual_machine_entry in virtual_machines:
        log(f"    ssh -i {SSH_KEY_PATH} {VM_USER}@{virtual_machine_entry['floating_ip']}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
