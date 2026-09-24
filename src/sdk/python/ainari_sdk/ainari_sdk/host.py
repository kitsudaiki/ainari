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


def get_host(context: AccessContext,
             host_uuid: str) -> dict:
    """
    Returns information of a sakura-host, which runs the virtual machines.
    """
    path = f"/v1alpha/host/{host_uuid}/admin"
    return ainari_request.send_get_request(context,
                                           context.hanami_address,
                                           path,
                                           "")


def list_hosts(context: AccessContext) -> dict:
    path = "/v1alpha/host/admin"
    return ainari_request.send_get_request(context,
                                           context.hanami_address,
                                           path,
                                           "")


def delete_host(context: AccessContext,
                host_uuid: str):
    path = f"/v1alpha/host/{host_uuid}/admin"
    ainari_request.send_delete_request(context,
                                       context.hanami_address,
                                       path,
                                       "")


def delete_all_hosts(context: AccessContext):
    body = list_hosts(context)["hosts"]
    for entry in body:
        delete_host(context, entry["uuid"])


def get_onsen_host(context: AccessContext,
                   host_uuid: str) -> dict:
    """
    Returns information of an onsen-host, which stores the images and snapshots.
    """
    path = f"/v1alpha/host/{host_uuid}/admin"
    return ainari_request.send_get_request(context,
                                           context.ryokan_adress,
                                           path,
                                           "")


def list_onsen_hosts(context: AccessContext) -> dict:
    path = "/v1alpha/host/admin"
    return ainari_request.send_get_request(context,
                                           context.ryokan_adress,
                                           path,
                                           "")


def delete_onsen_host(context: AccessContext,
                      host_uuid: str):
    path = f"/v1alpha/host/{host_uuid}/admin"
    ainari_request.send_delete_request(context,
                                       context.ryokan_adress,
                                       path,
                                       "")


def delete_all_onsen_hosts(context: AccessContext):
    body = list_onsen_hosts(context)["hosts"]
    for entry in body:
        delete_onsen_host(context, entry["uuid"])
