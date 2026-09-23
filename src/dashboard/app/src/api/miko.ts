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

// Endpoints of the miko, mirroring `src/binaries/miko/src/api/routes/v1alpha.rs`.

import { mikoClient } from "./client";
import type {
    ProjectBasicResp,
    ProjectCreateReq,
    ProjectResp,
    QuotaBasicResp,
    QuotaResp,
    QuotaSetReq,
    UserBasicResp,
    UserCreateReq,
    UserResp,
} from "./types";

//=============================================================================
// project
//=============================================================================

/** `GET /v1alpha/project/admin` */
export async function listProjects(): Promise<ProjectBasicResp[]> {
    const resp = await mikoClient().get("/v1alpha/project/admin");
    return resp.data.projects;
}

/** `GET /v1alpha/project/{project_id}/admin` */
export async function getProject(projectId: string): Promise<ProjectResp> {
    const resp = await mikoClient().get(`/v1alpha/project/${projectId}/admin`);
    return resp.data;
}

/** `POST /v1alpha/project/admin` */
export async function createProject(body: ProjectCreateReq): Promise<ProjectResp> {
    const resp = await mikoClient().post("/v1alpha/project/admin", body);
    return resp.data;
}

/** `DELETE /v1alpha/project/{project_id}/admin` */
export async function deleteProject(projectId: string): Promise<void> {
    await mikoClient().delete(`/v1alpha/project/${projectId}/admin`);
}

//=============================================================================
// user
//=============================================================================

/** `GET /v1alpha/user/admin` */
export async function listUsers(): Promise<UserBasicResp[]> {
    const resp = await mikoClient().get("/v1alpha/user/admin");
    return resp.data.users;
}

/** `GET /v1alpha/user/{user_id}/admin` */
export async function getUser(userId: string): Promise<UserResp> {
    const resp = await mikoClient().get(`/v1alpha/user/${userId}/admin`);
    return resp.data;
}

/** `POST /v1alpha/user/admin` */
export async function createUser(body: UserCreateReq): Promise<UserResp> {
    const resp = await mikoClient().post("/v1alpha/user/admin", body);
    return resp.data;
}

/** `DELETE /v1alpha/user/{user_id}/admin` */
export async function deleteUser(userId: string): Promise<void> {
    await mikoClient().delete(`/v1alpha/user/${userId}/admin`);
}

//=============================================================================
// quota
//=============================================================================

/** `GET /v1alpha/quota` - quota of the user, who is currently logged in. */
export async function getOwnQuota(): Promise<QuotaResp> {
    const resp = await mikoClient().get("/v1alpha/quota");
    return resp.data;
}

/** `GET /v1alpha/quota/admin` */
export async function listQuotas(): Promise<QuotaBasicResp[]> {
    const resp = await mikoClient().get("/v1alpha/quota/admin");
    return resp.data.quotas;
}

/** `GET /v1alpha/quota/{user_id}/admin` */
export async function getQuota(userId: string): Promise<QuotaResp> {
    const resp = await mikoClient().get(`/v1alpha/quota/${userId}/admin`);
    return resp.data;
}

/** `PUT /v1alpha/quota/{user_id}/admin` */
export async function setQuota(userId: string, body: QuotaSetReq): Promise<QuotaResp> {
    const resp = await mikoClient().put(`/v1alpha/quota/${userId}/admin`, body);
    return resp.data;
}
