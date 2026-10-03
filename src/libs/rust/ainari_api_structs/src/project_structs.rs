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

use std::collections::BTreeMap;

use apistos::ApiComponent;
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

use ainari_common::enums::ProjectRole;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct ProjectCreateReq {
    #[validate(length(min = 4, max = 127))]
    pub id: String,
    #[validate(length(min = 4, max = 127))]
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ProjectResp {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ProjectBasicResp {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ProjectListResp {
    pub projects: Vec<ProjectBasicResp>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ProjectInvitedResp {
    pub project_id: String,
    pub project_role: ProjectRole,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ProjectInvitedListResp {
    pub projects: Vec<ProjectInvitedResp>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ProjectMemberResp {
    pub user_id: String,
    pub project_role: ProjectRole,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ProjectMemberListResp {
    pub members: Vec<ProjectMemberResp>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct ProjectAddUserReq {
    #[validate(length(min = 4, max = 127))]
    pub user_id: String,
    pub project_role: ProjectRole,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ProjectAddUserResp {
    pub user_id: String,
    pub project_id: String,
    pub project_role: ProjectRole,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct ProjectRemoveUserReq {
    #[validate(length(min = 4, max = 127))]
    pub user_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ProjectResourceCountInternalResp {
    /// number of the existing resources of the project, by the type of the resource
    pub resource_counts: BTreeMap<String, i64>,
}
