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

use ainari_api_structs::proxy_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::prepare_client;
use crate::{handle_empty_response, handle_response};

/**
Creates a new proxy in the Ainari system.

This function communicates with the Torii endpoint to create a proxy that
will forward traffic to the specified target address for a given virtual_machine.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `internal_api_key`: Internal API key for privileged operations
- `virtual_machine_uuid`: UUID of the virtual_machine that will use this proxy
- `target_address`: The address to which the proxy should forward traffic
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the created `ProxyResp` or an `AinariError` if the operation fails.
*/
pub async fn create_proxy(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    virtual_machine_uuid: &Uuid,
    target_address: &str,
    insecure_client: bool,
) -> Result<ProxyResp, AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/proxy/internal");

    let body = ProxyCreateReq {
        target_address: target_address.to_owned(),
        virtual_machine_uuid: *virtual_machine_uuid,
    };
    let json_str = serde_json::to_string(&body).unwrap();

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    // Handle the response and return the result
    let resp: Result<ProxyResp, AinariError> = handle_response(response, "proxy", "").await;
    resp
}

/**
Retrieves information about a specific proxy.

This function fetches details about an existing proxy identified by its UUID.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `proxy_uuid`: UUID of the proxy to retrieve
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the `ProxyResp` with proxy details or an `AinariError` if the operation fails.
*/
pub async fn get_proxy(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    proxy_uuid: &Uuid,
    insecure_client: bool,
) -> Result<ProxyResp, AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/proxy/{proxy_uuid}");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .send()
        .await;

    let resp: Result<ProxyResp, AinariError> =
        handle_response(response, "proxy", &proxy_uuid.to_string()).await;
    resp
}

/**
Lists all proxies available in the system.

This function retrieves a list of all proxies configured in the Ainari system.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the `ProxyResp` with the list of proxies or an `AinariError` if the operation fails.
*/
pub async fn list_proxy(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    insecure_client: bool,
) -> Result<ProxyResp, AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/proxy");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .send()
        .await;

    let resp: Result<ProxyResp, AinariError> = handle_response(response, "proxy", "").await;
    resp
}

/**
Deletes a specific proxy from the system.

This function removes a proxy identified by its UUID from the Ainari system.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `internal_api_key`: Internal API key for privileged operations
- `proxy_uuid`: UUID of the proxy to delete
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` indicating success or an `AinariError` if the operation fails.
*/
pub async fn delete_proxy(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    proxy_uuid: &Uuid,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/proxy/{proxy_uuid}/internal");

    let response = client
        .delete(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_empty_response(response, "proxy", &proxy_uuid.to_string()).await
}
