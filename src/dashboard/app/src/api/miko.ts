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
    PassphraseChangeAdminReq,
    PassphraseChangeReq,
    ProjectAddUserReq,
    ProjectAddUserResp,
    ProjectBasicResp,
    ProjectCreateReq,
    ProjectInvitedResp,
    ProjectMemberResp,
    ProjectRemoveUserReq,
    ProjectResp,
    QuotaBasicResp,
    QuotaResp,
    QuotaSetReq,
    TokenRenewReq,
    UserBasicResp,
    UserCreateReq,
    UserResp,
    UserSetProjectRoleReq,
    UserSetProjectRoleResp,
    UserTokenResp,
} from "./types";

//=============================================================================
// token
//=============================================================================

/**
 * `PUT /v1alpha/token` - new token for the user of the current token. With a project-id, the
 * new token is created for this project, which switches the project without a new login.
 */
export async function renewToken(body: TokenRenewReq): Promise<UserTokenResp> {
    const resp = await mikoClient().put("/v1alpha/token", body);
    return resp.data;
}

//=============================================================================
// passphrase
//=============================================================================

/** `PUT /v1alpha/passphrase` - change the passphrase of the user, who is currently logged in. */
export async function changePassphrase(body: PassphraseChangeReq): Promise<void> {
    await mikoClient().put("/v1alpha/passphrase", body);
}

/** `PUT /v1alpha/passphrase/admin` - set a new passphrase for any user. */
export async function changePassphraseAdmin(body: PassphraseChangeAdminReq): Promise<void> {
    await mikoClient().put("/v1alpha/passphrase/admin", body);
}

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

/** `GET /v1alpha/project/{project_id}/users/admin` */
export async function listUsersInProject(projectId: string): Promise<ProjectMemberResp[]> {
    const resp = await mikoClient().get(`/v1alpha/project/${projectId}/users/admin`);
    return resp.data.members;
}

/** `POST /v1alpha/project/{project_id}/add_user/admin` */
export async function addUserToProject(
    projectId: string,
    body: ProjectAddUserReq,
): Promise<ProjectAddUserResp> {
    const resp = await mikoClient().post(`/v1alpha/project/${projectId}/add_user/admin`, body);
    return resp.data;
}

/** `POST /v1alpha/project/{project_id}/remove_user/admin` */
export async function removeUserFromProject(
    projectId: string,
    body: ProjectRemoveUserReq,
): Promise<void> {
    await mikoClient().post(`/v1alpha/project/${projectId}/remove_user/admin`, body);
}

//=============================================================================
// user
//=============================================================================

/** `GET /v1alpha/user/user_projects` - projects of the user, who is currently logged in. */
export async function listInvitedProjects(): Promise<ProjectInvitedResp[]> {
    const resp = await mikoClient().get("/v1alpha/user/user_projects");
    return resp.data.projects;
}

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

/** `PUT /v1alpha/user/{user_id}/set_project_role/admin` */
export async function setProjectRole(
    userId: string,
    body: UserSetProjectRoleReq,
): Promise<UserSetProjectRoleResp> {
    const resp = await mikoClient().put(`/v1alpha/user/${userId}/set_project_role/admin`, body);
    return resp.data;
}

//=============================================================================
// quota
//=============================================================================

/** `GET /v1alpha/quota` - quota of the project, for which the user is currently logged in. */
export async function getOwnQuota(): Promise<QuotaResp> {
    const resp = await mikoClient().get("/v1alpha/quota");
    return resp.data;
}

/** `GET /v1alpha/quota/admin` */
export async function listQuotas(): Promise<QuotaBasicResp[]> {
    const resp = await mikoClient().get("/v1alpha/quota/admin");
    return resp.data.quotas;
}

/** `GET /v1alpha/quota/{project_id}/admin` */
export async function getQuota(projectId: string): Promise<QuotaResp> {
    const resp = await mikoClient().get(`/v1alpha/quota/${projectId}/admin`);
    return resp.data;
}

/** `PUT /v1alpha/quota/{project_id}/admin` */
export async function setQuota(projectId: string, body: QuotaSetReq): Promise<QuotaResp> {
    const resp = await mikoClient().put(`/v1alpha/quota/${projectId}/admin`, body);
    return resp.data;
}
