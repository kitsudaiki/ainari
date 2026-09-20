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

use std::net::Ipv4Addr;
use uuid::Uuid;

use ainari_api_structs::floating_ip_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::prepare_client;
use crate::{handle_empty_response, handle_response};

/**
Registers a new floating-ip on the gateway.

This function communicates with the Torii endpoint to provision a floating IP and
register its SNAT/DNAT rules in the eBPF datapath, which makes the given internal
address reachable from the outside.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `internal_api_key`: Internal API key for privileged operations
- `name`: Name of the floating-ip
- `network_uuid`: UUID of the network the floating-ip belongs to
- `floating_ip`: The external address, which is mapped to the internal address
- `internal_ip`: The private address behind the floating-ip
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the created `FloatingIpInternalResp` or an `AinariError` if the operation fails.
*/
#[allow(clippy::too_many_arguments)]
pub async fn create_floating_ip(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    name: &str,
    network_uuid: &Uuid,
    floating_ip: &Ipv4Addr,
    internal_ip: &Ipv4Addr,
    insecure_client: bool,
) -> Result<FloatingIpInternalResp, AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/floating_ip/internal");

    let body = FloatingIpInternalCreateReq {
        name: name.to_owned(),
        network_uuid: *network_uuid,
        floating_ip: *floating_ip,
        internal_ip: *internal_ip,
    };
    let json_str = serde_json::to_string(&body).unwrap();

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    let resp: Result<FloatingIpInternalResp, AinariError> =
        handle_response(response, "floating-ip", "").await;
    resp
}

/**
Deletes a floating-ip from the gateway.

This function removes the NAT configuration of the given floating-ip, which
terminates the external access to the internal address behind it.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `internal_api_key`: Internal API key for privileged operations
- `floating_ip`: The external address, whose NAT configuration should be removed
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` indicating success or an `AinariError` if the operation fails.
*/
pub async fn delete_floating_ip(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    floating_ip: &Ipv4Addr,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/floating_ip/{floating_ip}/internal");

    let response = client
        .delete(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_empty_response(response, "floating-ip", &floating_ip.to_string()).await
}
