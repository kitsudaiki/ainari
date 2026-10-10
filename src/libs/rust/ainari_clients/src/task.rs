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

use uuid::Uuid;

use ainari_api_structs::task_structs::TaskResp;
use ainari_common::error::AinariError;

use crate::handle_response;
use crate::prepare_client;

/// Retrieves a task of a sakura-host, for example to follow its progress.
///
/// # Arguments
///
/// * `sakura_address` - The base URL of the Ainari Sakura service.
/// * `token` - Authentication token for the API.
/// * `task_uuid` - UUID of the task to retrieve.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// A `Result` containing the `TaskResp` on success, or an `AinariError` on failure.
pub async fn get_task(
    sakura_address: &str,
    token: &str,
    task_uuid: &Uuid,
    insecure_client: bool,
) -> Result<TaskResp, AinariError> {
    let client = prepare_client(sakura_address, insecure_client);
    let url = format!("{sakura_address}/v1alpha/task/{task_uuid}");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {token}")))
        .send()
        .await;

    handle_response(response, "task", &task_uuid.to_string()).await
}
