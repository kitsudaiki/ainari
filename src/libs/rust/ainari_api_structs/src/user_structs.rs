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

use apistos::ApiComponent;
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

use ainari_common::enums::ProjectRole;
use ainari_common::secret::Secret;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct UserCreateReq {
    #[validate(length(min = 4, max = 127))]
    pub id: String,
    #[validate(length(min = 4, max = 127))]
    pub name: String,
    #[validate(length(min = 8, max = 4096))]
    pub passphrase: Secret,
    pub is_admin: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct UserResp {
    pub id: String,
    pub name: String,
    pub is_admin: String,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct UserBasicResp {
    pub id: String,
    pub name: String,
    pub is_admin: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct UserListResp {
    pub users: Vec<UserBasicResp>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct UserSetProjectRoleReq {
    #[validate(length(min = 4, max = 127))]
    pub project_id: String,
    pub project_role: ProjectRole,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct UserSetProjectRoleResp {
    pub user_id: String,
    pub project_id: String,
    pub project_role: ProjectRole,
}
