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
from . import ainari_exceptions
from .access_context import AccessContext

# import base64


def create_secret(context: AccessContext,
                  secret_name: str,
                  secret_payload: str) -> dict:
    path = "/v1alpha/secret"
    json_body = {
        "name": secret_name,
        "secret_payload": secret_payload,
    }
    return ainari_request.send_post_request(context,
                                            context.omamori_address,
                                            path,
                                            json_body)


def get_secret(context: AccessContext,
               secret_uuid: str) -> dict:
    path = f'/v1alpha/secret/{secret_uuid}'
    return ainari_request.send_get_request(context,
                                           context.omamori_address,
                                           path,
                                           "")


def get_secret_payload(context: AccessContext,
                       secret_uuid: str) -> dict:
    path = f'/v1alpha/secret/{secret_uuid}/payload'
    return ainari_request.send_get_request(context,
                                           context.omamori_address,
                                           path,
                                           "")


def list_secrets(context: AccessContext) -> dict:
    path = "/v1alpha/secret"
    return ainari_request.send_get_request(context,
                                           context.omamori_address,
                                           path,
                                           "")


def delete_secret(context: AccessContext,
                  secret_uuid: str):
    path = f'/v1alpha/secret/{secret_uuid}'
    ainari_request.send_delete_request(context,
                                       context.omamori_address,
                                       path,
                                       "")


def delete_all_secrets(context: AccessContext):
    body = list_secrets(context)["secrets"]
    for entry in body:
        try:
            delete_secret(context, entry["id"])
        except ainari_exceptions.ConflictException:
            # when a secret tries to delete himself, then an exception
            # is raised, which is catched here.
            pass
