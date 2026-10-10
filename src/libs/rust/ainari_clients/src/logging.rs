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

use serde::Serialize;
use serde::de::DeserializeOwned;

use ainari_api_structs::log_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::handle_response;
use crate::prepare_client;

/// Sends a new entry to one of the logs of the Omikuji service.
///
/// The internal endpoints of the Omikuji service are authorized by the internal API key alone,
/// so no token of a user is required and the entries can also be sent by background-jobs.
///
/// # Arguments
///
/// * `omikuji_endpoint` - The endpoint configuration for the Omikuji service
/// * `internal_api_key` - The internal API key for authentication
/// * `log_type` - Path-segment of the log, `error_log` or `audit_log`
/// * `entry` - The new entry
/// * `insecure_client` - Whether to use an insecure (non-TLS) client
///
/// # Returns
///
/// A `Result` containing the stored entry if successful, or an `AinariError` if the operation fails.
async fn add_log_entry<Req: Serialize, Resp: DeserializeOwned>(
    omikuji_endpoint: &ainari_config::Endpoint,
    internal_api_key: &Secret,
    log_type: &str,
    entry: &Req,
    insecure_client: bool,
) -> Result<Resp, AinariError> {
    let address = omikuji_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/{log_type}/internal");

    let json_str = serde_json::to_string(entry).unwrap();

    let response = client
        .post(url)
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    handle_response(response, log_type, "").await
}

/// Adds an error-message to the central error-log of the Omikuji service.
///
/// # Arguments
///
/// * `omikuji_endpoint` - The endpoint configuration for the Omikuji service
/// * `internal_api_key` - The internal API key for authentication
/// * `entry` - The new error-message
/// * `insecure_client` - Whether to use an insecure (non-TLS) client
///
/// # Returns
///
/// A `Result` containing the stored entry if successful, or an `AinariError` if the operation fails.
pub async fn add_error_log(
    omikuji_endpoint: &ainari_config::Endpoint,
    internal_api_key: &Secret,
    entry: &ErrorLogAddInternalReq,
    insecure_client: bool,
) -> Result<ErrorLogResp, AinariError> {
    add_log_entry(
        omikuji_endpoint,
        internal_api_key,
        "error_log",
        entry,
        insecure_client,
    )
    .await
}

/// Adds an entry to the central audit-log of the Omikuji service.
///
/// # Arguments
///
/// * `omikuji_endpoint` - The endpoint configuration for the Omikuji service
/// * `internal_api_key` - The internal API key for authentication
/// * `entry` - The new audit-entry
/// * `insecure_client` - Whether to use an insecure (non-TLS) client
///
/// # Returns
///
/// A `Result` containing the stored entry if successful, or an `AinariError` if the operation fails.
pub async fn add_audit_log(
    omikuji_endpoint: &ainari_config::Endpoint,
    internal_api_key: &Secret,
    entry: &AuditLogAddInternalReq,
    insecure_client: bool,
) -> Result<AuditLogResp, AinariError> {
    add_log_entry(
        omikuji_endpoint,
        internal_api_key,
        "audit_log",
        entry,
        insecure_client,
    )
    .await
}
