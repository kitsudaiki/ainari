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
The proxies of the torii at the edge, which make the sakura-hosts of the virtual machines
reachable from the outside.
"""

from ainari_sdk import common
from ainari_sdk import proxy
from ainari_sdk import task

from ainari_test.checks import check_equal, check_in, check_keys
from ainari_test.framework import Suite

suite = Suite("proxies", "proxies of the torii towards the sakura-hosts",
              depends=("virtual_machines",))


@suite.test("list proxies", requires=("reserved",))
def list_proxies(ctx):
    proxies = proxy.list_proxys(ctx.api)["proxys"]
    by_vm = {entry["virtual_machine_uuid"]: entry for entry in proxies}
    for entry in ctx.state["reserved"]:
        check_in(entry["uuid"], by_vm, f"proxy of {entry['name']}")
        found = by_vm[entry["uuid"]]
        check_equal(found["port"], entry["torii_port"], f"port of the proxy of {entry['name']}")
        entry["proxy_uuid"] = found["uuid"]
        ctx.log(f"{entry['name']}: port {found['port']} -> {found['target_address']}")

    ports = [entry["port"] for entry in proxies]
    check_equal(len(set(ports)), len(ports), "number of unique proxy-ports")


@suite.test("get proxies", requires=("reserved",))
def get_proxies(ctx):
    for entry in ctx.state["reserved"]:
        check_in("proxy_uuid", entry, f"proxy of {entry['name']} found by the list-test")
        result = proxy.get_proxy(ctx.api, entry["proxy_uuid"])
        check_equal(result["virtual_machine_uuid"], entry["uuid"],
                    "virtual machine of the proxy")
        check_equal(result["port"], entry["torii_port"], "port of the proxy")


@suite.test("sakura-hosts are reachable over the proxies", requires=("reserved",))
def reachable_over_proxy(ctx):
    for entry in ctx.state["reserved"]:
        address = f"{ctx.api.torii_base_address}:{entry['torii_port']}"
        result = common.get_version(ctx.api, address)
        check_keys(result, ["version"], f"version of sakura behind {address}")
        ctx.log(f"{address}: sakura {result['version']}")


@suite.test("tasks are listed over the proxies", requires=("virtual_machines",))
def tasks_over_proxy(ctx):
    for entry in ctx.state["virtual_machines"]:
        tasks = task.list_tasks(ctx.api, entry["torii_port"])["tasks"]
        check_in(entry["create_task"], [item["uuid"] for item in tasks],
                 f"create-task of {entry['name']} in the task-list of its host")
