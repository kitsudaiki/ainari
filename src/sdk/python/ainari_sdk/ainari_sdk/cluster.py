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


def create_cluster(context: AccessContext,
                   name: str,
                   template: str) -> dict:
    path = "/v1alpha/cluster"
    json_body = {
        "name": name,
        "template": template,
    }
    return ainari_request.send_post_request(context,
                                            context.hanami_address,
                                            path,
                                            json_body)


def get_cluster(context: AccessContext,
                cluster_uuid: str) -> dict:
    path = f"/v1alpha/cluster/{cluster_uuid}"
    return ainari_request.send_get_request(context,
                                           context.hanami_address,
                                           path,
                                           "")


def list_clusters(context: AccessContext) -> dict:
    path = "/v1alpha/cluster"
    return ainari_request.send_get_request(context,
                                           context.hanami_address,
                                           path,
                                           "")


def delete_cluster(context: AccessContext,
                   cluster_uuid: str):
    path = f"/v1alpha/cluster/{cluster_uuid}"
    ainari_request.send_delete_request(context,
                                       context.hanami_address,
                                       path,
                                       "")


def delete_all_cluster(context: AccessContext):
    body = list_clusters(context)["clusters"]
    for entry in body:
        delete_cluster(context, entry["uuid"])


def train(context: AccessContext,
          torii_port: int,
          cluster_uuid: str,
          inputs: dict,
          outputs: dict):
    address = f"{context.torii_base_address}:{torii_port}"
    path = f"/v1alpha/cluster/{cluster_uuid}/train"
    json_body = {
        "inputs": inputs,
        "outputs": outputs,
    }
    ainari_request.send_put_request(context,
                                    address,
                                    path,
                                    json_body)


def request(context: AccessContext,
            torii_port: int,
            cluster_uuid: str,
            inputs: dict,
            outputs: list):
    address = f"{context.torii_base_address}:{torii_port}"
    path = f"/v1alpha/cluster/{cluster_uuid}/request"
    json_body = {
        "inputs": inputs,
        "outputs": outputs,
    }
    return ainari_request.send_put_request(context,
                                           address,
                                           path,
                                           json_body)
