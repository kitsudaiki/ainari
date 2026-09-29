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
Access to the virtual machines over ssh and their view on their own hardware and network.
"""

from ainari_test.checks import check, check_equal
from ainari_test.framework import Suite

suite = Suite("ssh", "ssh-access and hardware of the virtual machines",
              depends=("floating_ips",))


@suite.test("ssh-access to every virtual machine", requires=("floating_ips",),
            provides=("ssh",))
def ssh_access(ctx):
    ctx.ssh.refresh_sudo()
    for entry in ctx.state["virtual_machines"]:
        ctx.log(f"waiting for ssh on {entry['floating_ip']} ...")
        output = ctx.ssh.wait(entry["floating_ip"], "hostname && uptime")
        for line in output.splitlines():
            ctx.log(f"    {line}")
        # the machine-id identifies the virtual machine behind a floating ip-address later
        entry["machine_id"] = ctx.ssh.run(entry["floating_ip"], "cat /etc/machine-id")
    ctx.state["ssh"] = True


@suite.test("virtual machines are distinct", requires=("ssh",))
def distinct(ctx):
    machine_ids = [entry["machine_id"] for entry in ctx.state["virtual_machines"]]
    check_equal(len(set(machine_ids)), len(machine_ids), "number of unique machine-ids")


@suite.test("number of cores", requires=("ssh",))
def cores(ctx):
    for entry in ctx.state["virtual_machines"]:
        output = ctx.ssh.run(entry["floating_ip"], "nproc")
        check_equal(int(output), ctx.config.number_of_cores, f"cores of {entry['name']}")


@suite.test("size of the memory", requires=("ssh",))
def memory(ctx):
    expected_kib = ctx.config.memory_size * 1024
    for entry in ctx.state["virtual_machines"]:
        output = ctx.ssh.run(entry["floating_ip"], "awk '/MemTotal/ {print $2}' /proc/meminfo")
        # the kernel reserves a part of the memory for itself, so less is reported
        check(0.8 * expected_kib <= int(output) <= expected_kib,
              f"{entry['name']} reports {int(output) // 1024} MiB memory, "
              f"expected about {ctx.config.memory_size} MiB")


@suite.test("size of the root-disk", requires=("ssh",))
def disk(ctx):
    # the root-disk is the image, which is enlarged by the requested disk-size
    minimum = ctx.config.disk_size * 1024 ** 3
    for entry in ctx.state["virtual_machines"]:
        output = ctx.ssh.run(
            entry["floating_ip"],
            "lsblk -bdno SIZE /dev/$(lsblk -no PKNAME $(findmnt -no SOURCE /))")
        check(int(output) >= minimum,
              f"root-disk of {entry['name']} has only {int(output) / 1024 ** 3:.1f} GiB, "
              f"expected at least {ctx.config.disk_size} GiB")
        ctx.log(f"{entry['name']}: root-disk {int(output) / 1024 ** 3:.1f} GiB")


@suite.test("internal ip-address within the virtual machine", requires=("ssh",))
def internal_address(ctx):
    for entry in ctx.state["virtual_machines"]:
        output = ctx.ssh.run(entry["floating_ip"], "hostname -I")
        check(entry["internal_ip"] in output.split(),
              f"{entry['name']} has the addresses '{output}' instead of {entry['internal_ip']}")


@suite.test("root-filesystem is writable", requires=("ssh",))
def writable(ctx):
    for entry in ctx.state["virtual_machines"]:
        output = ctx.ssh.run(entry["floating_ip"],
                             "echo ainari > ~/write-test && cat ~/write-test && rm ~/write-test")
        check_equal(output, "ainari", f"content of the test-file on {entry['name']}")
