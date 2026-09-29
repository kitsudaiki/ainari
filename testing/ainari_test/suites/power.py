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
Reboot, stop and start of a virtual machine. A new boot is detected by the boot-id of the kernel,
which changes with every boot.
"""

from ainari_sdk import virtual_machine

from ainari_test.checks import check, check_equal
from ainari_test.framework import Suite
from ainari_test.waiting import wait_for_task, wait_for_vm_state

suite = Suite("power", "reboot, stop and start a virtual machine", depends=("ssh",))

BOOT_ID = "cat /proc/sys/kernel/random/boot_id"


def run_task(ctx, entry: dict, function, task_type: str) -> dict:
    result = function(ctx.api, entry["torii_port"], entry["uuid"])
    ctx.state.setdefault("tasks", []).append((entry["torii_port"], result["uuid"], task_type))
    return wait_for_task(ctx.api, entry["torii_port"], result["uuid"], ctx.config.task_timeout)


def expect_new_boot(ctx, entry: dict, old_boot_id: str) -> str:
    new_boot_id = ctx.ssh.wait(entry["floating_ip"], BOOT_ID,
                               expect=lambda output: output != old_boot_id)
    check(new_boot_id != old_boot_id, f"{entry['name']} was not booted again")
    return new_boot_id


@suite.test("reboot", requires=("ssh",))
def reboot(ctx):
    entry = ctx.state["virtual_machines"][0]
    ctx.ssh.refresh_sudo()
    boot_id = ctx.ssh.run(entry["floating_ip"], BOOT_ID)
    run_task(ctx, entry, virtual_machine.reboot_virtual_machine, "VirtualMachineReboot")
    expect_new_boot(ctx, entry, boot_id)
    check_equal(virtual_machine.get_virtual_machine(ctx.api, entry["uuid"])["vm_state"],
                "RUNNING", "state after reboot")


@suite.test("stop", requires=("ssh",), provides=("stopped",))
def stop(ctx):
    entry = ctx.state["virtual_machines"][0]
    ctx.ssh.refresh_sudo()
    entry["boot_id"] = ctx.ssh.run(entry["floating_ip"], BOOT_ID)
    run_task(ctx, entry, virtual_machine.stop_virtual_machine, "VirtualMachineStop")
    wait_for_vm_state(ctx.api, entry["uuid"], "STOPPED", ctx.config.task_timeout)
    ctx.ssh.wait_unreachable(entry["floating_ip"], timeout=60)
    ctx.state["stopped"] = True


@suite.test("stop of a stopped virtual machine is a no-op", requires=("stopped",))
def stop_again(ctx):
    entry = ctx.state["virtual_machines"][0]
    run_task(ctx, entry, virtual_machine.stop_virtual_machine, "VirtualMachineStop")
    check_equal(virtual_machine.get_virtual_machine(ctx.api, entry["uuid"])["vm_state"],
                "STOPPED", "state after second stop")


@suite.test("other virtual machines keep running", requires=("stopped",))
def others_running(ctx):
    for entry in ctx.state["virtual_machines"][1:]:
        check_equal(virtual_machine.get_virtual_machine(ctx.api, entry["uuid"])["vm_state"],
                    "RUNNING", f"state of {entry['name']}")
        check(ctx.ssh.is_reachable(entry["floating_ip"]), f"{entry['name']} is not reachable")


@suite.test("start", requires=("stopped",))
def start(ctx):
    entry = ctx.state["virtual_machines"][0]
    ctx.ssh.refresh_sudo()
    run_task(ctx, entry, virtual_machine.start_virtual_machine, "VirtualMachineStart")
    wait_for_vm_state(ctx.api, entry["uuid"], "RUNNING", ctx.config.task_timeout)
    expect_new_boot(ctx, entry, entry["boot_id"])


@suite.test("start of a running virtual machine is a no-op", requires=("ssh",))
def start_again(ctx):
    entry = ctx.state["virtual_machines"][0]
    boot_id = ctx.ssh.wait(entry["floating_ip"], BOOT_ID)
    run_task(ctx, entry, virtual_machine.start_virtual_machine, "VirtualMachineStart")
    check_equal(ctx.ssh.run(entry["floating_ip"], BOOT_ID), boot_id,
                "boot-id after starting a running virtual machine")
