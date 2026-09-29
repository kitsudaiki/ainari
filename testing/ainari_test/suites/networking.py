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
Traffic between the virtual machines and the movement of floating ip-addresses between them.
The virtual machines can run on different sakura-hosts, so this covers the routing over the
toriis as well.
"""

from ainari_sdk import floating_ip

from ainari_test.checks import SkipTest, check, check_equal
from ainari_test.framework import Suite
from ainari_test.ssh import SshError

suite = Suite("networking", "traffic between virtual machines and moving floating ips",
              depends=("ssh",))


def require_two(ctx) -> list:
    virtual_machines = ctx.state["virtual_machines"]
    if len(virtual_machines) < 2:
        raise SkipTest("requires at least two virtual machines")
    return virtual_machines


@suite.test("ping between the virtual machines", requires=("ssh",))
def ping(ctx):
    virtual_machines = require_two(ctx)
    for source in virtual_machines:
        for target in virtual_machines:
            if source is target:
                continue
            try:
                ctx.ssh.run(source["floating_ip"], f"ping -c 3 -W 2 {target['internal_ip']}")
            except SshError as error:
                check(False, f"{source['name']} can not ping {target['name']} "
                             f"({target['internal_ip']}): {error}")


@suite.test("tcp between the virtual machines", requires=("ssh",))
def tcp(ctx):
    virtual_machines = require_two(ctx)
    for source in virtual_machines:
        for target in virtual_machines:
            if source is target:
                continue
            try:
                ctx.ssh.run(source["floating_ip"],
                            f"timeout 5 bash -c '</dev/tcp/{target['internal_ip']}/22'")
            except SshError as error:
                check(False, f"{source['name']} can not reach port 22 of {target['name']} "
                             f"({target['internal_ip']}): {error}")


@suite.test("gateway is reachable", requires=("ssh",))
def gateway(ctx):
    for entry in ctx.state["virtual_machines"]:
        route = ctx.ssh.run(entry["floating_ip"], "ip -4 route show default")
        check("via" in route, f"{entry['name']} has no default-route: '{route}'")
        ctx.log(f"{entry['name']}: {route}")


@suite.test("detached floating ip-address is unreachable", requires=("ssh",))
def detached_unreachable(ctx):
    entry = ctx.state["virtual_machines"][-1]
    ctx.ssh.refresh_sudo()
    floating_ip.detach_floating_ip(ctx.api, entry["floating_ip_uuid"])
    try:
        ctx.ssh.wait_unreachable(entry["floating_ip"], timeout=60)
    finally:
        floating_ip.attach_floating_ip(ctx.api, entry["floating_ip_uuid"], entry["uuid"])
    output = ctx.ssh.wait(entry["floating_ip"], "cat /etc/machine-id")
    check_equal(output, entry["machine_id"], "machine behind the re-attached floating ip")


@suite.test("swap floating ip-addresses between virtual machines", requires=("ssh",))
def swap(ctx):
    first, second = require_two(ctx)[:2]
    ctx.ssh.refresh_sudo()

    def attach(fip_owner, target):
        result = floating_ip.attach_floating_ip(ctx.api, fip_owner["floating_ip_uuid"],
                                                target["uuid"])
        check_equal(result["internal_ip"], target["internal_ip"],
                    f"internal ip behind {fip_owner['floating_ip']}")

    def expect_behind(address, owner):
        output = ctx.ssh.wait(address, "cat /etc/machine-id",
                              expect=lambda output: output == owner["machine_id"])
        check_equal(output, owner["machine_id"], f"machine behind {address}")

    try:
        # both are detached first, so no virtual machine has two floating ip-addresses at once
        floating_ip.detach_floating_ip(ctx.api, first["floating_ip_uuid"])
        floating_ip.detach_floating_ip(ctx.api, second["floating_ip_uuid"])
        attach(second, first)
        attach(first, second)
        expect_behind(first["floating_ip"], second)
        expect_behind(second["floating_ip"], first)
    finally:
        # restore the original assignment for the following tests
        for entry in (first, second):
            try:
                floating_ip.detach_floating_ip(ctx.api, entry["floating_ip_uuid"])
            except Exception:  # noqa: BLE001
                pass
        for entry in (first, second):
            floating_ip.attach_floating_ip(ctx.api, entry["floating_ip_uuid"], entry["uuid"])

    expect_behind(first["floating_ip"], first)
    expect_behind(second["floating_ip"], second)
