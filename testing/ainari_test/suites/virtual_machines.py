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
Reservation of the virtual machines in hanami and their creation on the sakura-hosts. Hanami
picks a sakura-host for every one of them, so they can land on the same host or on different
ones, which differs from run to run.
"""

import ipaddress
import uuid

from ainari_sdk import ainari_exceptions
from ainari_sdk import virtual_machine

from ainari_test.checks import check, check_equal, check_in, exists, expect_error
from ainari_test.framework import Suite
from ainari_test.waiting import wait_for_vm_state

from . import hosts

suite = Suite("virtual_machines", "reserve and create virtual machines",
              depends=("hosts", "resources"))


def register_virtual_machine(ctx, name: str, vm_uuid: str):
    ctx.cleanup.add("virtual_machine", vm_uuid, name,
                    lambda: virtual_machine.delete_virtual_machine(ctx.api, vm_uuid),
                    lambda: exists(virtual_machine.get_virtual_machine, ctx.api, vm_uuid))


@suite.test("reserve virtual machines", requires=("hosts", "network"),
            provides=("reserved",))
def reserve(ctx):
    config = ctx.config
    reserved = []
    for number in range(1, config.number_of_virtual_machines + 1):
        name = ctx.name(str(number))
        result = virtual_machine.reserve_virtual_machine(ctx.api,
                                                         name,
                                                         config.number_of_cores,
                                                         config.memory_size,
                                                         config.disk_size,
                                                         ctx.state["network"])
        register_virtual_machine(ctx, name, result["uuid"])
        reserved.append({
            "name": name,
            "uuid": result["uuid"],
            "internal_ip": result["internal_ip"],
            "torii_port": result["torii_port"],
        })
        ctx.log(f"{name}: {result['uuid']} ({result['internal_ip']}, "
                f"proxy-port {result['torii_port']})")
    ctx.state["reserved"] = reserved

    subnet = ipaddress.ip_network(config.network_subnet, strict=False)
    for entry in reserved:
        check(ipaddress.ip_address(entry["internal_ip"]) in subnet,
              f"internal ip {entry['internal_ip']} is not within {subnet}")
    check_equal(len({entry["internal_ip"] for entry in reserved}), len(reserved),
                "number of unique internal ip-addresses")
    check_equal(len({entry["torii_port"] for entry in reserved}), len(reserved),
                "number of unique proxy-ports")


@suite.test("reservation allocates host-resources",
            requires=("reserved", "host_usage_baseline"))
def reservation_allocates_resources(ctx):
    config = ctx.config
    count = len(ctx.state["reserved"])
    baseline = ctx.state["host_usage_baseline"]
    usage = hosts.host_usage(ctx)
    check_equal(usage["used_number_of_cores"] - baseline["used_number_of_cores"],
                count * config.number_of_cores, "additionally used cores")
    check_equal(usage["amount_of_used_memory"] - baseline["amount_of_used_memory"],
                count * config.memory_size, "additionally used memory in MiB")
    check_equal(usage["amount_of_used_disk_space"] - baseline["amount_of_used_disk_space"],
                count * config.disk_size, "additionally used disk-space in GiB")


@suite.test("too large virtual machine is rejected", requires=("network",))
def too_large(ctx):
    # no host has that many cores, so hanami can not find a host for it
    error = expect_error(ainari_exceptions.ConflictException,
                         virtual_machine.reserve_virtual_machine, ctx.api,
                         ctx.name("too-large"), 100000, ctx.config.memory_size,
                         ctx.config.disk_size, ctx.state["network"])
    ctx.log(f"rejected: {error}")


@suite.test("create virtual machines on their sakura-hosts",
            requires=("reserved", "image", "public_key"), provides=("virtual_machines",))
def create(ctx):
    for entry in ctx.state["reserved"]:
        result = virtual_machine.create_virtual_machine(ctx.api,
                                                        entry["torii_port"],
                                                        entry["uuid"],
                                                        ctx.state["image"],
                                                        ctx.state["public_key"])
        entry["create_task"] = result["uuid"]
        ctx.state.setdefault("tasks", []).append(
            (entry["torii_port"], result["uuid"], "VirtualMachineCreate"))

    # the virtual machines are installed and booted in parallel, so they are awaited together
    for entry in ctx.state["reserved"]:
        wait_for_vm_state(ctx.api, entry["uuid"], "RUNNING", ctx.config.vm_create_timeout)
        ctx.log(f"{entry['name']} is running")
    ctx.state["virtual_machines"] = ctx.state["reserved"]


@suite.test("get virtual machines", requires=("virtual_machines",))
def get(ctx):
    config = ctx.config
    for entry in ctx.state["virtual_machines"]:
        result = virtual_machine.get_virtual_machine(ctx.api, entry["uuid"])
        what = f"virtual machine {entry['name']}"
        check_equal(result["name"], entry["name"], f"name of {what}")
        check_equal(result["vm_state"], "RUNNING", f"state of {what}")
        check_equal(result["number_of_cores"], config.number_of_cores, f"cores of {what}")
        check_equal(result["memory_size"], config.memory_size, f"memory of {what}")
        check_equal(result["disk_size"], config.disk_size, f"disk of {what}")
        check_equal(result["network_uuid"], ctx.state["network"], f"network of {what}")
        check_equal(result["image_uuid"], ctx.state["image"], f"image of {what}")
        check_equal(result["internal_ip"], entry["internal_ip"], f"internal ip of {what}")
        check_equal(result["torii_port"], entry["torii_port"], f"proxy-port of {what}")


@suite.test("list and count virtual machines", requires=("virtual_machines",))
def list_and_count(ctx):
    listed = {entry["uuid"]: entry
              for entry in virtual_machine.list_virtual_machines(ctx.api)["virtual_machines"]}
    for entry in ctx.state["virtual_machines"]:
        check_in(entry["uuid"], listed, "virtual machine in list")
        check_equal(listed[entry["uuid"]]["proxy_port"], entry["torii_port"],
                    f"proxy-port of {entry['name']} in list")
    count = virtual_machine.get_virtual_machine_count(ctx.api)["number_of_items"]
    check(count >= len(ctx.state["virtual_machines"]),
          f"virtual-machine-count is {count}")


@suite.test("unknown virtual machine is not found")
def unknown(ctx):
    expect_error(ainari_exceptions.NotFoundException, virtual_machine.get_virtual_machine,
                 ctx.api, str(uuid.uuid4()))
