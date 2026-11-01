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
Backups and restores of isekai. A marker-file is written before the backup of a virtual machine
and changed afterwards, so the restore can be verified by its content, like for the snapshots.

The restores of whole projects are only checked as dry-run, because they would restore all
virtual machines of the project of the test-user, also the ones, which were created outside of
the test, like by prepare_resources.py.
"""

import dataclasses
import uuid

from ainari_sdk import ainari_exceptions
from ainari_sdk import backup
from ainari_sdk import common
from ainari_sdk import floating_ip
from ainari_sdk import host
from ainari_sdk import image
from ainari_sdk import login
from ainari_sdk import network
from ainari_sdk import project
from ainari_sdk import public_key
from ainari_sdk import secret
from ainari_sdk import task
from ainari_sdk import virtual_machine

from ainari_test.checks import (CheckFailed, SkipTest, check, check_equal, check_in, check_not_in,
                                exists, expect_error)
from ainari_test.framework import Suite
from ainari_test.ssh import SshError
from ainari_test.waiting import wait_for_vm_state, wait_until

from . import hosts

suite = Suite("backups", "backup and restore of virtual machines and projects with isekai",
              depends=("ssh",))

MARKER = "~/ainari-backup-marker"
AFTER_BACKUP = "~/ainari-after-backup"

# resources of a project, which can be created by a restore: kind of the cleanup, list-function,
# key of the list within the response, get-function and delete-function
RESOURCE_KINDS = (
    ("virtual_machine", virtual_machine.list_virtual_machines, "virtual_machines",
     virtual_machine.get_virtual_machine, virtual_machine.delete_virtual_machine),
    ("floating_ip", floating_ip.list_floating_ips, "floating_ips",
     floating_ip.get_floating_ip, floating_ip.delete_floating_ip),
    ("image", image.list_images, "images", image.get_image, image.delete_image),
    ("secret", secret.list_secrets, "secrets", secret.get_secret, secret.delete_secret),
    ("network", network.list_networks, "networks", network.get_network, network.delete_network),
    ("public_key", public_key.list_public_keys, "public_keys",
     public_key.get_public_key, public_key.delete_public_key),
)


def own_project(ctx) -> str:
    return login.validate_token(ctx.api)["context"]["project_id"]


def project_context(ctx, project_id: str):
    """
    Access-context of the admin for another project, so its resources can be listed and deleted.
    """
    token = login.renew_token_for_project_admin(ctx.api, project_id)["access_token"]
    return dataclasses.replace(ctx.api, token=token)


def list_resources(context) -> dict:
    return {kind: {entry["uuid"]: entry for entry in list_function(context)[key]}
            for kind, list_function, key, _, _ in RESOURCE_KINDS}


def register_new_resources(ctx, context, before: dict):
    """
    Registers all resources of the project of the context, which are not in `before`, for the
    cleanup. A restore creates resources, like the images of the restored disks, which are not
    returned by its response. An admin lists the resources of all projects, so `before` has to be
    taken right before the restore.
    """
    registered = {entry.uuid for entry in ctx.cleanup.pending()}
    for kind, list_function, key, get_function, delete_function in RESOURCE_KINDS:
        for entry in list_function(context)[key]:
            entry_uuid = entry["uuid"]
            if entry_uuid in before.get(kind, {}) or entry_uuid in registered:
                continue
            # the internal secrets of images and backups are deleted together with them
            if kind == "secret" and entry.get("owned_by", "user") != "user":
                continue
            label = entry.get("name", entry_uuid)
            ctx.cleanup.add(kind, entry_uuid, label,
                            lambda f=delete_function, u=entry_uuid: f(context, u),
                            lambda f=get_function, u=entry_uuid: exists(f, context, u))
            ctx.log(f"restored {kind} {label} ({entry_uuid})")


def register_backup(ctx, result: dict) -> str:
    backup_uuid = result["backup"]["uuid"]
    ctx.cleanup.add("backup", backup_uuid, result["backup"]["name"],
                    lambda: backup.delete_backup(ctx.api, backup_uuid),
                    lambda: exists(backup.get_backup, ctx.api, backup_uuid))
    return backup_uuid


def wait_for_job(ctx, job_uuid: str) -> dict:
    """
    Waits until the job of isekai is done and checks, that it was finished successfully.
    """
    result = wait_until(
        lambda: _final_job(ctx, job_uuid),
        ctx.config.backup_timeout,
        f"job '{job_uuid}' not finished",
        interval=5.0)
    if result["state"] != "FINISHED":
        raise CheckFailed(f"job '{job_uuid}' ({result['job_type']}) ended in state "
                          f"'{result['state']}' at step '{result['step']}': {result['messages']}")
    return result


def _final_job(ctx, job_uuid: str):
    result = backup.get_job(ctx.api, job_uuid)
    if result["state"] in backup.JOB_FINAL_STATES:
        return result
    return None


def register_unlock(ctx, project_id: str) -> str:
    """
    A failed restore keeps its project locked, which blocks the deletion of all its resources, so
    the lock is removed by the cleanup. Returns the key of the cleanup-entry, which differs from
    the one of the project itself.
    """
    key = f"lock-{project_id}"

    def unlock():
        if project.get_project(ctx.api, project_id).get("lock") is not None:
            project.unlock_project(ctx.api, project_id)

    ctx.cleanup.add("project_lock", key, project_id, unlock)
    return key


def check_backup_task(ctx, entry: dict, task_type: str, backup_uuid: str):
    """
    Checks, that the sakura-host of the virtual machine has a finished task of the given type for
    the backup, so the tasks of the virtual machine show, that it was saved or restored by a backup.
    """
    tasks = [item for item in task.list_tasks(ctx.api, entry["torii_address"])["tasks"]
             if item["task_type"] == task_type and backup_uuid in item["description"]
             and entry["uuid"] in item["description"]]
    check(tasks, f"no task {task_type} of backup {backup_uuid} for {entry['name']}")
    check_equal(tasks[0]["state"], "Finished", f"state of the task {task_type}")


def check_unlocked(ctx, project_id: str, what: str):
    check_equal(project.get_project(ctx.api, project_id).get("lock"), None,
                f"lock of the project {what}")


def planned(plan: dict, key: str, resource_type: str) -> list:
    return [entry["uuid"] for entry in plan[key] if entry["resource_type"] == resource_type]


@suite.test("isekai is reachable", provides=("isekai",))
def isekai_reachable(ctx):
    if not ctx.api.isekai_address:
        raise SkipTest("isekai is not part of the endpoints of miko")
    version = common.get_version(ctx.api, ctx.api.isekai_address)
    ctx.log(f"isekai {version} at {ctx.api.isekai_address}")
    ctx.state["isekai"] = True
    ctx.state["backup_project"] = own_project(ctx)


@suite.test("onsen-host is marked as backup-target", requires=("isekai",),
            provides=("backup_target",))
def backup_target(ctx):
    hosts = host.list_onsen_hosts(ctx.api)["hosts"]
    targets = [entry for entry in hosts if entry["is_backup_target"]]
    check(targets, "no onsen-host is marked as backup-target, it is marked by "
                   "setup_local_stack.sh or prepare_resources.py")
    check(len(targets) < len(hosts), "all onsen-hosts are backup-targets, so there is no onsen "
                                     "for the normal images left")
    target = targets[0]
    check_equal(host.get_onsen_host(ctx.api, target["uuid"])["is_backup_target"], True,
                "backup-target-flag of the onsen-host")
    ctx.log(f"backup-target '{target['name']}' at {target['host_address']}")
    ctx.state["backup_target"] = target["uuid"]


@suite.test("onsen-host with images can not be marked as backup-target",
            requires=("backup_target", "image"))
def target_with_images(ctx):
    normal = [entry for entry in host.list_onsen_hosts(ctx.api)["hosts"]
              if not entry["is_backup_target"]]
    if len(normal) != 1:
        raise SkipTest("the onsen-host, which stores the image, is only known with a single "
                       "normal onsen-host")
    try:
        expect_error(ainari_exceptions.ConflictException, host.set_onsen_host_backup_target,
                     ctx.api, normal[0]["uuid"], True)
    finally:
        if host.get_onsen_host(ctx.api, normal[0]["uuid"])["is_backup_target"]:
            host.set_onsen_host_backup_target(ctx.api, normal[0]["uuid"], False)


@suite.test("invalid backups are rejected", requires=("backup_target", "virtual_machines"))
def invalid_backups(ctx):
    entry = ctx.state["virtual_machines"][0]
    expect_error(ainari_exceptions.BadRequestException, backup.create_virtual_machine_backup,
                 ctx.api, entry["uuid"], ctx.name("backup-short"), "short")
    expect_error(ainari_exceptions.BadRequestException, backup.create_project_backup,
                 ctx.api, ctx.name("backup-short"), "short")
    expect_error(ainari_exceptions.NotFoundException, backup.create_virtual_machine_backup,
                 ctx.api, str(uuid.uuid4()), ctx.name("backup-unknown"),
                 ctx.config.backup_passphrase)
    names = [item["name"] for item in backup.list_backups(ctx.api)["backups"]]
    check_not_in(ctx.name("backup-short"), names, "rejected backup in list")
    check_unlocked(ctx, ctx.state["backup_project"], "after the rejected backups")


@suite.test("write marker-file", requires=("ssh", "backup_target"), provides=("backup_marker",))
def write_marker(ctx):
    entry = ctx.state["virtual_machines"][0]
    ctx.ssh.refresh_sudo()
    marker = str(uuid.uuid4())
    # the backup contains only, what is written to the disk, so the data is synced first
    ctx.ssh.run(entry["floating_ip"], f"echo {marker} > {MARKER} && rm -f {AFTER_BACKUP} && sync")
    ctx.state["backup_marker"] = marker


@suite.test("create backup of a virtual machine", requires=("backup_marker",),
            provides=("vm_backup",))
def create_vm_backup(ctx):
    entry = ctx.state["virtual_machines"][0]
    name = ctx.name("vm-backup")
    result = backup.create_virtual_machine_backup(ctx.api, entry["uuid"], name,
                                                  ctx.config.backup_passphrase)
    backup_uuid = register_backup(ctx, result)
    check_equal(result["backup"]["kind"], "VIRTUAL_MACHINE", "kind of the backup")
    check_equal(result["backup"]["virtual_machine_uuid"], entry["uuid"],
                "virtual machine of the backup")
    check_equal(result["job"]["job_type"], "BACKUP", "type of the job")
    check_equal(result["job"]["backup_uuid"], backup_uuid, "backup of the job")

    wait_for_job(ctx, result["job"]["uuid"])
    data = backup.get_backup(ctx.api, backup_uuid)
    check_equal(data["state"], "READY", "state of the backup")
    check_equal(data["host_uuid"], ctx.state["backup_target"], "backup-target of the backup")
    check(data["file_count"] and data["file_count"] > 0,
          f"number of files of the backup is {data['file_count']}")
    check(data["total_size"] and data["total_size"] > 0,
          f"size of the backup is {data['total_size']}")
    ctx.log(f"backup {backup_uuid}: {data['file_count']} files, {data['total_size']} bytes")
    check_backup_task(ctx, entry, "BackupSave", backup_uuid)
    ctx.state["vm_backup"] = backup_uuid
    ctx.state["vm_backup_job"] = result["job"]["uuid"]


@suite.test("project is unlocked after the backup", requires=("vm_backup",))
def unlocked_after_backup(ctx):
    check_unlocked(ctx, ctx.state["backup_project"], "after the backup")
    check_equal(login.validate_token(ctx.api)["project_locked"], False,
                "lock-state in the token-validation")


@suite.test("get and list backups and jobs", requires=("vm_backup",))
def get_and_list(ctx):
    backup_uuid = ctx.state["vm_backup"]
    job_uuid = ctx.state["vm_backup_job"]
    check_equal(backup.get_backup(ctx.api, backup_uuid)["name"], ctx.name("vm-backup"),
                "name of the backup")
    check_in(backup_uuid, [item["uuid"] for item in backup.list_backups(ctx.api)["backups"]],
             "backup in list")
    listed = [item["uuid"] for item in backup.list_backups(
        ctx.api, project_id=ctx.state["backup_project"])["backups"]]
    check_in(backup_uuid, listed, "backup in list of its project")

    job = backup.get_job(ctx.api, job_uuid)
    check_equal(job["state"], "FINISHED", "state of the job")
    check_equal(job["project_id"], ctx.state["backup_project"], "project of the job")
    jobs = backup.list_jobs(ctx.api, backup_uuid=backup_uuid)["jobs"]
    check_equal([item["uuid"] for item in jobs], [job_uuid], "jobs of the backup")


@suite.test("unknown backup is not found", requires=("isekai",))
def unknown_backup(ctx):
    expect_error(ainari_exceptions.NotFoundException, backup.get_backup, ctx.api,
                 str(uuid.uuid4()))
    expect_error(ainari_exceptions.NotFoundException, backup.get_job, ctx.api, str(uuid.uuid4()))
    expect_error(ainari_exceptions.NotFoundException, backup.restore_backup, ctx.api,
                 str(uuid.uuid4()), ctx.config.backup_passphrase, dry_run=True)


@suite.test("virtual machine keeps running after the backup", requires=("vm_backup",))
def running_after_backup(ctx):
    entry = ctx.state["virtual_machines"][0]
    check_equal(virtual_machine.get_virtual_machine(ctx.api, entry["uuid"])["vm_state"],
                "RUNNING", "state after the backup")
    check_equal(ctx.ssh.run(entry["floating_ip"], f"cat {MARKER}"), ctx.state["backup_marker"],
                "marker-file after the backup")


@suite.test("dry-run of a restore changes nothing", requires=("vm_backup",))
def restore_dry_run(ctx):
    entry = ctx.state["virtual_machines"][0]
    jobs_before = backup.list_jobs(ctx.api, backup_uuid=ctx.state["vm_backup"])["jobs"]
    result = backup.restore_backup(ctx.api, ctx.state["vm_backup"],
                                   ctx.config.backup_passphrase, force=True, dry_run=True)
    check_equal(result["job"], None, "job of a dry-run")
    plan = result["plan"]
    check_equal(plan["target_project_id"], ctx.state["backup_project"], "target of the restore")
    check_equal(plan["create_project"], False, "project is created by the restore")
    # with force every resource keeps its UUID
    check_equal(plan["id_map"], [], "id-map with force")
    check_in(entry["uuid"], planned(plan, "overwritten", "virtual_machine"),
             "virtual machine is overwritten")
    check_equal(plan["deleted"], [], "deleted resources without prune")

    jobs_after = backup.list_jobs(ctx.api, backup_uuid=ctx.state["vm_backup"])["jobs"]
    check_equal(len(jobs_after), len(jobs_before), "number of jobs after the dry-run")
    check_unlocked(ctx, ctx.state["backup_project"], "after the dry-run")


@suite.test("restore with wrong passphrase is rejected", requires=("vm_backup",))
def restore_wrong_passphrase(ctx):
    expect_error(ainari_exceptions.BadRequestException, backup.restore_backup, ctx.api,
                 ctx.state["vm_backup"], "wrong-backup-passphrase", force=True)
    expect_error(ainari_exceptions.BadRequestException, backup.restore_backup, ctx.api,
                 ctx.state["vm_backup"], "wrong-backup-passphrase", dry_run=True)
    # the passphrase is checked, before the project is locked
    check_unlocked(ctx, ctx.state["backup_project"], "after the wrong passphrase")
    jobs = backup.list_jobs(ctx.api, backup_uuid=ctx.state["vm_backup"])["jobs"]
    check_equal([item["job_type"] for item in jobs], ["BACKUP"],
                "jobs of the backup after the rejected restore")


@suite.test("prune without force is rejected", requires=("vm_backup",))
def prune_without_force(ctx):
    expect_error(ainari_exceptions.BadRequestException, backup.restore_backup, ctx.api,
                 ctx.state["vm_backup"], ctx.config.backup_passphrase, prune=True, dry_run=True)


@suite.test("locked project can only be read", requires=("vm_backup",))
def locked_project(ctx):
    project_id = ctx.state["backup_project"]
    reason = f"maintenance of {ctx.name()}"
    project.lock_project(ctx.api, project_id, reason)
    lock_key = register_unlock(ctx, project_id)
    try:
        lock = project.get_project(ctx.api, project_id)["lock"]
        check(lock is not None, "project is not locked")
        check_equal(lock["lock_reason"], reason, "reason of the lock")
        check_equal(login.validate_token(ctx.api)["project_locked"], True,
                    "lock-state in the token-validation")

        # writes are rejected with 423, reads still work
        expect_error(ainari_exceptions.LockedException, secret.generate_secret, ctx.api,
                     ctx.name("locked-secret"))
        secret_names = [item["name"] for item in secret.list_secrets(ctx.api)["secrets"]]
        check_not_in(ctx.name("locked-secret"), secret_names, "secret of the locked project")
        virtual_machine.list_virtual_machines(ctx.api)

        # a backup or restore can't take over the lock of an admin
        expect_error(ainari_exceptions.ConflictException, backup.create_project_backup, ctx.api,
                     ctx.name("locked-backup"), ctx.config.backup_passphrase)
        expect_error(ainari_exceptions.ConflictException, backup.restore_backup, ctx.api,
                     ctx.state["vm_backup"], ctx.config.backup_passphrase, force=True)
    finally:
        project.unlock_project(ctx.api, project_id)
        ctx.cleanup.discard(lock_key)
    check_unlocked(ctx, project_id, "after the unlock")


@suite.test("change the disk after the backup", requires=("vm_backup",),
            provides=("backup_changed",))
def change_disk(ctx):
    entry = ctx.state["virtual_machines"][0]
    ctx.ssh.run(entry["floating_ip"], f"echo changed > {MARKER} && touch {AFTER_BACKUP} && sync")
    ctx.state["backup_changed"] = True


@suite.test("restore virtual machine with force", requires=("backup_changed",))
def restore_with_force(ctx):
    entry = ctx.state["virtual_machines"][0]
    before = list_resources(ctx.api)
    lock_key = register_unlock(ctx, ctx.state["backup_project"])
    result = backup.restore_backup(ctx.api, ctx.state["vm_backup"],
                                   ctx.config.backup_passphrase, force=True)
    check(result["job"] is not None, "restore without job")
    try:
        job = wait_for_job(ctx, result["job"]["uuid"])
    finally:
        # the disk of the backup is restored as snapshot-image of the project
        register_new_resources(ctx, ctx.api, before)
    check_unlocked(ctx, ctx.state["backup_project"], "after the restore")
    ctx.cleanup.discard(lock_key)
    check_equal(job["job_type"], "RESTORE", "type of the job")
    check_equal(job["id_map"], [], "id-map of the restore with force")
    check_backup_task(ctx, entry, "BackupRestore", ctx.state["vm_backup"])

    data = wait_for_vm_state(ctx.api, entry["uuid"], "RUNNING", ctx.config.task_timeout)
    check_equal(data["internal_ip"], entry["internal_ip"], "internal ip after the restore")
    ctx.ssh.refresh_sudo()
    marker = ctx.ssh.wait(entry["floating_ip"], f"cat {MARKER}")
    check_equal(marker, ctx.state["backup_marker"], "marker-file after the restore")
    check_equal(ctx.ssh.run(entry["floating_ip"],
                            f"test -e {AFTER_BACKUP} && echo exists || echo missing"),
                "missing", "file, which was created after the backup")


@suite.test("restore into another project gets new UUIDs", requires=("vm_backup",))
def restore_into_other_project(ctx):
    entry = ctx.state["virtual_machines"][0]
    target_project = ctx.name("restore")
    project.create_project(ctx.api, target_project, f"restore of {ctx.test_id}")
    ctx.cleanup.add("project", target_project, target_project,
                    lambda: project.delete_project(ctx.api, target_project),
                    lambda: exists(project.get_project, ctx.api, target_project))
    target_context = project_context(ctx, target_project)
    lock_key = register_unlock(ctx, target_project)
    before = list_resources(target_context)

    result = backup.restore_backup(ctx.api, ctx.state["vm_backup"],
                                   ctx.config.backup_passphrase,
                                   target_project_id=target_project)
    # the job is awaited first, so a failed check doesn't leave a running restore behind
    try:
        job = wait_for_job(ctx, result["job"]["uuid"])
    finally:
        register_new_resources(ctx, target_context, before)
    check_unlocked(ctx, target_project, "after the restore")
    ctx.cleanup.discard(lock_key)

    plan = result["plan"]
    check_equal(plan["target_project_id"], target_project, "target of the restore")
    check_equal(plan["overwritten"], [], "overwritten resources in a new project")
    mapped = {item["old_uuid"]: item["new_uuid"] for item in plan["id_map"]
              if item["resource_type"] == "virtual_machine"}
    check_in(entry["uuid"], mapped, "virtual machine in the id-map")
    check(mapped[entry["uuid"]] != entry["uuid"], "virtual machine kept its UUID")
    check_in({"resource_type": "virtual_machine", "old_uuid": entry["uuid"],
              "new_uuid": mapped[entry["uuid"]]}, job["id_map"], "id-map of the job")

    restored = wait_for_vm_state(target_context, mapped[entry["uuid"]], "RUNNING",
                                 ctx.config.vm_create_timeout)
    networks = [item["uuid"] for item in network.list_networks(target_context)["networks"]]
    check_in(restored["network_uuid"], networks, "network of the restored virtual machine")
    check(restored["network_uuid"] != ctx.state["network"],
          "restored virtual machine uses the network of the other project")
    # the original virtual machine is untouched
    check_equal(virtual_machine.get_virtual_machine(ctx.api, entry["uuid"])["vm_state"],
                "RUNNING", "state of the original virtual machine")


@suite.test("restore a deleted virtual machine", requires=("vm_backup", "floating_ips"))
def restore_deleted_vm(ctx):
    """
    The virtual machine is deleted and restored from its backup into its own project without
    force, so it gets its UUID, its internal ip-address and its floating ip-address back. The
    restored virtual machine replaces the deleted one in the state, so the following suites, like
    the migration, run with it.
    """
    entry = ctx.state["virtual_machines"][0]
    usage_before_delete = hosts.usage_by_host(ctx)
    virtual_machine.delete_virtual_machine(ctx.api, entry["uuid"])
    wait_until(lambda: not exists(virtual_machine.get_virtual_machine, ctx.api, entry["uuid"]),
               ctx.config.delete_timeout, f"{entry['name']} is deleted")
    # hanami releases the host-resources and the addresses only after the sakura-host deleted the
    # virtual machine, so the restore can only get the old internal ip-address afterwards
    expected = {entry["host"]: hosts.vm_resources(ctx, -1)}
    wait_until(lambda: hosts.usage_changes(usage_before_delete, hosts.usage_by_host(ctx))
               == expected,
               ctx.config.delete_timeout, f"resources of {entry['name']} are released")

    # the deletion only detaches the floating ip-address, which keeps its address reserved. The
    # restore allocates a new floating ip-address, which only gets the old address, if it is free.
    floating_ip.delete_floating_ip(ctx.api, entry["floating_ip_uuid"])
    ctx.cleanup.discard(entry["floating_ip_uuid"])
    wait_until(lambda: not exists(floating_ip.get_floating_ip, ctx.api, entry["floating_ip_uuid"]),
               ctx.config.delete_timeout, f"floating ip {entry['floating_ip']} is deleted")

    before = list_resources(ctx.api)
    usage_before_restore = hosts.usage_by_host(ctx)
    lock_key = register_unlock(ctx, ctx.state["backup_project"])
    result = backup.restore_backup(ctx.api, ctx.state["vm_backup"], ctx.config.backup_passphrase)
    check(result["job"] is not None, "restore without job")
    # the job is awaited first, so a failed check doesn't leave a running restore behind
    try:
        job = wait_for_job(ctx, result["job"]["uuid"])
    finally:
        # the virtual machine keeps its cleanup-entry, a new floating ip-address is registered
        register_new_resources(ctx, ctx.api, before)
    check_unlocked(ctx, ctx.state["backup_project"], "after the restore")
    ctx.cleanup.discard(lock_key)

    # the UUID of a deleted virtual machine is free again for its own project
    check_equal(job["job_type"], "RESTORE", "type of the job")
    check_equal(result["plan"]["overwritten"], [], "overwritten resources without force")
    check_not_in(entry["uuid"], [item["old_uuid"] for item in job["id_map"]
                                 if item["resource_type"] == "virtual_machine"],
                 "deleted virtual machine in the id-map")

    data = wait_for_vm_state(ctx.api, entry["uuid"], "RUNNING", ctx.config.vm_create_timeout)
    check_equal(data["name"], entry["name"], "name after the restore")
    check_equal(data["network_uuid"], ctx.state["network"], "network after the restore")
    check_equal(data["internal_ip"], entry["internal_ip"], "internal ip after the restore")

    # hanami places the restored virtual machine on any shared sakura-host
    changes = hosts.usage_changes(usage_before_restore, hosts.usage_by_host(ctx))
    check_equal(list(changes.values()), [hosts.vm_resources(ctx)],
                "allocated resources of the restored virtual machine")
    entry["host"] = next(iter(changes))
    entry["torii_address"] = data["torii_address"]

    restored_fips = [item for item in floating_ip.list_floating_ips(ctx.api)["floating_ips"]
                     if item["internal_ip"] == entry["internal_ip"]
                     and item["network_uuid"] == ctx.state["network"]]
    check_equal(len(restored_fips), 1, "floating ip-addresses of the restored virtual machine")
    check_equal(restored_fips[0]["floating_ip"], entry["floating_ip"],
                "floating ip after the restore")
    entry["floating_ip_uuid"] = restored_fips[0]["uuid"]
    ctx.log(f"{entry['name']} restored on host {entry['host']} "
            f"({entry['internal_ip']}, {entry['floating_ip']})")

    # reachable from outside over the floating ip-address and with the content of the backup
    ctx.ssh.refresh_sudo()
    marker = ctx.ssh.wait(entry["floating_ip"], f"cat {MARKER}")
    check_equal(marker, ctx.state["backup_marker"], "marker-file after the restore")
    # and reachable from the other virtual machines over the internal network
    for other in ctx.state["virtual_machines"][1:]:
        try:
            ctx.ssh.run(other["floating_ip"], f"ping -c 3 -W 2 {entry['internal_ip']}")
        except SshError:
            check(False, f"{other['name']} can not ping the restored {entry['name']} "
                         f"({entry['internal_ip']})")


@suite.test("backup of the project locks it", requires=("vm_backup",),
            provides=("project_backup",))
def create_project_backup(ctx):
    project_id = ctx.state["backup_project"]
    result = backup.create_project_backup(ctx.api, ctx.name("project-backup"),
                                          ctx.config.backup_passphrase, include_images=False)
    backup_uuid = register_backup(ctx, result)
    job_uuid = result["job"]["uuid"]
    check_equal(result["backup"]["kind"], "PROJECT", "kind of the backup")

    # the project is locked by the job, so it is awaited, as long as the job is running
    locked = wait_until(
        lambda: (project.get_project(ctx.api, project_id).get("lock")
                 or _final_job(ctx, job_uuid)),
        ctx.config.backup_timeout, "project is locked by the backup")
    if "lock_reason" not in locked:
        raise SkipTest("backup was finished, before the lock was seen")
    check_equal(locked["locked_by_job"], job_uuid, "job of the lock")

    expect_error(ainari_exceptions.LockedException, secret.generate_secret, ctx.api,
                 ctx.name("backup-secret"))
    virtual_machine.list_virtual_machines(ctx.api)
    expect_error(ainari_exceptions.ConflictException, backup.create_project_backup, ctx.api,
                 ctx.name("second-backup"), ctx.config.backup_passphrase)
    expect_error(ainari_exceptions.ConflictException, backup.restore_backup, ctx.api,
                 ctx.state["vm_backup"], ctx.config.backup_passphrase, force=True)
    # a backup, which is created right now, can't be deleted
    expect_error(ainari_exceptions.ConflictException, backup.delete_backup, ctx.api, backup_uuid)

    wait_for_job(ctx, job_uuid)
    check_equal(backup.get_backup(ctx.api, backup_uuid)["state"], "READY", "state of the backup")
    check_unlocked(ctx, project_id, "after the backup")
    ctx.state["project_backup"] = backup_uuid


@suite.test("dry-run with prune lists the resources, which are not in the backup",
            requires=("project_backup",))
def prune_dry_run(ctx):
    secret_name = ctx.name("after-backup-secret")
    secret_uuid = secret.generate_secret(ctx.api, secret_name)["uuid"]
    ctx.cleanup.add("secret", secret_uuid, secret_name,
                    lambda: secret.delete_secret(ctx.api, secret_uuid),
                    lambda: exists(secret.get_secret, ctx.api, secret_uuid))

    result = backup.restore_backup(ctx.api, ctx.state["project_backup"],
                                   ctx.config.backup_passphrase, force=True, prune=True,
                                   dry_run=True)
    check_equal(result["job"], None, "job of a dry-run")
    plan = result["plan"]
    check_in(secret_uuid, planned(plan, "deleted", "secret"), "secret created after the backup")
    for entry in ctx.state["virtual_machines"]:
        check_not_in(entry["uuid"], planned(plan, "deleted", "virtual_machine"),
                     f"{entry['name']}, which is part of the backup")
        check_in(entry["uuid"], planned(plan, "overwritten", "virtual_machine"),
                 f"{entry['name']} is overwritten")
    check_unlocked(ctx, ctx.state["backup_project"], "after the dry-run")

    secret.delete_secret(ctx.api, secret_uuid)
    ctx.cleanup.discard(secret_uuid)


@suite.test("scan of the backup-target lists only unknown backups", requires=("vm_backup",))
def scan_backup_target(ctx):
    found = [item["uuid"] for item in
             backup.scan_backup_target(ctx.api, ctx.state["backup_target"])["backups"]]
    check_not_in(ctx.state["vm_backup"], found, "registered backup in the scan")
    if "project_backup" in ctx.state:
        check_not_in(ctx.state["project_backup"], found, "registered backup in the scan")


@suite.test("invalid imports are rejected", requires=("vm_backup",))
def invalid_imports(ctx):
    expect_error((ainari_exceptions.NotFoundException, ainari_exceptions.BadRequestException),
                 backup.import_backup, ctx.api, ctx.state["backup_target"], str(uuid.uuid4()),
                 ctx.config.backup_passphrase)
    # a registered backup can't be imported a second time
    expect_error(ainari_exceptions.ConflictException, backup.import_backup, ctx.api,
                 ctx.state["backup_target"], ctx.state["vm_backup"],
                 ctx.config.backup_passphrase)


@suite.test("backup-target with backups can not be unmarked", requires=("vm_backup",))
def unmark_with_backups(ctx):
    target = ctx.state["backup_target"]
    try:
        expect_error(ainari_exceptions.ConflictException, host.set_onsen_host_backup_target,
                     ctx.api, target, False)
    finally:
        if not host.get_onsen_host(ctx.api, target)["is_backup_target"]:
            host.set_onsen_host_backup_target(ctx.api, target, True)


@suite.test("delete backups", requires=("vm_backup",))
def delete_backups(ctx):
    for key in ("vm_backup", "project_backup"):
        backup_uuid = ctx.state.get(key)
        if backup_uuid is None:
            continue
        backup.delete_backup(ctx.api, backup_uuid)
        ctx.cleanup.discard(backup_uuid)
        expect_error(ainari_exceptions.NotFoundException, backup.get_backup, ctx.api,
                     backup_uuid)
        listed = [item["uuid"] for item in backup.list_backups(ctx.api)["backups"]]
        check_not_in(backup_uuid, listed, "deleted backup in list")
    found = [item["uuid"] for item in
             backup.scan_backup_target(ctx.api, ctx.state["backup_target"])["backups"]]
    check_not_in(ctx.state["vm_backup"], found, "files of the deleted backup on the target")
