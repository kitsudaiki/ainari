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

from typing import Optional

from . import ainari_request
from .access_context import AccessContext

import time

# final states of a job of isekai
JOB_FINAL_STATES = ("FINISHED", "ERROR")


def create_project_backup(context: AccessContext,
                          name: str,
                          passphrase: str,
                          stop_vms: bool = False,
                          include_images: bool = True) -> dict:
    """
    Creates a backup of the whole project of the token. The backup is created by a job in the
    background, so the response contains the backup and the job, which can be polled. The project
    is locked, until the job is done.

    The backup is encrypted with a key, which is derived from the passphrase. The passphrase is
    not stored, so the backup can't be restored without it.
    """
    path = "/v1alpha/backup"
    json_body = {
        "name": name,
        "passphrase": passphrase,
        "stop_vms": stop_vms,
        "include_images": include_images,
    }
    return ainari_request.send_post_request(context,
                                            context.isekai_address,
                                            path,
                                            json_body)


def create_virtual_machine_backup(context: AccessContext,
                                  virtual_machine_uuid: str,
                                  name: str,
                                  passphrase: str,
                                  stop_vm: bool = False) -> dict:
    """
    Creates a backup of a single virtual machine with its root-disk, public-key, network and
    packet-filters. Like the backup of a project, it is created by a job in the background.
    """
    path = "/v1alpha/backup/virtual_machine"
    json_body = {
        "virtual_machine_uuid": virtual_machine_uuid,
        "name": name,
        "passphrase": passphrase,
        "stop_vm": stop_vm,
    }
    return ainari_request.send_post_request(context,
                                            context.isekai_address,
                                            path,
                                            json_body)


def get_backup(context: AccessContext,
               backup_uuid: str) -> dict:
    path = f"/v1alpha/backup/{backup_uuid}"
    return ainari_request.send_get_request(context,
                                           context.isekai_address,
                                           path,
                                           "")


def list_backups(context: AccessContext,
                 project_id: Optional[str] = None) -> dict:
    """
    Lists the backups of the project of the token. Admins see the backups of all projects, which
    can be restricted to one project with `project_id`.
    """
    path = "/v1alpha/backup"
    values = f"project_id={project_id}" if project_id else ""
    return ainari_request.send_get_request(context,
                                           context.isekai_address,
                                           path,
                                           values)


def delete_backup(context: AccessContext,
                  backup_uuid: str):
    """
    Deletes a backup together with all its files. A backup, which is created or restored right
    now, can't be deleted.
    """
    path = f"/v1alpha/backup/{backup_uuid}"
    ainari_request.send_delete_request(context,
                                       context.isekai_address,
                                       path,
                                       "")


def delete_all_backups(context: AccessContext):
    body = list_backups(context)["backups"]
    for entry in body:
        delete_backup(context, entry["uuid"])


def restore_backup(context: AccessContext,
                   backup_uuid: str,
                   passphrase: str,
                   force: bool = False,
                   prune: bool = False,
                   target_project_id: Optional[str] = None,
                   start_vms: bool = False,
                   include_dependencies: bool = True,
                   dry_run: bool = False) -> dict:
    """
    Restores a backup into its own project or into `target_project_id`. The response contains the
    plan of the restore and the job, which restores the backup in the background. With `dry_run`
    only the plan is returned, without a job, and nothing is changed.

    Without `force`, resources, whose UUID is already used, get a new UUID. With `force`, existing
    resources are overwritten. `prune` deletes all resources of the project, which are not part
    of the backup, and is only allowed together with `force`.
    """
    path = f"/v1alpha/backup/{backup_uuid}/restore"
    json_body = {
        "passphrase": passphrase,
        "force": force,
        "prune": prune,
        "start_vms": start_vms,
        "include_dependencies": include_dependencies,
        "dry_run": dry_run,
    }
    if target_project_id is not None:
        json_body["target_project_id"] = target_project_id
    return ainari_request.send_post_request(context,
                                            context.isekai_address,
                                            path,
                                            json_body)


def scan_backup_target(context: AccessContext,
                       onsen_host_uuid: str) -> dict:
    """
    Lists the backups on a backup-target, which are not known yet, because they were copied onto
    it from outside, like from another deployment for a disaster-recovery. Admin only.
    """
    path = "/v1alpha/backup/scan/admin"
    values = f"onsen_host_uuid={onsen_host_uuid}"
    return ainari_request.send_get_request(context,
                                           context.isekai_address,
                                           path,
                                           values)


def import_backup(context: AccessContext,
                  onsen_host_uuid: str,
                  backup_uuid: str,
                  passphrase: str,
                  name: Optional[str] = None) -> dict:
    """
    Imports a backup, which was found by `scan_backup_target`, so it can be restored like every
    other backup. The passphrase is checked, before the backup is registered. Admin only.
    """
    path = "/v1alpha/backup/import/admin"
    json_body = {
        "onsen_host_uuid": onsen_host_uuid,
        "backup_uuid": backup_uuid,
        "passphrase": passphrase,
    }
    if name is not None:
        json_body["name"] = name
    return ainari_request.send_post_request(context,
                                            context.isekai_address,
                                            path,
                                            json_body)


def get_job(context: AccessContext,
            job_uuid: str) -> dict:
    path = f"/v1alpha/job/{job_uuid}"
    return ainari_request.send_get_request(context,
                                           context.isekai_address,
                                           path,
                                           "")


def list_jobs(context: AccessContext,
              backup_uuid: Optional[str] = None) -> dict:
    """
    Lists the backup- and restore-jobs, optionally only the ones of a single backup.
    """
    path = "/v1alpha/job"
    values = f"backup_uuid={backup_uuid}" if backup_uuid else ""
    return ainari_request.send_get_request(context,
                                           context.isekai_address,
                                           path,
                                           values)


def wait_for_job_finished(context: AccessContext,
                          job_uuid: str,
                          time_interval: float = 2.0) -> dict:
    """
    Waits until the job reached one of its final states `FINISHED` or `ERROR` and returns the job,
    so the caller can check its state and messages.
    """
    while True:
        result = get_job(context, job_uuid)
        # in case that the job is already done, an unnecessary sleep should be avoided
        if result["state"] in JOB_FINAL_STATES:
            return result
        time.sleep(time_interval)
