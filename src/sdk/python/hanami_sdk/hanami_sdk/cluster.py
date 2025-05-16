# Copyright 2022 Tobias Anker
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

from . import hanami_request


def create_cluster(token: str,
                   address: str,
                   name: str,
                   template: str,
                   verify_connection: bool = True) -> dict:
    path = "/v1alpha/cluster"
    json_body = {
        "name": name,
        "template": template,
    }
    return hanami_request.send_post_request(token,
                                            address,
                                            path,
                                            json_body,
                                            verify=verify_connection)


def save_cluster(token: str,
                 address: str,
                 name: str,
                 cluster_uuid: str,
                 verify_connection: bool = True) -> dict:
    path = "/v1alpha/cluster/save"
    json_body = {
        "name": name,
        "cluster_uuid": cluster_uuid,
    }
    return hanami_request.send_post_request(token,
                                            address,
                                            path,
                                            json_body,
                                            verify=verify_connection)


def restore_cluster(token: str,
                    address: str,
                    checkpoint_uuid: str,
                    cluster_uuid: str,
                    verify_connection: bool = True) -> dict:
    path = "/v1alpha/cluster/load"
    json_body = {
        "checkpoint_uuid": checkpoint_uuid,
        "cluster_uuid": cluster_uuid,
    }
    return hanami_request.send_post_request(token,
                                            address,
                                            path,
                                            json_body,
                                            verify=verify_connection)


def get_cluster(token: str,
                address: str,
                cluster_uuid: str,
                verify_connection: bool = True) -> dict:
    path = f"/v1alpha/cluster/{cluster_uuid}"
    return hanami_request.send_get_request(token,
                                           address,
                                           path,
                                           "",
                                           verify=verify_connection)


def list_clusters(token: str,
                  address: str,
                  verify_connection: bool = True) -> dict:
    path = "/v1alpha/cluster"
    return hanami_request.send_get_request(token,
                                           address,
                                           path,
                                           "",
                                           verify=verify_connection)


def delete_cluster(token: str,
                   address: str,
                   cluster_uuid: str,
                   verify_connection: bool = True):
    path = f"/v1alpha/cluster/{cluster_uuid}"
    hanami_request.send_delete_request(token,
                                       address,
                                       path,
                                       "",
                                       verify=verify_connection)


def delete_all_cluster(token: str,
                       address: str,
                       verify_connection: bool = True):
    body = list_clusters(token, address, False)["clusters"]
    for entry in body:
        delete_cluster(token, address, entry["uuid"], verify_connection)


def switch_host(token: str,
                address: str,
                cluster_uuid: str,
                host_uuid: str,
                verify_connection: bool = True):
    path = "/v1alpha/cluster/switch_host"
    json_body = {
        "cluster_uuid": cluster_uuid,
        "host_uuid": host_uuid,
        "hexagon_id": 1,
    }
    return hanami_request.send_put_request(token,
                                           address,
                                           path,
                                           json_body,
                                           verify=verify_connection)


def switch_to_task_mode(token: str,
                        address: str,
                        cluster_uuid: str,
                        verify_connection: bool = True):
    path = f"/v1alpha/cluster/{cluster_uuid}/mode"
    json_body = {
        "mode": "Task",
    }
    return hanami_request.send_put_request(token,
                                           address,
                                           path,
                                           json_body,
                                           verify=verify_connection)


def switch_to_direct_mode(token: str,
                          address: str,
                          cluster_uuid: str,
                          verify_connection: bool = True):
    path = f"/v1alpha/cluster/{cluster_uuid}/mode"
    json_body = {
        "mode": "Direct",
    }
    return hanami_request.send_put_request(token,
                                           address,
                                           path,
                                           json_body,
                                           verify=verify_connection)


def train(token: str,
          address: str,
          cluster_uuid: str,
          inputs: dict,
          outputs: dict,
          verify_connection: bool = True):
    path = f"/v1alpha/cluster/{cluster_uuid}/train"
    json_body = {
        "inputs": inputs,
        "outputs": outputs,
    }
    hanami_request.send_put_request(token,
                                    address,
                                    path,
                                    json_body,
                                    verify=verify_connection)


def request(token: str,
            address: str,
            cluster_uuid: str,
            inputs: dict,
            outputs: list,
            verify_connection: bool = True):
    path = f"/v1alpha/cluster/{cluster_uuid}/request"
    json_body = {
        "inputs": inputs,
        "outputs": outputs,
    }
    return hanami_request.send_put_request(token,
                                           address,
                                           path,
                                           json_body,
                                           verify=verify_connection)

