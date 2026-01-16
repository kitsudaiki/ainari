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

use ainari_api_structs::quota_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;

use crate::handle_response;
use crate::prepare_client;

/// Retrieves the quota information for a specific user from the Miko endpoint.
///
/// This function makes an HTTP GET request to the Miko endpoint's quota API
/// to fetch the current quota status for the specified user. The response
/// is parsed and returned as a `QuotaResp` struct.
///
/// # Arguments
///
/// * `miko_endpoint` - Reference to the Miko endpoint configuration containing the address.
/// * `token` - Authentication token for the API request.
/// * `user_id` - The ID of the user whose quota information is being requested.
/// * `insecure_client` - Boolean flag indicating whether to use an insecure (non-SSL) client.
///
/// # Returns
///
/// * `Result<QuotaResp, AinariError>` - The quota information if successful, or an error if the request fails.
pub async fn get_quota(
    miko_endpoint: &ainari_config::MikoEndpoint,
    token: &String,
    user_id: &str,
    insecure_client: bool,
) -> Result<QuotaResp, AinariError> {
    // Clone the endpoint address to construct the full URL
    let address = miko_endpoint.address.clone();

    // Prepare the HTTP client with the specified security settings
    let client = prepare_client(&address, insecure_client);

    // Construct the full URL for the quota API endpoint
    let url = format!("{address}/v1alpha/quota");

    // Make the HTTP GET request with the authorization header
    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .send()
        .await;

    // Handle the response and parse it into a QuotaResp struct
    let resp: Result<QuotaResp, AinariError> = handle_response(response, "quota", user_id).await;
    resp
}
