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

// Endpoints of the omikuji, mirroring `src/binaries/omikuji/src/api/routes/v1alpha.rs`. Only the
// admin-endpoints are used, because the entries are only added by the other components.

import { omikujiClient } from "./client";
import type { AuditLogResp, ErrorLogResp } from "./types";

//=============================================================================
// error-log
//=============================================================================

/** `GET /v1alpha/error_log/admin` - the newest 100 entries, the newest first. */
export async function listErrorLogs(): Promise<ErrorLogResp[]> {
    const resp = await omikujiClient().get("/v1alpha/error_log/admin");
    return resp.data.entries;
}

/** `GET /v1alpha/error_log/{error_log_uuid}/admin` */
export async function getErrorLog(uuid: string): Promise<ErrorLogResp> {
    const resp = await omikujiClient().get(`/v1alpha/error_log/${uuid}/admin`);
    return resp.data;
}

//=============================================================================
// audit-log
//=============================================================================

/** `GET /v1alpha/audit_log/admin` - the newest 100 entries, the newest first. */
export async function listAuditLogs(): Promise<AuditLogResp[]> {
    const resp = await omikujiClient().get("/v1alpha/audit_log/admin");
    return resp.data.entries;
}

/** `GET /v1alpha/audit_log/{audit_log_uuid}/admin` */
export async function getAuditLog(uuid: string): Promise<AuditLogResp> {
    const resp = await omikujiClient().get(`/v1alpha/audit_log/${uuid}/admin`);
    return resp.data;
}
