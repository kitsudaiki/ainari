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
use uuid::Uuid;
use validator::Validate;

/// Number of log-entries, which are listed, if the request doesn't limit it.
pub const DEFAULT_LOG_LIST_LIMIT: u32 = 100;

/// Request of the other components to add an error-message to the error-log of omikuji.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct ErrorLogAddInternalReq {
    #[validate(length(min = 1, max = 65535))]
    pub message: String,
    /// Name of the component, which reports the entry, like `hanami`
    #[validate(length(min = 1, max = 64))]
    pub component_name: String,
    /// Host, where the entry was created
    #[validate(length(max = 256))]
    pub host_name: String,
    /// User, which is related to the entry. Empty, if the entry was not caused by a user.
    #[validate(length(max = 256))]
    pub user_id: String,
    /// Project, which is related to the entry. Empty, if the entry belongs to no project.
    #[validate(length(max = 256))]
    pub project_id: String,
}

/// Optional filters for listing the entries of the error- or audit-log.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct LogEntryListQuery {
    /// Only list the entries of this project
    pub project_id: Option<String>,
    /// Only list the entries of this user
    pub user_id: Option<String>,
    /// Only list the entries of this host
    pub host_name: Option<String>,
    /// Maximum number of entries to return, the newest first. Defaults to 100.
    #[validate(range(min = 1, max = 10000))]
    pub limit: Option<u32>,
}

/// Single entry of the error-log.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ErrorLogResp {
    pub uuid: Uuid,
    pub message: String,
    pub component_name: String,
    pub host_name: String,
    pub user_id: String,
    pub project_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ErrorLogListResp {
    /// The entries, sorted by their creation-time with the newest first
    pub entries: Vec<ErrorLogResp>,
}

/// Request of the other components to add an entry to the audit-log of omikuji.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct AuditLogAddInternalReq {
    /// Type of the audited request, like `POST`
    #[validate(length(min = 1, max = 64))]
    pub request_type: String,
    /// Endpoint, which was called by the audited request
    #[validate(length(min = 1, max = 1024))]
    pub endpoint: String,
    /// Name of the component, which reports the entry, like `hanami`
    #[validate(length(min = 1, max = 64))]
    pub component_name: String,
    /// Host, where the entry was created
    #[validate(length(max = 256))]
    pub host_name: String,
    /// User, which is related to the entry. Empty, if the entry was not caused by a user.
    #[validate(length(max = 256))]
    pub user_id: String,
    /// Project, which is related to the entry. Empty, if the entry belongs to no project.
    #[validate(length(max = 256))]
    pub project_id: String,
}

/// Single entry of the audit-log.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct AuditLogResp {
    pub uuid: Uuid,
    pub request_type: String,
    pub endpoint: String,
    pub component_name: String,
    pub host_name: String,
    pub user_id: String,
    pub project_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct AuditLogListResp {
    /// The entries, sorted by their creation-time with the newest first
    pub entries: Vec<AuditLogResp>,
}
