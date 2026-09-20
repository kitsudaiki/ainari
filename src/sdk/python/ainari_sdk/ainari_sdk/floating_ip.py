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


def create_floating_ip(context: AccessContext,
                       name: str,
                       network_uuid: str,
                       internal_ip: str,
                       floating_ip: str = "") -> dict:
    """
    Assigns a floating ip-address to an internal address. If no floating ip-address is requested,
    a free one is selected by the server.
    """
    path = "/v1alpha/floating_ip"
    json_body = {
        "name": name,
        "network_uuid": network_uuid,
        "internal_ip": internal_ip,
    }
    if floating_ip:
        json_body["floating_ip"] = floating_ip

    return ainari_request.send_post_request(context,
                                            context.hanami_address,
                                            path,
                                            json_body)


def get_floating_ip(context: AccessContext,
                    floating_ip_uuid: str) -> dict:
    path = f"/v1alpha/floating_ip/{floating_ip_uuid}"
    return ainari_request.send_get_request(context,
                                           context.hanami_address,
                                           path,
                                           "")


def list_floating_ips(context: AccessContext) -> dict:
    path = "/v1alpha/floating_ip"
    return ainari_request.send_get_request(context,
                                           context.hanami_address,
                                           path,
                                           "")


def delete_floating_ip(context: AccessContext,
                       floating_ip_uuid: str):
    path = f"/v1alpha/floating_ip/{floating_ip_uuid}"
    ainari_request.send_delete_request(context,
                                       context.hanami_address,
                                       path,
                                       "")


def delete_all_floating_ips(context: AccessContext):
    body = list_floating_ips(context)["floating_ips"]
    for entry in body:
        delete_floating_ip(context, entry["uuid"])
