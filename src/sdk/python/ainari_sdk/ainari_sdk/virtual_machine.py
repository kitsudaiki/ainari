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

from . import ainari_request
from .access_context import AccessContext


def reserve_virtual_machine(context: AccessContext,
                            name: str,
                            number_of_cores: int,
                            memory_size: int,
                            network_uuid: str) -> dict:
    """
    Reserves a new virtual machine on one of the sakura-hosts. The image and the public-key are
    not deployed here, but by the task of create_virtual_machine.
    """
    path = "/v1alpha/virtual_machine"
    json_body = {
        "name": name,
        "number_of_cores": number_of_cores,
        "memory_size": memory_size,
        "network_uuid": network_uuid,
    }
    return ainari_request.send_post_request(context,
                                            context.hanami_address,
                                            path,
                                            json_body)


def create_virtual_machine(context: AccessContext,
                           torii_port: int,
                           virtual_machine_uuid: str,
                           image_uuid: str,
                           public_key_uuid: str) -> dict:
    """
    Creates a task on the sakura-host of a reserved virtual machine, which installs the image and
    the public-key in the virtual machine and boots it.
    """
    address = f"{context.torii_base_address}:{torii_port}"
    path = f"/v1alpha/virtual_machine/{virtual_machine_uuid}"
    json_body = {
        "vm_uuid": virtual_machine_uuid,
        "image_uuid": image_uuid,
        "public_key_uuid": public_key_uuid,
    }
    return ainari_request.send_post_request(context,
                                            address,
                                            path,
                                            json_body)


def get_virtual_machine(context: AccessContext,
                        virtual_machine_uuid: str) -> dict:
    path = f"/v1alpha/virtual_machine/{virtual_machine_uuid}"
    return ainari_request.send_get_request(context,
                                           context.hanami_address,
                                           path,
                                           "")


def list_virtual_machines(context: AccessContext) -> dict:
    path = "/v1alpha/virtual_machine"
    return ainari_request.send_get_request(context,
                                           context.hanami_address,
                                           path,
                                           "")


def get_virtual_machine_count(context: AccessContext) -> dict:
    path = "/v1alpha/virtual_machine/count"
    return ainari_request.send_get_request(context,
                                           context.hanami_address,
                                           path,
                                           "")


def delete_virtual_machine(context: AccessContext,
                           virtual_machine_uuid: str):
    path = f"/v1alpha/virtual_machine/{virtual_machine_uuid}"
    ainari_request.send_delete_request(context,
                                       context.hanami_address,
                                       path,
                                       "")


def delete_all_virtual_machines(context: AccessContext):
    body = list_virtual_machines(context)["virtual_machines"]
    for entry in body:
        delete_virtual_machine(context, entry["uuid"])
