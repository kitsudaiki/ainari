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
Isolated sakura-hosts, which are only used by the virtual machines of a single project. A host is
isolated by an admin, bound to the project of its first virtual machine and released again, when
its last virtual machine is deleted.

The suite runs before the virtual machines of the other suites are reserved, because the isolation
can only be changed on a host, which runs no virtual machine. Its own virtual machines are only
reserved and deleted again, so the host-resources are back at their baseline afterwards.
"""

import uuid

from ainari_sdk import ainari_exceptions
from ainari_sdk import host
from ainari_sdk import login
from ainari_sdk import virtual_machine

from ainari_test.checks import SkipTest, check, check_equal, expect_error
from ainari_test.framework import Suite
from ainari_test.waiting import wait_until

from . import hosts
from .virtual_machines import register_virtual_machine

suite = Suite("host_isolation", "isolated sakura-hosts for single projects",
              depends=("hosts", "resources"))


def is_unused(entry: dict) -> bool:
    return all(entry[key] == 0 for key in hosts.RESOURCE_FIELDS)


def isolated_host(ctx) -> dict:
    """
    The isolated host with its used resources, which are only part of the list, and its project,
    which is only part of the get-response.
    """
    host_uuid = ctx.state["isolated_host"]
    entry = next(item for item in host.list_hosts(ctx.api)["hosts"] if item["uuid"] == host_uuid)
    entry["project_id"] = host.get_host(ctx.api, host_uuid)["project_id"]
    return entry


def set_isolation(ctx, host_uuid: str, is_host_isolated: bool) -> dict:
    result = host.set_host_isolation(ctx.api, host_uuid, is_host_isolated)
    check_equal(result["uuid"], host_uuid, "uuid of the updated host")
    check_equal(result["is_host_isolated"], is_host_isolated, "isolation of the updated host")
    return result


def reserve(ctx, suffix: str, isolated: bool) -> dict:
    name = ctx.name(suffix)
    result = virtual_machine.reserve_virtual_machine(ctx.api,
                                                     name,
                                                     ctx.state["vm_type"],
                                                     ctx.config.disk_size,
                                                     ctx.state["network"],
                                                     isolated_host=isolated)
    register_virtual_machine(ctx, name, result["uuid"])
    ctx.log(f"{name}: {result['uuid']} (isolated_host={isolated})")
    return result


def delete(ctx, vm_uuid: str):
    """
    Deletes the virtual machine and waits, until its host-resources are released, so the following
    suites start again with the baseline of the host-resources.
    """
    virtual_machine.delete_virtual_machine(ctx.api, vm_uuid)
    ctx.cleanup.discard(vm_uuid)
    # the resources are released by a watcher, as soon as sakura has deleted the virtual machine
    if "host_usage_baseline" in ctx.state:
        wait_until(lambda: hosts.host_usage(ctx) == ctx.state["host_usage_baseline"],
                   ctx.config.delete_timeout,
                   "host-resources of the deleted virtual machine are released")


@suite.test("unknown sakura-host can not be isolated")
def unknown_host(ctx):
    expect_error(ainari_exceptions.NotFoundException,
                 host.set_host_isolation, ctx.api, str(uuid.uuid4()), True)


@suite.test("isolate an unused sakura-host", requires=("hosts",), provides=("isolated_host",))
def isolate(ctx):
    candidates = [entry for entry in host.list_hosts(ctx.api)["hosts"]
                  if is_unused(entry) and not entry["is_host_isolated"]]
    if not candidates:
        raise SkipTest("no sakura-host without virtual machines and isolation")
    entry = candidates[0]

    result = set_isolation(ctx, entry["uuid"], True)
    # the host is not bound to a project before its first virtual machine
    check_equal(result["project_id"], None, "project of the newly isolated host")
    # registered right away, so the host is not left isolated after an abort
    ctx.cleanup.add("host_isolation", entry["uuid"], entry["name"],
                    lambda: host.set_host_isolation(ctx.api, entry["uuid"], False))
    ctx.log(f"isolated sakura-host '{entry['name']}' ({entry['uuid']})")

    listed = {item["uuid"]: item for item in host.list_hosts(ctx.api)["hosts"]}
    check(listed[entry["uuid"]]["is_host_isolated"], "isolated host is not isolated in the list")
    ctx.state["isolated_host"] = entry["uuid"]


@suite.test("normal virtual machine avoids the isolated host",
            requires=("isolated_host", "network", "vm_type"))
def normal_vm_avoids_isolated_host(ctx):
    free_shared_cores = sum(entry["number_of_cores"] - entry["used_number_of_cores"]
                            for entry in host.list_hosts(ctx.api)["hosts"]
                            if not entry["is_host_isolated"])
    if free_shared_cores < ctx.config.number_of_cores:
        # all other hosts are full or there is no other host, so it has to be rejected, also
        # though the isolated host has enough free resources
        expect_error(ainari_exceptions.ConflictException,
                     virtual_machine.reserve_virtual_machine, ctx.api,
                     ctx.name("shared"), ctx.state["vm_type"], ctx.config.disk_size,
                     ctx.state["network"])
        return

    result = reserve(ctx, "shared", False)
    try:
        check(is_unused(isolated_host(ctx)),
              "virtual machine without isolated_host was placed on the isolated host")
    finally:
        delete(ctx, result["uuid"])


@suite.test("normal virtual machine can not be forced onto the isolated host",
            requires=("isolated_host", "network", "vm_type"))
def normal_vm_forced_on_isolated_host(ctx):
    # the host has enough free resources, but its isolation doesn't match the request
    error = expect_error(ainari_exceptions.ConflictException,
                         virtual_machine.reserve_virtual_machine, ctx.api,
                         ctx.name("forced-shared"), ctx.state["vm_type"], ctx.config.disk_size,
                         ctx.state["network"], host_uuid=ctx.state["isolated_host"])
    ctx.log(f"rejected: {error}")
    check(is_unused(isolated_host(ctx)), "rejected virtual machine allocated resources")


@suite.test("isolated virtual machine binds the isolated host to the project",
            requires=("isolated_host", "network", "vm_type"), provides=("isolated_vm",))
def isolated_vm_binds_host(ctx):
    result = reserve(ctx, "isolated", True)
    ctx.state["isolated_vm"] = result["uuid"]

    entry = isolated_host(ctx)
    project_id = login.validate_token(ctx.api)["context"]["project_id"]
    check_equal(entry["project_id"], project_id, "project of the isolated host")
    check_equal(entry["used_number_of_cores"], ctx.config.number_of_cores,
                "used cores of the isolated host")


@suite.test("isolation of a host in use can not be changed", requires=("isolated_vm",))
def change_host_in_use(ctx):
    error = expect_error(ainari_exceptions.ConflictException,
                         host.set_host_isolation, ctx.api, ctx.state["isolated_host"], False)
    ctx.log(f"rejected: {error}")
    check(isolated_host(ctx)["is_host_isolated"], "host in use lost its isolation")


@suite.test("deleting the last virtual machine releases the isolated host",
            requires=("isolated_vm",))
def release_host(ctx):
    delete(ctx, ctx.state.pop("isolated_vm"))
    wait_until(lambda: is_unused(isolated_host(ctx)), ctx.config.delete_timeout,
               "resources of the isolated host are released")
    entry = isolated_host(ctx)
    check_equal(entry["project_id"], None, "project of the released isolated host")
    check(entry["is_host_isolated"], "released host lost its isolation")


@suite.test("remove the isolation of the unused host", requires=("isolated_host",))
def remove_isolation(ctx):
    host_uuid = ctx.state.pop("isolated_host")
    set_isolation(ctx, host_uuid, False)
    ctx.cleanup.discard(host_uuid)
