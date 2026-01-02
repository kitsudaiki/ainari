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

import time


def create_train_task(context: AccessContext,
                      torii_port: int,
                      name: str,
                      cluster_uuid: str,
                      inputs: list,
                      outputs: list,
                      number_of_epochs: int = 1,
                      timeLength: int = 1) -> dict:
    address = f"{context.torii_base_address}:{torii_port}"
    print(f"address: {address}")
    path = f"/v1alpha/cluster/{cluster_uuid}/task/train"
    json_body = {
        "name": name,
        "number_of_epochs": number_of_epochs,
        "inputs": inputs,
        "outputs": outputs,
        "time_length": timeLength,
    }
    return ainari_request.send_post_request(context,
                                            address,
                                            path,
                                            json_body)


def create_request_task(context: AccessContext,
                        torii_port: int,
                        name: str,
                        cluster_uuid: str,
                        inputs: list,
                        results: list,
                        timeLength: int = 1) -> dict:
    address = f"{context.torii_base_address}:{torii_port}"
    path = f"/v1alpha/cluster/{cluster_uuid}/task/request"
    json_body = {
        "name": name,
        "inputs": inputs,
        "results": results,
        "time_length": timeLength,
    }
    return ainari_request.send_post_request(context,
                                            address,
                                            path,
                                            json_body)


def create_checkpoint_save_task(context: AccessContext,
                                torii_port: int,
                                cluster_uuid: str,
                                name: str) -> dict:
    address = f"{context.torii_base_address}:{torii_port}"
    path = f"/v1alpha/cluster/{cluster_uuid}/task/checkpoint_save"
    json_body = {
        "name": name,
    }
    return ainari_request.send_post_request(context,
                                            address,
                                            path,
                                            json_body)


def create_checkpoint_restore_task(context: AccessContext,
                                   torii_port: int,
                                   cluster_uuid: str,
                                   name: str,
                                   checkpoint_uuid: str) -> dict:
    address = f"{context.torii_base_address}:{torii_port}"
    path = f"/v1alpha/cluster/{cluster_uuid}/task/checkpoint_restore"
    json_body = {
        "name": name,
        "checkpoint_uuid": checkpoint_uuid,
    }
    return ainari_request.send_post_request(context,
                                            address,
                                            path,
                                            json_body)


def get_task(context: AccessContext,
             torii_port: int,
             task_uuid: str,
             cluster_uuid: str) -> dict:
    address = f"{context.torii_base_address}:{torii_port}"
    path = f"/v1alpha/cluster/{cluster_uuid}/task/{task_uuid}"
    return ainari_request.send_get_request(context,
                                           address,
                                           path,
                                           "")


def list_tasks(context: AccessContext,
               torii_port: int,
               cluster_uuid: str) -> dict:
    address = f"{context.torii_base_address}:{torii_port}"
    path = f"/v1alpha/cluster/{cluster_uuid}/task"
    return ainari_request.send_get_request(context,
                                           address,
                                           path,
                                           "")


def delete_task(context: AccessContext,
                torii_port: int,
                task_uuid: str,
                cluster_uuid: str):
    address = f"{context.torii_base_address}:{torii_port}"
    path = f"/v1alpha/cluster/{cluster_uuid}/task/{task_uuid}"
    ainari_request.send_delete_request(context,
                                       address,
                                       path,
                                       "")


def abort_task(context: AccessContext,
               torii_port: int,
               task_uuid: str,
               cluster_uuid: str):
    address = f"{context.torii_base_address}:{torii_port}"
    path = f"/v1alpha/cluster/{cluster_uuid}/task/{task_uuid}/abort"
    ainari_request.send_put_request(context,
                                    address,
                                    path,
                                    "")


def wait_for_task_finished(context: AccessContext,
                           torii_port: int,
                           task_uuid: str,
                           cluster_uuid: str,
                           time_interval: float = 1.0):
    address = f"{context.torii_base_address}:{torii_port}"
    finished = False
    while not finished:
        result = get_task(context, address, task_uuid, cluster_uuid)
        finished = result["state"] == "FINISHED"
        # in case that the task is already finished, an unnecessary sleep should be avoided
        if finished:
            return
        time.sleep(time_interval)
