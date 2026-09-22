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


def create_checkpoint_save_task(context: AccessContext,
                                torii_port: int,
                                virtual_machine_uuid: str,
                                name: str) -> dict:
    address = f"{context.torii_base_address}:{torii_port}"
    path = f"/v1alpha/virtual_machine/{virtual_machine_uuid}/checkpoint_save"
    json_body = {
        "name": name,
    }
    return ainari_request.send_post_request(context,
                                            address,
                                            path,
                                            json_body)


def create_checkpoint_restore_task(context: AccessContext,
                                   torii_port: int,
                                   virtual_machine_uuid: str,
                                   name: str,
                                   checkpoint_uuid: str) -> dict:
    address = f"{context.torii_base_address}:{torii_port}"
    path = f"/v1alpha/virtual_machine/{virtual_machine_uuid}/checkpoint_restore"
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
             task_uuid: str) -> dict:
    """
    The tasks are not bound to a virtual machine anymore, but the torii-port still selects the
    sakura-host, which holds the task.
    """
    address = f"{context.torii_base_address}:{torii_port}"
    path = f"/v1alpha/task/{task_uuid}"
    return ainari_request.send_get_request(context,
                                           address,
                                           path,
                                           "")


def list_tasks(context: AccessContext,
               torii_port: int) -> dict:
    """
    Lists all tasks of the sakura-host behind the given torii-port.
    """
    address = f"{context.torii_base_address}:{torii_port}"
    path = "/v1alpha/task"
    return ainari_request.send_get_request(context,
                                           address,
                                           path,
                                           "")


def abort_task(context: AccessContext,
               torii_port: int,
               task_uuid: str):
    address = f"{context.torii_base_address}:{torii_port}"
    path = f"/v1alpha/task/{task_uuid}/abort"
    ainari_request.send_put_request(context,
                                    address,
                                    path,
                                    "")


def wait_for_task_finished(context: AccessContext,
                           torii_port: int,
                           task_uuid: str,
                           time_interval: float = 1.0):
    finished = False
    while not finished:
        result = get_task(context, torii_port, task_uuid)
        finished = result["state"] == "FINISHED"
        # in case that the task is already finished, an unnecessary sleep should be avoided
        if finished:
            return
        time.sleep(time_interval)
