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
Deletion of all resources of the run. Every deletion is verified, so this suite tests the
delete-endpoints as well. It runs always, also after an abort, unless --keep is given.
"""

from ainari_sdk import network_filter
from ainari_sdk import virtual_machine
from ainari_sdk import vm_type

from ainari_test.checks import check, check_equal, check_not_in
from ainari_test.framework import Suite
from ainari_test.waiting import wait_until

from . import hosts

suite = Suite("cleanup", "delete and verify all created resources", always=True)


def delete_kind(ctx, kind: str):
    entries = ctx.cleanup.pending(kind)
    if not entries:
        ctx.log(f"no {kind} to delete")
        return entries
    errors = ctx.cleanup.run(kind)
    check(not errors, f"deletion failed: {errors}")
    for entry in entries:
        if entry.exists is not None:
            wait_until(lambda entry=entry: not entry.exists(), ctx.config.delete_timeout,
                       f"{kind} {entry.label} ({entry.uuid}) is gone")
    return entries


@suite.test("delete floating ip-addresses")
def delete_floating_ips(ctx):
    delete_kind(ctx, "floating_ip")


@suite.test("delete virtual machines")
def delete_virtual_machines(ctx):
    entries = delete_kind(ctx, "virtual_machine")
    listed = [entry["uuid"]
              for entry in virtual_machine.list_virtual_machines(ctx.api)["virtual_machines"]]
    for entry in entries:
        check_not_in(entry.uuid, listed, "deleted virtual machine in list")


@suite.test("network-filters of the deleted virtual machines are gone",
            requires=("network_filter_left",))
def network_filters_deleted(ctx):
    listed = [entry["uuid"]
              for entry in network_filter.list_network_filters(ctx.api)["network_filters"]]
    check_not_in(ctx.state["network_filter_left"], listed,
                 "network-filter of a deleted virtual machine in list")


@suite.test("host-resources are released", requires=("host_usage_baseline",))
def resources_released(ctx):
    baseline = ctx.state["host_usage_baseline"]
    # the resources are released by a watcher, as soon as sakura has deleted the virtual machine
    try:
        wait_until(lambda: hosts.host_usage(ctx) == baseline, ctx.config.delete_timeout,
                   "host-resources are released")
    except TimeoutError:
        check_equal(hosts.host_usage(ctx), baseline, "used host-resources after the deletion")


# after the virtual machines, because a host can only be released, while it runs none of them
@suite.test("reset host isolation")
def reset_host_isolation(ctx):
    delete_kind(ctx, "host_isolation")


@suite.test("delete secrets")
def delete_secrets(ctx):
    delete_kind(ctx, "secret")


@suite.test("delete images and snapshots")
def delete_images(ctx):
    delete_kind(ctx, "image")


@suite.test("delete network")
def delete_network(ctx):
    delete_kind(ctx, "network")


@suite.test("delete vm-types")
def delete_vm_types(ctx):
    entries = delete_kind(ctx, "vm_type")
    listed = [entry["uuid"] for entry in vm_type.list_vm_types(ctx.api)["vm_types"]]
    for entry in entries:
        check_not_in(entry.uuid, listed, "deleted vm-type in list")


@suite.test("delete public key")
def delete_public_key(ctx):
    delete_kind(ctx, "public_key")


# Users and projects come last, because a project can only be deleted without resources and a
# user only, if the projects, in which the user is admin, have no resources anymore. The
# default-project of a user is deleted together with the user, so the users come first.
@suite.test("delete users")
def delete_users(ctx):
    delete_kind(ctx, "user")


@suite.test("delete projects")
def delete_projects(ctx):
    delete_kind(ctx, "project")


@suite.test("nothing is left")
def nothing_left(ctx):
    leftovers = [f"{entry.kind} {entry.label}" for entry in ctx.cleanup.pending()]
    check(not leftovers, f"resources left: {leftovers}")
