// Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Endpoints of the isekai, mirroring `src/binaries/isekai/src/api/routes/v1alpha.rs`.

import { isekaiClient } from "./client";
import type {
    BackupImportReq,
    BackupJobResp,
    BackupResp,
    BackupRestoreReq,
    BackupScanEntry,
    JobResp,
    ProjectBackupCreateReq,
    RestoreResp,
    VirtualMachineBackupCreateReq,
} from "./types";

//=============================================================================
// backup
//=============================================================================

/**
 * `POST /v1alpha/backup`
 *
 * Creates a backup of the whole project of the token by a job in the background. The project is
 * locked, until the job is done.
 */
export async function createProjectBackup(body: ProjectBackupCreateReq): Promise<BackupJobResp> {
    const resp = await isekaiClient().post("/v1alpha/backup", body);
    return resp.data;
}

/** `POST /v1alpha/backup/virtual_machine` - backup of a single virtual-machine. */
export async function createVirtualMachineBackup(
    body: VirtualMachineBackupCreateReq,
): Promise<BackupJobResp> {
    const resp = await isekaiClient().post("/v1alpha/backup/virtual_machine", body);
    return resp.data;
}

/** `GET /v1alpha/backup` - the backups of the project of the token, the newest first. */
export async function listBackups(): Promise<BackupResp[]> {
    const resp = await isekaiClient().get("/v1alpha/backup");
    return resp.data.backups;
}

/** `GET /v1alpha/backup/{backup_uuid}` */
export async function getBackup(uuid: string): Promise<BackupResp> {
    const resp = await isekaiClient().get(`/v1alpha/backup/${uuid}`);
    return resp.data;
}

/** `DELETE /v1alpha/backup/{backup_uuid}` - deletes the backup together with all its files. */
export async function deleteBackup(uuid: string): Promise<void> {
    await isekaiClient().delete(`/v1alpha/backup/${uuid}`);
}

/**
 * `POST /v1alpha/backup/{backup_uuid}/restore`
 *
 * Returns the plan of the restore and, if it is no dry-run, the job, which restores the backup.
 */
export async function restoreBackup(uuid: string, body: BackupRestoreReq): Promise<RestoreResp> {
    const resp = await isekaiClient().post(`/v1alpha/backup/${uuid}/restore`, body);
    return resp.data;
}

//=============================================================================
// import of foreign backups (admin)
//=============================================================================

/** `GET /v1alpha/backup/scan/admin` - backups on a backup-target, which are not known yet. */
export async function scanBackupTarget(onsenHostUuid: string): Promise<BackupScanEntry[]> {
    const resp = await isekaiClient().get("/v1alpha/backup/scan/admin", {
        params: { onsen_host_uuid: onsenHostUuid },
    });
    return resp.data.backups;
}

/** `POST /v1alpha/backup/import/admin` - registers a backup, which was found by the scan. */
export async function importBackup(body: BackupImportReq): Promise<BackupResp> {
    const resp = await isekaiClient().post("/v1alpha/backup/import/admin", body);
    return resp.data;
}

//=============================================================================
// job
//=============================================================================

/** `GET /v1alpha/job` - the backup- and restore-jobs, optionally only the ones of a backup. */
export async function listJobs(backupUuid?: string): Promise<JobResp[]> {
    const resp = await isekaiClient().get("/v1alpha/job", {
        params: backupUuid ? { backup_uuid: backupUuid } : {},
    });
    return resp.data.jobs;
}

/** `GET /v1alpha/job/{job_uuid}` */
export async function getJob(uuid: string): Promise<JobResp> {
    const resp = await isekaiClient().get(`/v1alpha/job/${uuid}`);
    return resp.data;
}
