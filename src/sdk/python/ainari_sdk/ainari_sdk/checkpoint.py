# Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>
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


def list_checkpoints(context: AccessContext) -> dict:
    path = "/v1alpha/checkpoint"
    return ainari_request.send_get_request(context,
                                           context.ryokan_adress,
                                           path,
                                           "")


def delete_checkpoint(context: AccessContext,
                      checkpoint_uuid: str):
    path = f"/v1alpha/checkpoint/{checkpoint_uuid}"
    ainari_request.send_delete_request(context,
                                       context.ryokan_adress,
                                       path,
                                       "")


def delete_all_checkpoints(context: AccessContext):
    body = list_checkpoints(context)["checkpoints"]
    for entry in body:
        delete_checkpoint(context, entry["uuid"])
