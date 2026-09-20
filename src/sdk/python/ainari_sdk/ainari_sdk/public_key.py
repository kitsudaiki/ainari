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


def upload_public_key(context: AccessContext,
                      name: str,
                      public_key: str) -> dict:
    """
    Uploads a ssh-public-key in its one-line openssh-representation. The fingerprint is calculated
    by the server.
    """
    path = "/v1alpha/public_key"
    json_body = {
        "name": name,
        "public_key": public_key,
    }
    return ainari_request.send_post_request(context,
                                            context.omamori_address,
                                            path,
                                            json_body)


def get_public_key(context: AccessContext,
                   public_key_uuid: str) -> dict:
    path = f"/v1alpha/public_key/{public_key_uuid}"
    return ainari_request.send_get_request(context,
                                           context.omamori_address,
                                           path,
                                           "")


def list_public_keys(context: AccessContext) -> dict:
    path = "/v1alpha/public_key"
    return ainari_request.send_get_request(context,
                                           context.omamori_address,
                                           path,
                                           "")


def delete_public_key(context: AccessContext,
                      public_key_uuid: str):
    path = f"/v1alpha/public_key/{public_key_uuid}"
    ainari_request.send_delete_request(context,
                                       context.omamori_address,
                                       path,
                                       "")


def delete_all_public_keys(context: AccessContext):
    body = list_public_keys(context)["public_keys"]
    for entry in body:
        delete_public_key(context, entry["uuid"])
