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


def get_quota(context: AccessContext,
              user_id: str) -> dict:
    path = f'/v1alpha/quota/{user_id}/admin'
    return ainari_request.send_get_request(context,
                                           context.miko_address,
                                           path,
                                           "")


def list_quotas(context: AccessContext) -> dict:
    path = "/v1alpha/quota/admin"
    return ainari_request.send_get_request(context,
                                           context.miko_address,
                                           path,
                                           "")


def set_quota(context: AccessContext,
              user_id: str,
              max_cluster: int,
              max_dataset: int,
              max_checkpoint: int,
              max_secret: int,
              max_taskqueue: int) -> dict:
    path = f"/v1alpha/quota/{user_id}/admin"
    json_body = {
        "max_cluster": max_cluster,
        "max_dataset": max_dataset,
        "max_checkpoint": max_checkpoint,
        "max_secret": max_secret,
        "max_taskqueue": max_taskqueue,
    }
    return ainari_request.send_put_request(context,
                                           context.miko_address,
                                           path,
                                           json_body)
