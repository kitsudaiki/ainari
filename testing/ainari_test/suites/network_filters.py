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
Packet filters of the virtual machines. They are managed by hanami and applied by the torii of
the host of a virtual machine: the ingress-filter guards the traffic towards the virtual machine
by its source, the egress-filter the traffic of the virtual machine by its destination. The
traffic-tests run between two virtual machines, which can run on different sakura-hosts, so the
filters are checked on the local delivery and behind the overlay as well.
"""

from ainari_sdk import ainari_exceptions
from ainari_sdk import network_filter

from ainari_test.checks import (SkipTest, check, check_equal, check_in, check_not_in,
                                exists, expect_error)
from ainari_test.framework import Suite
from ainari_test.waiting import wait_until

suite = Suite("network_filters", "ingress- and egress-filters of the virtual machines",
              depends=("networking",))

# the filters take effect at once, but the check of a blocked path needs a few tries
FILTER_TIMEOUT = 30

# an address of TEST-NET-2, which is never used within a setup
FOREIGN_ADDRESS = "198.51.100.1"

# a port, on which nothing listens within the virtual machines
CLOSED_PORT = 8080


def require_two(ctx) -> list:
    virtual_machines = ctx.state["virtual_machines"]
    if len(virtual_machines) < 2:
        raise SkipTest("requires at least two virtual machines")
    return virtual_machines


def clear(ctx, entry: dict, direction: str):
    """
    Removes the filter of one direction of a virtual machine, which may not exist.
    """
    try:
        network_filter.delete_network_filter(ctx.api, entry["uuid"], direction)
    except ainari_exceptions.NotFoundException:
        pass


def can_ping(ctx, source: dict, target: dict) -> bool:
    output = ctx.ssh.run(source["floating_ip"],
                         f"ping -c 2 -W 1 {target['internal_ip']} >/dev/null "
                         f"&& echo reachable || echo blocked")
    return output == "reachable"


def tcp_state(ctx, source: dict, target: dict, port: int) -> str:
    """
    Connects from one virtual machine to a port of another one. A closed port answers with a
    reset, while a filtered one doesn't answer at all.

    Returns "open", "closed" or "filtered".
    """
    output = ctx.ssh.run(source["floating_ip"],
                         f"timeout 3 bash -c '</dev/tcp/{target['internal_ip']}/{port}' "
                         f"2>/dev/null; echo $?")
    return {"0": "open", "124": "filtered"}.get(output, "closed")


def wait_ping(ctx, source: dict, target: dict, expected: bool, what: str):
    wait_until(lambda: can_ping(ctx, source, target) == expected, FILTER_TIMEOUT, what)


@suite.test("invalid network-filter requests are rejected", requires=("virtual_machines",))
def invalid_requests(ctx):
    entry = ctx.state["virtual_machines"][0]
    unknown = "00000000-0000-0000-0000-000000000000"

    expect_error(ainari_exceptions.NotFoundException, network_filter.add_network_filter_ports,
                 ctx.api, unknown, "ingress", ["22"])
    expect_error(ainari_exceptions.NotFoundException, network_filter.get_network_filter,
                 ctx.api, entry["uuid"], "ingress")
    expect_error((ainari_exceptions.NotFoundException, ainari_exceptions.BadRequestException),
                 network_filter.add_network_filter_ports, ctx.api, entry["uuid"], "sideways",
                 ["22"])
    expect_error(ainari_exceptions.BadRequestException,
                 network_filter.add_network_filter_ip_ranges, ctx.api, entry["uuid"], "ingress",
                 [])
    expect_error(ainari_exceptions.BadRequestException,
                 network_filter.add_network_filter_ip_ranges, ctx.api, entry["uuid"], "ingress",
                 ["10.0.0.256"])
    expect_error(ainari_exceptions.BadRequestException,
                 network_filter.add_network_filter_ports, ctx.api, entry["uuid"], "egress", ["0"])

    # nothing of the rejected requests was stored
    listed = [item["virtual_machine_uuid"]
              for item in network_filter.list_network_filters(ctx.api)["network_filters"]]
    check_not_in(entry["uuid"], listed, "virtual machine with rejected filters in list")


@suite.test("add, get, list and remove network-filter entries", requires=("virtual_machines",))
def add_get_list_remove(ctx):
    entry = ctx.state["virtual_machines"][0]
    # the entries allow every address and port, so the virtual machine stays reachable
    try:
        result = network_filter.add_network_filter_ip_ranges(
            ctx.api, entry["uuid"], "ingress", ["0.0.0.0/0", "10.1.0.0-10.1.0.255"])
        check_equal(result["ip_ranges"], ["0.0.0.0/0", "10.1.0.0/24"],
                    "ip-ranges in canonical notation")
        check_equal(result["direction"], "ingress", "direction")
        check_equal(result["virtual_machine_uuid"], entry["uuid"], "virtual machine")
        filter_uuid = result["uuid"]

        # an entry, which is already present, is not added twice
        result = network_filter.add_network_filter_ip_ranges(ctx.api, entry["uuid"], "ingress",
                                                             ["10.1.0.0/24"])
        check_equal(result["ip_ranges"], ["0.0.0.0/0", "10.1.0.0/24"], "ip-ranges")
        result = network_filter.add_network_filter_ports(ctx.api, entry["uuid"], "ingress",
                                                         ["1-65535", "22"])
        check_equal(result["ports"], ["1-65535", "22"], "ports")
        check_equal(result["uuid"], filter_uuid, "uuid of the updated filter")

        result = network_filter.get_network_filter(ctx.api, entry["uuid"], "ingress")
        check_equal(result["uuid"], filter_uuid, "uuid")
        check_equal(result["ip_ranges"], ["0.0.0.0/0", "10.1.0.0/24"], "ip-ranges of get")
        check_equal(result["ports"], ["1-65535", "22"], "ports of get")
        expect_error(ainari_exceptions.NotFoundException, network_filter.get_network_filter,
                     ctx.api, entry["uuid"], "egress")

        listed = {item["uuid"]: item
                  for item in network_filter.list_network_filters(ctx.api)["network_filters"]}
        check_in(filter_uuid, listed, "filter in list")
        check_equal(listed[filter_uuid]["ports"], ["1-65535", "22"], "ports in list")

        # an entry is removed by the addresses it covers, not by its notation
        result = network_filter.delete_network_filter_ip_ranges(
            ctx.api, entry["uuid"], "ingress", ["10.1.0.0-10.1.0.255"])
        check_equal(result["ip_ranges"], ["0.0.0.0/0"], "ip-ranges after the removal")
        result = network_filter.delete_network_filter_ports(ctx.api, entry["uuid"], "ingress",
                                                            ["22", "1-65535"])
        check_equal(result["ports"], [], "ports after the removal")

        # removing the last entry removes the whole filter
        result = network_filter.delete_network_filter_ip_ranges(ctx.api, entry["uuid"],
                                                                "ingress", ["0.0.0.0/0"])
        check_equal(result["ip_ranges"], [], "ip-ranges after removing the last one")
        check(not exists(network_filter.get_network_filter, ctx.api, entry["uuid"], "ingress"),
              "filter without entries still exists")
        listed = [item["uuid"]
                  for item in network_filter.list_network_filters(ctx.api)["network_filters"]]
        check_not_in(filter_uuid, listed, "removed filter in list")

        # a filter is deleted as a whole as well
        network_filter.add_network_filter_ports(ctx.api, entry["uuid"], "egress", ["1-65535"])
        network_filter.delete_network_filter(ctx.api, entry["uuid"], "egress")
        check(not exists(network_filter.get_network_filter, ctx.api, entry["uuid"], "egress"),
              "deleted filter still exists")
    finally:
        clear(ctx, entry, "ingress")
        clear(ctx, entry, "egress")


@suite.test("ingress ip-ranges only let the listed sources in", requires=("ssh",))
def ingress_ip_ranges(ctx):
    source, target = require_two(ctx)[:2]
    check(can_ping(ctx, source, target), f"{source['name']} can not ping {target['name']} "
                                         f"without a filter")
    try:
        # the target is only reachable for a foreign address now. Its own ssh-access is blocked
        # as well, so everything is checked from the source.
        network_filter.add_network_filter_ip_ranges(ctx.api, target["uuid"], "ingress",
                                                    [FOREIGN_ADDRESS])
        wait_ping(ctx, source, target, False,
                  f"{source['name']} is blocked by the ingress-filter of {target['name']}")

        network_filter.add_network_filter_ip_ranges(ctx.api, target["uuid"], "ingress",
                                                    [source["internal_ip"]])
        wait_ping(ctx, source, target, True,
                  f"{source['name']} is allowed by the ingress-filter of {target['name']}")
    finally:
        clear(ctx, target, "ingress")
    wait_ping(ctx, source, target, True, f"{target['name']} is unfiltered again")
    ctx.ssh.wait(target["floating_ip"], "true", timeout=FILTER_TIMEOUT)


@suite.test("ingress ports only let the listed ports in", requires=("ssh",))
def ingress_ports(ctx):
    source, target = require_two(ctx)[:2]
    check_equal(tcp_state(ctx, source, target, CLOSED_PORT), "closed",
                f"port {CLOSED_PORT} of {target['name']} without a filter")
    try:
        network_filter.add_network_filter_ports(ctx.api, target["uuid"], "ingress", ["22"])
        wait_until(lambda: tcp_state(ctx, source, target, CLOSED_PORT) == "filtered",
                   FILTER_TIMEOUT, f"port {CLOSED_PORT} of {target['name']} is filtered")
        check_equal(tcp_state(ctx, source, target, 22), "open",
                    f"listed port 22 of {target['name']}")
        # traffic without ports is only governed by the ip-ranges
        check(can_ping(ctx, source, target), "ping is blocked by a port-filter")
    finally:
        clear(ctx, target, "ingress")
    wait_until(lambda: tcp_state(ctx, source, target, CLOSED_PORT) == "closed", FILTER_TIMEOUT,
               f"port {CLOSED_PORT} of {target['name']} is unfiltered again")


@suite.test("egress ip-ranges only let the listed destinations out", requires=("ssh",))
def egress_ip_ranges(ctx):
    source, target = require_two(ctx)[:2]
    # the answers of the ssh-session have to leave the source as well
    ssh_client = ctx.ssh.run(source["floating_ip"], "echo ${SSH_CLIENT%% *}")
    check(ssh_client != "", "address of the ssh-client is unknown")
    ctx.log(f"ssh-client as seen by {source['name']}: {ssh_client}")
    try:
        network_filter.add_network_filter_ip_ranges(ctx.api, source["uuid"], "egress",
                                                    [ssh_client])
        wait_ping(ctx, source, target, False,
                  f"{target['name']} is blocked by the egress-filter of {source['name']}")

        network_filter.add_network_filter_ip_ranges(ctx.api, source["uuid"], "egress",
                                                    [target["internal_ip"]])
        wait_ping(ctx, source, target, True,
                  f"{target['name']} is allowed by the egress-filter of {source['name']}")
    finally:
        clear(ctx, source, "egress")
    wait_ping(ctx, source, target, True, f"{source['name']} is unfiltered again")


@suite.test("network-filter, which outlives the test", requires=("virtual_machines",),
            provides=("network_filter_left",))
def left_filter(ctx):
    # The filter allows everything. It is deleted together with its virtual machine, which is
    # checked by the cleanup.
    entry = ctx.state["virtual_machines"][-1]
    result = network_filter.add_network_filter_ip_ranges(ctx.api, entry["uuid"], "egress",
                                                         ["0.0.0.0/0"])
    ctx.state["network_filter_left"] = result["uuid"]
