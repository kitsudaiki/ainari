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
Floating ip-addresses of the virtual machines. The first virtual machine gets its address
attached directly by the create-call, the others by a separate attach-call after the create.
"""

from ainari_sdk import ainari_exceptions
from ainari_sdk import floating_ip

from ainari_test.checks import check, check_equal, check_in, exists, expect_error
from ainari_test.framework import Suite

suite = Suite("floating_ips", "create, attach, get and list floating ip-addresses",
              depends=("virtual_machines",))


def register_floating_ip(ctx, name: str, fip_uuid: str):
    ctx.cleanup.add("floating_ip", fip_uuid, name,
                    lambda: floating_ip.delete_floating_ip(ctx.api, fip_uuid),
                    lambda: exists(floating_ip.get_floating_ip, ctx.api, fip_uuid))


@suite.test("create and attach floating ip-addresses", requires=("virtual_machines",),
            provides=("floating_ips",))
def create_and_attach(ctx):
    for index, entry in enumerate(ctx.state["virtual_machines"]):
        name = f"{entry['name']}-fip"
        if index == 0:
            # create and attach within one call
            result = floating_ip.create_floating_ip(ctx.api, name,
                                                    virtual_machine_uuid=entry["uuid"])
            register_floating_ip(ctx, name, result["uuid"])
        else:
            # only reserve first and attach afterwards
            result = floating_ip.create_floating_ip(ctx.api, name)
            register_floating_ip(ctx, name, result["uuid"])
            check_equal(result["internal_ip"], None, "internal ip of a new floating ip")
            result = floating_ip.attach_floating_ip(ctx.api, result["uuid"], entry["uuid"])

        check_equal(result["internal_ip"], entry["internal_ip"],
                    f"internal ip behind floating ip {result['floating_ip']}")
        check_equal(result["network_uuid"], ctx.state["network"],
                    f"network behind floating ip {result['floating_ip']}")
        entry["floating_ip"] = result["floating_ip"]
        entry["floating_ip_uuid"] = result["uuid"]
        ctx.log(f"{entry['floating_ip']} -> {entry['internal_ip']} ({entry['name']})")
    ctx.state["floating_ips"] = True


@suite.test("get and list floating ip-addresses", requires=("floating_ips",))
def get_and_list(ctx):
    listed = {entry["uuid"]: entry
              for entry in floating_ip.list_floating_ips(ctx.api)["floating_ips"]}
    for entry in ctx.state["virtual_machines"]:
        result = floating_ip.get_floating_ip(ctx.api, entry["floating_ip_uuid"])
        check_equal(result["floating_ip"], entry["floating_ip"], "floating ip")
        check_equal(result["internal_ip"], entry["internal_ip"], "internal ip")
        check_in(entry["floating_ip_uuid"], listed, "floating ip in list")
        check_equal(listed[entry["floating_ip_uuid"]]["internal_ip"], entry["internal_ip"],
                    "internal ip in list")


@suite.test("floating ip-addresses are unique", requires=("floating_ips",))
def unique(ctx):
    addresses = [entry["floating_ip"] for entry in ctx.state["virtual_machines"]]
    check_equal(len(set(addresses)), len(addresses), "number of unique floating ip-addresses")


@suite.test("used floating ip-address can not be reserved again", requires=("floating_ips",))
def already_used(ctx):
    used = ctx.state["virtual_machines"][0]["floating_ip"]
    expect_error(ainari_exceptions.ConflictException, floating_ip.create_floating_ip,
                 ctx.api, ctx.name("duplicate-fip"), floating_ip=used)


@suite.test("floating ip-address outside of the range is rejected")
def out_of_range(ctx):
    # TEST-NET-3, which is never part of the floating-ip-range of a setup
    expect_error(ainari_exceptions.BadRequestException, floating_ip.create_floating_ip,
                 ctx.api, ctx.name("invalid-fip"), floating_ip="203.0.113.77")


@suite.test("detach and re-attach floating ip-address", requires=("floating_ips",))
def detach_and_attach(ctx):
    entry = ctx.state["virtual_machines"][-1]
    result = floating_ip.detach_floating_ip(ctx.api, entry["floating_ip_uuid"])
    check_equal(result.get("internal_ip"), None, "internal ip after detach")
    check_equal(floating_ip.get_floating_ip(ctx.api, entry["floating_ip_uuid"])["internal_ip"],
                None, "internal ip of the detached floating ip")

    result = floating_ip.attach_floating_ip(ctx.api, entry["floating_ip_uuid"], entry["uuid"])
    check_equal(result["internal_ip"], entry["internal_ip"], "internal ip after re-attach")
    check(result["floating_ip"] == entry["floating_ip"],
          "floating ip changed by detach and re-attach")
