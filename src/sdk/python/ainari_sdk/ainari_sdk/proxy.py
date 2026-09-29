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


# The proxies are stored in the torii at the edge, whose api is only reachable within the
# cluster, so they are read over the hanami. They are created and deleted by the hanami together
# with the virtual machines.


def get_proxy(context: AccessContext,
              proxy_uuid: str) -> dict:
    path = f"/v1alpha/proxy/{proxy_uuid}"
    return ainari_request.send_get_request(context,
                                           context.hanami_address,
                                           path,
                                           "")


def list_proxys(context: AccessContext) -> dict:
    path = "/v1alpha/proxy"
    return ainari_request.send_get_request(context,
                                           context.hanami_address,
                                           path,
                                           "")
