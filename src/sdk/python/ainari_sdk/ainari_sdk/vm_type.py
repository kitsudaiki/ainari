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

from typing import Optional

from . import ainari_request
from .access_context import AccessContext


def create_vm_type(context: AccessContext,
                   name: str,
                   number_of_cores: int,
                   amount_of_memory: int) -> dict:
    """
    Creates a new vm-type, which defines the number of cores and the amount of memory in MiB of
    a virtual machine. Only admins are allowed to do this.
    """
    path = "/v1alpha/vm_type/admin"
    json_body = {
        "name": name,
        "number_of_cores": number_of_cores,
        "amount_of_memory": amount_of_memory,
    }
    return ainari_request.send_post_request(context,
                                            context.hanami_address,
                                            path,
                                            json_body)


def update_vm_type(context: AccessContext,
                   vm_type_uuid: str,
                   name: Optional[str] = None,
                   number_of_cores: Optional[int] = None,
                   amount_of_memory: Optional[int] = None) -> dict:
    """
    Updates the values of a vm-type. Only the values, which are not None, are changed. Only admins
    are allowed to do this.
    """
    path = f"/v1alpha/vm_type/{vm_type_uuid}/admin"
    json_body = {}
    if name is not None:
        json_body["name"] = name
    if number_of_cores is not None:
        json_body["number_of_cores"] = number_of_cores
    if amount_of_memory is not None:
        json_body["amount_of_memory"] = amount_of_memory
    return ainari_request.send_put_request(context,
                                           context.hanami_address,
                                           path,
                                           json_body)


def get_vm_type(context: AccessContext,
                vm_type_uuid: str) -> dict:
    path = f"/v1alpha/vm_type/{vm_type_uuid}"
    return ainari_request.send_get_request(context,
                                           context.hanami_address,
                                           path,
                                           "")


def list_vm_types(context: AccessContext) -> dict:
    path = "/v1alpha/vm_type"
    return ainari_request.send_get_request(context,
                                           context.hanami_address,
                                           path,
                                           "")


def delete_vm_type(context: AccessContext,
                   vm_type_uuid: str):
    """
    Deletes a vm-type. Only admins are allowed to do this.
    """
    path = f"/v1alpha/vm_type/{vm_type_uuid}/admin"
    ainari_request.send_delete_request(context,
                                       context.hanami_address,
                                       path,
                                       "")
