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


def _network_filter_path(virtual_machine_uuid: str, direction: str) -> str:
    """
    Returns the path of the packet filter of one direction ("ingress" or "egress") of a virtual
    machine.
    """
    return f"/v1alpha/network_filter/{virtual_machine_uuid}/{direction}"


def add_network_filter_ip_ranges(context: AccessContext,
                                 virtual_machine_uuid: str,
                                 direction: str,
                                 ranges: list) -> dict:
    """
    Adds ip-ranges to the packet filter of one direction of a virtual machine. Each range is a
    single address, a subnet in CIDR notation or an explicit range "first-last".
    """
    path = _network_filter_path(virtual_machine_uuid, direction) + "/ip_range"
    return ainari_request.send_post_request(context,
                                            context.hanami_address,
                                            path,
                                            {"ranges": ranges})


def delete_network_filter_ip_ranges(context: AccessContext,
                                    virtual_machine_uuid: str,
                                    direction: str,
                                    ranges: list) -> dict:
    """
    Removes ip-ranges from the packet filter of one direction of a virtual machine.
    """
    path = _network_filter_path(virtual_machine_uuid, direction) + "/ip_range"
    return ainari_request.send_delete_request_with_body(context,
                                                        context.hanami_address,
                                                        path,
                                                        {"ranges": ranges})


def add_network_filter_ports(context: AccessContext,
                             virtual_machine_uuid: str,
                             direction: str,
                             ports: list) -> dict:
    """
    Adds ports to the packet filter of one direction of a virtual machine. Each entry is a single
    port or an explicit range "first-last".
    """
    path = _network_filter_path(virtual_machine_uuid, direction) + "/port"
    return ainari_request.send_post_request(context,
                                            context.hanami_address,
                                            path,
                                            {"ports": ports})


def delete_network_filter_ports(context: AccessContext,
                                virtual_machine_uuid: str,
                                direction: str,
                                ports: list) -> dict:
    """
    Removes ports from the packet filter of one direction of a virtual machine.
    """
    path = _network_filter_path(virtual_machine_uuid, direction) + "/port"
    return ainari_request.send_delete_request_with_body(context,
                                                        context.hanami_address,
                                                        path,
                                                        {"ports": ports})


def get_network_filter(context: AccessContext,
                       virtual_machine_uuid: str,
                       direction: str) -> dict:
    path = _network_filter_path(virtual_machine_uuid, direction)
    return ainari_request.send_get_request(context,
                                           context.hanami_address,
                                           path,
                                           "")


def list_network_filters(context: AccessContext) -> dict:
    path = "/v1alpha/network_filter"
    return ainari_request.send_get_request(context,
                                           context.hanami_address,
                                           path,
                                           "")


def delete_network_filter(context: AccessContext,
                          virtual_machine_uuid: str,
                          direction: str):
    """
    Removes the whole packet filter of one direction of a virtual machine.
    """
    path = _network_filter_path(virtual_machine_uuid, direction)
    ainari_request.send_delete_request(context,
                                       context.hanami_address,
                                       path,
                                       "")


def delete_all_network_filters(context: AccessContext):
    body = list_network_filters(context)["network_filters"]
    for entry in body:
        delete_network_filter(context, entry["virtual_machine_uuid"], entry["direction"])
