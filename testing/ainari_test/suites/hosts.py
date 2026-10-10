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
The sakura-hosts, which run the virtual machines, and the onsen-hosts, which store the images.
"""

import uuid

from ainari_sdk import ainari_exceptions
from ainari_sdk import host

from ainari_test.checks import check, check_equal, check_keys, expect_error
from ainari_test.framework import Suite
from ainari_test.waiting import wait_until

suite = Suite("hosts", "sakura- and onsen-hosts")

RESOURCE_FIELDS = ("used_number_of_cores", "amount_of_used_memory", "amount_of_used_disk_space")


def host_usage(ctx) -> dict:
    """
    Sum of the resources of all sakura-hosts, which are allocated by virtual machines.
    """
    hosts = host.list_hosts(ctx.api)["hosts"]
    return {key: sum(entry.get(key, 0) for entry in hosts) for key in RESOURCE_FIELDS}


def usage_by_host(ctx) -> dict:
    """
    Resources of every sakura-host, which are allocated by virtual machines, by the uuid of the
    host.
    """
    return {entry["uuid"]: {key: entry.get(key, 0) for key in RESOURCE_FIELDS}
            for entry in host.list_hosts(ctx.api)["hosts"]}


def vm_resources(ctx, factor: int = 1) -> dict:
    """
    Resources, which a virtual machine of the test allocates on its host, multiplied by the
    factor, so -1 are the resources, which it releases.
    """
    config = ctx.config
    return {
        "used_number_of_cores": factor * config.number_of_cores,
        "amount_of_used_memory": factor * config.memory_size,
        "amount_of_used_disk_space": factor * config.disk_size,
    }


def usage_changes(before: dict, after: dict) -> dict:
    """
    Changes of the allocated resources between two results of usage_by_host. Hosts without a
    change are left out.
    """
    changes = {}
    for host_uuid, usage in after.items():
        previous = before.get(host_uuid, {key: 0 for key in RESOURCE_FIELDS})
        change = {key: usage[key] - previous[key] for key in RESOURCE_FIELDS}
        if any(change.values()):
            changes[host_uuid] = change
    return changes


@suite.test("all sakura-hosts are registered", provides=("hosts",))
def sakura_hosts_registered(ctx):
    # A sakura, which is started before hanami, fails its registration and is restarted by
    # docker, so the hosts are not there immediately after the stack is up.
    expected = ctx.config.number_of_sakura_hosts

    def registered():
        hosts = host.list_hosts(ctx.api)["hosts"]
        return hosts if len(hosts) >= expected else None

    hosts = wait_until(registered,
                       ctx.config.host_registration_timeout,
                       f"{expected} sakura-hosts registered")
    for entry in hosts:
        ctx.log(f"sakura-host '{entry['name']}' at {entry['host_address']}")
    ctx.state["hosts"] = hosts


@suite.test("sakura-hosts have unique names and addresses", requires=("hosts",))
def unique_hosts(ctx):
    hosts = ctx.state["hosts"]
    names = [entry["name"] for entry in hosts]
    addresses = [entry["host_address"] for entry in hosts]
    check_equal(len(set(names)), len(names), "number of unique host-names")
    check_equal(len(set(addresses)), len(addresses), "number of unique host-addresses")


@suite.test("get every sakura-host", requires=("hosts",))
def get_hosts(ctx):
    for entry in ctx.state["hosts"]:
        result = host.get_host(ctx.api, entry["uuid"])
        check_equal(result["uuid"], entry["uuid"], "uuid of the host")
        check_equal(result["name"], entry["name"], "name of the host")
        check_equal(result["host_address"], entry["host_address"], "address of the host")


@suite.test("sakura-hosts report their resources", requires=("hosts",),
            provides=("host_usage_baseline",))
def host_resources(ctx):
    for entry in host.list_hosts(ctx.api)["hosts"]:
        check_keys(entry, ("number_of_cores", "memory_size", "disk_space") + RESOURCE_FIELDS,
                   f"host '{entry['name']}'")
        check(entry["number_of_cores"] > 0, f"host '{entry['name']}' has no cores")
        check(entry["memory_size"] > 0, f"host '{entry['name']}' has no memory")
        check(entry["used_number_of_cores"] <= entry["number_of_cores"],
              f"host '{entry['name']}' uses more cores than it has")
        ctx.log(f"{entry['name']}: cores {entry['used_number_of_cores']}/"
                f"{entry['number_of_cores']}, memory {entry['amount_of_used_memory']}/"
                f"{entry['memory_size']} MiB, disk {entry['amount_of_used_disk_space']}/"
                f"{entry['disk_space']} GiB")

    total_free_cores = sum(entry["number_of_cores"] - entry["used_number_of_cores"]
                           for entry in host.list_hosts(ctx.api)["hosts"])
    needed_cores = ctx.config.number_of_virtual_machines * ctx.config.number_of_cores
    check(total_free_cores >= needed_cores,
          f"the hosts have only {total_free_cores} free cores, but {needed_cores} are needed")
    ctx.state["host_usage_baseline"] = host_usage(ctx)


@suite.test("unknown sakura-host is not found")
def unknown_host(ctx):
    expect_error(ainari_exceptions.NotFoundException, host.get_host, ctx.api, str(uuid.uuid4()))


@suite.test("onsen-hosts are registered")
def onsen_hosts(ctx):
    hosts = wait_until(lambda: host.list_onsen_hosts(ctx.api)["hosts"],
                       ctx.config.host_registration_timeout,
                       "at least one onsen-host registered")
    for entry in hosts:
        ctx.log(f"onsen-host '{entry['name']}' at {entry['host_address']}")
        check_equal(host.get_onsen_host(ctx.api, entry["uuid"])["uuid"], entry["uuid"],
                    "uuid of the onsen-host")
