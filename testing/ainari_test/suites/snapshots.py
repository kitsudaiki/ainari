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
Snapshots of the root-disk of a virtual machine. A marker-file is written before the snapshot and
changed afterwards, so the restore can be verified by its content.
"""

import re
import uuid

from ainari_sdk import ainari_exceptions
from ainari_sdk import image
from ainari_sdk import task
from ainari_sdk import virtual_machine

from ainari_test.checks import check, check_equal, check_in, exists, expect_error
from ainari_test.framework import Suite
from ainari_test.waiting import wait_for_task, wait_for_vm_state

suite = Suite("snapshots", "save and restore snapshots", depends=("ssh",))

MARKER = "~/ainari-snapshot-marker"
AFTER_SNAPSHOT = "~/ainari-after-snapshot"


@suite.test("write marker-file", requires=("ssh",), provides=("marker",))
def write_marker(ctx):
    entry = ctx.state["virtual_machines"][0]
    ctx.ssh.refresh_sudo()
    marker = str(uuid.uuid4())
    # the snapshot contains only, what is written to the disk, so the data is synced first
    ctx.ssh.run(entry["floating_ip"], f"echo {marker} > {MARKER} && sync")
    ctx.state["marker"] = marker


@suite.test("save snapshot", requires=("marker",), provides=("snapshot",))
def save_snapshot(ctx):
    entry = ctx.state["virtual_machines"][0]
    name = ctx.name("snapshot")
    result = task.create_snapshot_save_task(ctx.api, entry["torii_port"], entry["uuid"], name)
    ctx.state.setdefault("tasks", []).append((entry["torii_port"], result["uuid"],
                                              "SnapshotSave"))

    # the snapshot is registered in ryokan before the task is queued, so it has to be removed,
    # even if the task fails
    match = re.search(r"snapshot-image ([0-9a-f-]{36})", result["description"])
    if match is None:
        snapshots = [item for item in image.list_images(ctx.api)["images"]
                     if item["name"] == name]
        snapshot_uuid = snapshots[0]["uuid"] if snapshots else None
    else:
        snapshot_uuid = match.group(1)
    if snapshot_uuid is not None:
        ctx.cleanup.add("image", snapshot_uuid, name,
                        lambda: image.delete_image(ctx.api, snapshot_uuid),
                        lambda: exists(image.get_image, ctx.api, snapshot_uuid))

    wait_for_task(ctx.api, entry["torii_port"], result["uuid"], ctx.config.task_timeout)
    check(snapshot_uuid is not None, "uuid of the snapshot not found")
    ctx.state["snapshot"] = snapshot_uuid
    ctx.log(f"snapshot {snapshot_uuid}")


@suite.test("snapshot is listed as image", requires=("snapshot",))
def snapshot_image(ctx):
    snapshot_uuid = ctx.state["snapshot"]
    result = image.get_image(ctx.api, snapshot_uuid)
    check_equal(result["name"], ctx.name("snapshot"), "name of the snapshot")
    check_equal(result["is_snapshot"], True, "snapshot is marked as snapshot")
    listed = {item["uuid"]: item for item in image.list_images(ctx.api)["images"]}
    check_in(snapshot_uuid, listed, "snapshot in image-list")
    check_equal(listed[snapshot_uuid]["is_snapshot"], True, "snapshot-flag in image-list")


@suite.test("virtual machine keeps running after the snapshot", requires=("snapshot",))
def running_after_snapshot(ctx):
    entry = ctx.state["virtual_machines"][0]
    check_equal(virtual_machine.get_virtual_machine(ctx.api, entry["uuid"])["vm_state"],
                "RUNNING", "state after the snapshot")
    check_equal(ctx.ssh.run(entry["floating_ip"], f"cat {MARKER}"), ctx.state["marker"],
                "marker-file after the snapshot")


@suite.test("change the disk after the snapshot", requires=("snapshot",),
            provides=("changed",))
def change_disk(ctx):
    entry = ctx.state["virtual_machines"][0]
    ctx.ssh.run(entry["floating_ip"],
                f"echo changed > {MARKER} && touch {AFTER_SNAPSHOT} && sync")
    ctx.state["changed"] = True


@suite.test("restore snapshot", requires=("changed",))
def restore_snapshot(ctx):
    entry = ctx.state["virtual_machines"][0]
    result = task.create_snapshot_restore_task(ctx.api, entry["torii_port"], entry["uuid"],
                                               ctx.state["snapshot"])
    ctx.state.setdefault("tasks", []).append((entry["torii_port"], result["uuid"],
                                              "SnapshotRestore"))
    wait_for_task(ctx.api, entry["torii_port"], result["uuid"], ctx.config.task_timeout)
    wait_for_vm_state(ctx.api, entry["uuid"], "RUNNING", ctx.config.task_timeout)

    ctx.ssh.refresh_sudo()
    marker = ctx.ssh.wait(entry["floating_ip"], f"cat {MARKER}")
    check_equal(marker, ctx.state["marker"], "marker-file after the restore")
    check_equal(ctx.ssh.run(entry["floating_ip"],
                            f"test -e {AFTER_SNAPSHOT} && echo exists || echo missing"),
                "missing", "file, which was created after the snapshot")


@suite.test("machine-id is kept by the restore", requires=("changed",))
def machine_id_kept(ctx):
    entry = ctx.state["virtual_machines"][0]
    check_equal(ctx.ssh.wait(entry["floating_ip"], "cat /etc/machine-id"), entry["machine_id"],
                "machine-id after the restore")


@suite.test("restore of an image, which is no snapshot, is rejected",
            requires=("virtual_machines", "image"))
def restore_non_snapshot(ctx):
    entry = ctx.state["virtual_machines"][0]
    expect_error(ainari_exceptions.BadRequestException, task.create_snapshot_restore_task,
                 ctx.api, entry["torii_port"], entry["uuid"], ctx.state["image"])


@suite.test("restore of an unknown image is rejected", requires=("virtual_machines",))
def restore_unknown(ctx):
    entry = ctx.state["virtual_machines"][0]
    expect_error(ainari_exceptions.NotFoundException, task.create_snapshot_restore_task,
                 ctx.api, entry["torii_port"], entry["uuid"], str(uuid.uuid4()))
