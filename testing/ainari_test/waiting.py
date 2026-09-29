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

"""
Polling of states, which the backend reaches asynchronously, like the tasks of the sakura-hosts.
"""

import time

from ainari_sdk import task
from ainari_sdk import virtual_machine

from .checks import CheckFailed

# final states of a task, like they are serialized by the api
TASK_FINAL_STATES = ("Finished", "Aborted", "Error")


def wait_until(condition, timeout: float, what: str, interval: float = 2.0):
    """
    Calls the condition until it returns a truthy value and returns this value. Exceptions of the
    condition are treated like a falsy result, but the last one is part of the timeout-message.
    """
    end_time = time.time() + timeout
    last_error = None
    while True:
        try:
            result = condition()
            if result:
                return result
        except Exception as error:  # noqa: BLE001
            last_error = error
        if time.time() >= end_time:
            break
        time.sleep(interval)

    message = f"timeout after {timeout}s: {what}"
    if last_error is not None:
        message += f" (last error: {type(last_error).__name__}: {last_error})"
    raise TimeoutError(message)


def wait_for_task(context, torii_port: int, task_uuid: str, timeout: float) -> dict:
    """
    Waits until the task reached a final state and checks, that it was finished successfully.
    """
    result = wait_until(
        lambda: _final_task(context, torii_port, task_uuid),
        timeout,
        f"task '{task_uuid}' not finished")
    if result["state"] != "Finished":
        raise CheckFailed(f"task '{task_uuid}' ({result['task_type']}) ended in state "
                          f"'{result['state']}': {result.get('messages')}")
    return result


def _final_task(context, torii_port: int, task_uuid: str):
    result = task.get_task(context, torii_port, task_uuid)
    if result["state"] in TASK_FINAL_STATES:
        return result
    return None


def wait_for_vm_state(context, virtual_machine_uuid: str, expected_state: str,
                      timeout: float) -> dict:
    """
    Waits until hanami reports the virtual machine in the expected state and returns its data.
    The state ERROR is final, so it stops the wait immediately.
    """
    def condition():
        data = virtual_machine.get_virtual_machine(context, virtual_machine_uuid)
        if data.get("vm_state") == expected_state:
            return data
        if data.get("vm_state") == "ERROR" and expected_state != "ERROR":
            raise CheckFailed(f"virtual machine '{virtual_machine_uuid}' is in state ERROR")
        return None

    end_time = time.time() + timeout
    while True:
        try:
            result = condition()
        except CheckFailed:
            raise
        except Exception:  # noqa: BLE001
            result = None
        if result:
            return result
        if time.time() >= end_time:
            break
        time.sleep(2.0)

    current = virtual_machine.get_virtual_machine(context, virtual_machine_uuid).get("vm_state")
    raise TimeoutError(f"virtual machine '{virtual_machine_uuid}' did not reach state "
                       f"'{expected_state}' within {timeout}s (current: '{current}')")
