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
Cold migration of a virtual machine to another sakura-host. The virtual machine is shut down,
its disk is transferred directly between the hosts and it is booted on the new host again. It
keeps its disk, its address, its proxy-port and its packet-filters, which are checked here.

The end of a migration is recognized by the host-resources: the resources are allocated on the
new host, before the migration starts, and released on the old host as its last step. If the
migration fails, it is rolled back and the resources on the new host are released instead.

The suite runs after the tasks, because the proxy-port of a migrated virtual machine leads to its
new host, which doesn't know the tasks of the old host.
"""

import time
import uuid

from ainari_sdk import ainari_exceptions
from ainari_sdk import host
from ainari_sdk import network_filter
from ainari_sdk import virtual_machine

from ainari_test.checks import CheckFailed, SkipTest, check, check_equal, expect_error
from ainari_test.framework import Suite
from ainari_test.waiting import wait_for_vm_state, wait_until

from . import hosts
from .network_filters import CLOSED_PORT, FILTER_TIMEOUT, can_ping, clear, tcp_state
from .power import BOOT_ID, expect_new_boot, run_task
from .virtual_machines import has_room

suite = Suite("migration", "move virtual machines to another sakura-host", depends=("ssh",))

MARKER = "~/ainari-migration-marker"


def target_host(ctx, entry: dict) -> str:
    """
    Another shared sakura-host than the current one of the virtual machine, which has enough free
    resources for it.
    """
    candidates = sorted(item["uuid"] for item in host.list_hosts(ctx.api)["hosts"]
                        if item["uuid"] != entry["host"]
                        and not item["is_host_isolated"]
                        and has_room(ctx, item))
    if not candidates:
        raise SkipTest("no other sakura-host with enough free resources")
    return candidates[0]


def migrate(ctx, entry: dict, target: str, expected_state: str):
    """
    Migrates the virtual machine and waits, until the migration is finished and the virtual
    machine reached the expected state on its new host.
    """
    source = entry["host"]
    before = hosts.usage_by_host(ctx)
    result = virtual_machine.migrate_virtual_machine(ctx.api, entry["uuid"], target)
    ctx.log(f"migrate {entry['name']} from host {source} to host {target}")
    check_equal(result["virtual_machine_uuid"], entry["uuid"], "migrated virtual machine")
    check_equal(result["source_host_uuid"], source, "old host of the migration")
    check_equal(result["target_host_uuid"], target, "new host of the migration")

    start = time.time()
    wait_for_migration(ctx, before, source, target)
    entry["host"] = target
    wait_for_vm_state(ctx.api, entry["uuid"], expected_state, ctx.config.migration_timeout)
    ctx.log(f"{entry['name']} migrated in {time.time() - start:.0f}s")


def wait_for_migration(ctx, before: dict, source: str, target: str):
    """
    Waits, until the resources of the virtual machine moved from the old to the new host.
    """
    expected = {source: hosts.vm_resources(ctx, -1), target: hosts.vm_resources(ctx)}
    end_time = time.time() + ctx.config.migration_timeout
    while True:
        changes = hosts.usage_changes(before, hosts.usage_by_host(ctx))
        if changes == expected:
            return
        # the resources of the new host are allocated, before the migration is accepted, so
        # they are only gone again, if the migration was rolled back
        if target not in changes:
            raise CheckFailed(f"migration to host {target} was rolled back, see the log of "
                              f"hanami (changed resources: {changes})")
        if time.time() >= end_time:
            raise TimeoutError(f"migration to host {target} did not finish within "
                               f"{ctx.config.migration_timeout}s (changed resources: {changes})")
        time.sleep(2.0)


def check_unchanged(ctx, entry: dict):
    """
    Checks, that the virtual machine kept its address and its proxy-port.
    """
    result = virtual_machine.get_virtual_machine(ctx.api, entry["uuid"])
    check_equal(result["internal_ip"], entry["internal_ip"],
                f"internal ip of {entry['name']} after the migration")
    check_equal(result["torii_port"], entry["torii_port"],
                f"proxy-port of {entry['name']} after the migration")


def require_two(ctx) -> list:
    virtual_machines = ctx.state["virtual_machines"]
    if len(virtual_machines) < 2:
        raise SkipTest("requires at least two virtual machines")
    return virtual_machines


@suite.test("migration requires an admin", requires=("virtual_machines", "second_user"))
def requires_admin(ctx):
    entry = ctx.state["virtual_machines"][0]
    expect_error(ainari_exceptions.UnauthorizedException,
                 virtual_machine.migrate_virtual_machine, ctx.state["second_user"],
                 entry["uuid"], entry["host"])


@suite.test("invalid migrations are rejected", requires=("virtual_machines",))
def invalid_migrations(ctx):
    entry = ctx.state["virtual_machines"][0]
    before = hosts.usage_by_host(ctx)

    expect_error(ainari_exceptions.NotFoundException,
                 virtual_machine.migrate_virtual_machine, ctx.api, str(uuid.uuid4()),
                 entry["host"])
    expect_error(ainari_exceptions.NotFoundException,
                 virtual_machine.migrate_virtual_machine, ctx.api, entry["uuid"],
                 str(uuid.uuid4()))
    expect_error(ainari_exceptions.BadRequestException,
                 virtual_machine.migrate_virtual_machine, ctx.api, entry["uuid"], entry["host"])

    check_equal(hosts.usage_changes(before, hosts.usage_by_host(ctx)), {},
                "allocated resources after the rejected migrations")
    check_equal(virtual_machine.get_virtual_machine(ctx.api, entry["uuid"])["vm_state"],
                "RUNNING", f"state of {entry['name']} after the rejected migrations")


@suite.test("migrate a running virtual machine", requires=("ssh",), provides=("migrated",))
def migrate_running(ctx):
    entry = ctx.state["virtual_machines"][0]
    target = target_host(ctx, entry)
    ctx.ssh.refresh_sudo()

    # the guest is shut down gracefully, so the content of the disk survives
    marker = str(uuid.uuid4())
    ctx.ssh.run(entry["floating_ip"], f"echo {marker} > {MARKER}")
    boot_id = ctx.ssh.run(entry["floating_ip"], BOOT_ID)

    source = entry["host"]
    before = hosts.usage_by_host(ctx)
    virtual_machine.migrate_virtual_machine(ctx.api, entry["uuid"], target)
    ctx.log(f"migrate {entry['name']} from host {source} to host {target}")

    # the migration is marked, before it is accepted, so these are rejected for sure. The second
    # migration goes to the new host, because hanami knows the virtual machine on its old host
    # until the end of the migration, so it would reject the old host as its own host first.
    expect_error(ainari_exceptions.ConflictException,
                 virtual_machine.migrate_virtual_machine, ctx.api, entry["uuid"], target)
    expect_error(ainari_exceptions.ConflictException,
                 virtual_machine.delete_virtual_machine, ctx.api, entry["uuid"])
    expect_error(ainari_exceptions.ConflictException,
                 network_filter.add_network_filter_ports, ctx.api, entry["uuid"], "ingress",
                 ["22"])

    wait_for_migration(ctx, before, source, target)
    entry["host"] = target
    wait_for_vm_state(ctx.api, entry["uuid"], "RUNNING", ctx.config.migration_timeout)
    check_unchanged(ctx, entry)

    ctx.ssh.refresh_sudo()
    expect_new_boot(ctx, entry, boot_id)
    check_equal(ctx.ssh.run(entry["floating_ip"], f"cat {MARKER}"), marker,
                f"marker-file of {entry['name']} after the migration")
    check_equal(ctx.ssh.run(entry["floating_ip"], "cat /etc/machine-id"), entry["machine_id"],
                f"machine-id of {entry['name']} after the migration")
    ctx.state["migrated"] = True


@suite.test("migrated virtual machine is managed by its new host", requires=("migrated",))
def managed_by_new_host(ctx):
    # the proxy-port leads to the new host now, which wouldn't know the virtual machine otherwise
    entry = ctx.state["virtual_machines"][0]
    ctx.ssh.refresh_sudo()
    boot_id = ctx.ssh.run(entry["floating_ip"], BOOT_ID)
    run_task(ctx, entry, virtual_machine.reboot_virtual_machine, "VirtualMachineReboot")
    expect_new_boot(ctx, entry, boot_id)


@suite.test("virtual machines reach each other after the migration", requires=("migrated",))
def network_after_migration(ctx):
    migrated, other = require_two(ctx)[:2]
    ctx.ssh.refresh_sudo()
    for source, target in ((migrated, other), (other, migrated)):
        wait_until(lambda source=source, target=target: can_ping(ctx, source, target),
                   FILTER_TIMEOUT, f"{source['name']} reaches {target['name']}")
        check_equal(tcp_state(ctx, source, target, 22), "open",
                    f"ssh-port of {target['name']} from {source['name']}")


@suite.test("packet-filters move with the virtual machine", requires=("migrated",))
def filters_move(ctx):
    migrated, other = require_two(ctx)[:2]
    ctx.ssh.refresh_sudo()
    try:
        network_filter.add_network_filter_ports(ctx.api, migrated["uuid"], "ingress", ["22"])
        wait_until(lambda: tcp_state(ctx, other, migrated, CLOSED_PORT) == "filtered",
                   FILTER_TIMEOUT, f"port {CLOSED_PORT} of {migrated['name']} is filtered")

        # back to a host, which ran the virtual machine before, so its uuid is used there again
        migrate(ctx, migrated, target_host(ctx, migrated), "RUNNING")
        ctx.ssh.refresh_sudo()
        ctx.ssh.wait(migrated["floating_ip"], "true")

        check_equal(tcp_state(ctx, other, migrated, CLOSED_PORT), "filtered",
                    f"port {CLOSED_PORT} of {migrated['name']} after the migration")
        check_equal(tcp_state(ctx, other, migrated, 22), "open",
                    f"listed port 22 of {migrated['name']} after the migration")
    finally:
        clear(ctx, migrated, "ingress")
    wait_until(lambda: tcp_state(ctx, other, migrated, CLOSED_PORT) == "closed",
               FILTER_TIMEOUT, f"port {CLOSED_PORT} of {migrated['name']} is unfiltered again")


@suite.test("stopped virtual machine stays stopped", requires=("migrated",))
def migrate_stopped(ctx):
    entry = ctx.state["virtual_machines"][0]
    ctx.ssh.refresh_sudo()
    boot_id = ctx.ssh.run(entry["floating_ip"], BOOT_ID)
    run_task(ctx, entry, virtual_machine.stop_virtual_machine, "VirtualMachineStop")
    wait_for_vm_state(ctx.api, entry["uuid"], "STOPPED", ctx.config.task_timeout)

    migrate(ctx, entry, target_host(ctx, entry), "STOPPED")
    check_unchanged(ctx, entry)
    check(not ctx.ssh.is_reachable(entry["floating_ip"]),
          f"{entry['name']} was booted by the migration")

    run_task(ctx, entry, virtual_machine.start_virtual_machine, "VirtualMachineStart")
    wait_for_vm_state(ctx.api, entry["uuid"], "RUNNING", ctx.config.task_timeout)
    ctx.ssh.refresh_sudo()
    expect_new_boot(ctx, entry, boot_id)
