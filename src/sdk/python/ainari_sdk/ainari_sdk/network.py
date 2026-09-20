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


def create_network(context: AccessContext,
                   name: str,
                   subnet: str) -> dict:
    """
    Creates a new network with the given name for the given subnet in CIDR-notation.
    """
    path = "/v1alpha/network"
    json_body = {
        "name": name,
        "subnet": subnet,
    }
    return ainari_request.send_post_request(context,
                                            context.hanami_address,
                                            path,
                                            json_body)


def get_network(context: AccessContext,
                network_uuid: str) -> dict:
    path = f"/v1alpha/network/{network_uuid}"
    return ainari_request.send_get_request(context,
                                           context.hanami_address,
                                           path,
                                           "")


def list_networks(context: AccessContext) -> dict:
    path = "/v1alpha/network"
    return ainari_request.send_get_request(context,
                                           context.hanami_address,
                                           path,
                                           "")


def delete_network(context: AccessContext,
                   network_uuid: str):
    path = f"/v1alpha/network/{network_uuid}"
    ainari_request.send_delete_request(context,
                                       context.hanami_address,
                                       path,
                                       "")


def delete_all_networks(context: AccessContext):
    body = list_networks(context)["networks"]
    for entry in body:
        delete_network(context, entry["uuid"])
