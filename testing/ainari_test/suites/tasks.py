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
The tasks, which the previous suites created on the sakura-hosts.
"""

import uuid

from ainari_sdk import ainari_exceptions
from ainari_sdk import task

from ainari_test.checks import check, check_equal, check_in, expect_error
from ainari_test.framework import Suite

suite = Suite("tasks", "tasks of the sakura-hosts", depends=("virtual_machines",))


@suite.test("get every created task", requires=("tasks",))
def get_tasks(ctx):
    for torii_port, task_uuid, task_type in ctx.state["tasks"]:
        result = task.get_task(ctx.api, torii_port, task_uuid)
        check_equal(result["task_type"], task_type, f"type of task {task_uuid}")
        check_equal(result["state"], "Finished", f"state of task {task_uuid}")
        check(result["queued_at"] and result["started_at"] and result["finished_at"],
              f"timestamps of task {task_uuid} are incomplete: {result}")
        check(result["started_at"] <= result["finished_at"],
              f"task {task_uuid} finished before it started")


@suite.test("list tasks of the sakura-hosts", requires=("tasks",))
def list_tasks(ctx):
    by_port = {}
    for torii_port, task_uuid, _ in ctx.state["tasks"]:
        by_port.setdefault(torii_port, []).append(task_uuid)
    for torii_port, task_uuids in by_port.items():
        listed = [entry["uuid"] for entry in task.list_tasks(ctx.api, torii_port)["tasks"]]
        for task_uuid in task_uuids:
            check_in(task_uuid, listed, f"task in the list behind port {torii_port}")


@suite.test("unknown task is not found", requires=("virtual_machines",))
def unknown_task(ctx):
    torii_port = ctx.state["virtual_machines"][0]["torii_port"]
    expect_error(ainari_exceptions.NotFoundException, task.get_task, ctx.api, torii_port,
                 str(uuid.uuid4()))
