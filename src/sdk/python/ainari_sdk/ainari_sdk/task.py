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


def create_snapshot_save_task(context: AccessContext,
                              torii_address: str,
                              virtual_machine_uuid: str,
                              name: str,
                              secret_uuid: str = None) -> dict:
    """
    Saves the root-disk of a virtual machine as new snapshot-image. The virtual machine is only
    paused during the copy, so data, which is still in its memory, is missing in the snapshot.
    Run `sync` inside the virtual machine right before creating the snapshot.

    With `secret_uuid`, the snapshot is encrypted with this already existing secret, instead of a
    new generated secret.
    """
    path = f"/v1alpha/virtual_machine/{virtual_machine_uuid}/snapshot_save"
    json_body = {
        "name": name,
    }
    if secret_uuid:
        json_body["secret_uuid"] = secret_uuid
    return ainari_request.send_post_request(context,
                                            torii_address,
                                            path,
                                            json_body)


def create_snapshot_restore_task(context: AccessContext,
                                 torii_address: str,
                                 virtual_machine_uuid: str,
                                 image_uuid: str) -> dict:
    """
    Resets the root-disk of a virtual machine to an image, which must be a snapshot.
    """
    path = f"/v1alpha/virtual_machine/{virtual_machine_uuid}/snapshot_restore"
    json_body = {
        "image_uuid": image_uuid,
    }
    return ainari_request.send_post_request(context,
                                            torii_address,
                                            path,
                                            json_body)


def get_task(context: AccessContext,
             torii_address: str,
             task_uuid: str) -> dict:
    """
    The tasks are not bound to a virtual machine anymore, but the torii-address of a virtual
    machine still selects the sakura-host, which holds the task.
    """
    path = f"/v1alpha/task/{task_uuid}"
    return ainari_request.send_get_request(context,
                                           torii_address,
                                           path,
                                           "")


def list_tasks(context: AccessContext,
               torii_address: str) -> dict:
    """
    Lists all tasks of the sakura-host behind the given torii-address.
    """
    path = "/v1alpha/task"
    return ainari_request.send_get_request(context,
                                           torii_address,
                                           path,
                                           "")


def abort_task(context: AccessContext,
               torii_address: str,
               task_uuid: str):
    path = f"/v1alpha/task/{task_uuid}/abort"
    ainari_request.send_put_request(context,
                                    torii_address,
                                    path,
                                    "")


def wait_for_task_finished(context: AccessContext,
                           torii_address: str,
                           task_uuid: str,
                           time_interval: float = 1.0) -> dict:
    """
    Waits until the task reached one of its final states `Finished`, `Aborted` or `Error` and
    returns the task, so the caller can check its state and messages.
    """
    while True:
        result = get_task(context, torii_address, task_uuid)
        # in case that the task is already done, an unnecessary sleep should be avoided
        if result["state"] in ("Finished", "Aborted", "Error"):
            return result
        time.sleep(time_interval)
