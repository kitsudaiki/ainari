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

use ainari_api_structs::route_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::prepare_client;
use crate::{handle_empty_response, handle_response};

/**
Registers a new route on the gateway.

This function communicates with the Torii endpoint to add a route, which directs
the traffic for the destination address to the given interface within the eBPF
datapath.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `internal_api_key`: Internal API key for privileged operations
- `route`: Definition of the route, which should be registered
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the created `RouteResp` or an `AinariError` if the operation fails.
*/
pub async fn create_route(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    route: &RouteReq,
    insecure_client: bool,
) -> Result<RouteResp, AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/route/internal");

    let json_str = serde_json::to_string(route).unwrap();

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    let resp: Result<RouteResp, AinariError> = handle_response(response, "route", "").await;
    resp
}

/**
Lists all routes of the gateway.

This function retrieves all dynamically configured routes, which currently manage
the packet flow of the eBPF datapath.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `internal_api_key`: Internal API key for privileged operations
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the `RouteListResp` with the list of routes or an `AinariError` if the operation fails.
*/
pub async fn list_route(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    insecure_client: bool,
) -> Result<RouteListResp, AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/route/internal");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    let resp: Result<RouteListResp, AinariError> = handle_response(response, "route", "").await;
    resp
}

/**
Updates an existing route of the gateway.

Because the update of the eBPF map is atomic on kernel-level, active connections
pivot to the new target without dropping packets.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `internal_api_key`: Internal API key for privileged operations
- `route_uuid`: UUID of the route to update
- `route`: New definition of the route
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the updated `RouteResp` or an `AinariError` if the operation fails.
*/
pub async fn update_route(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    route_uuid: &Uuid,
    route: &RouteReq,
    insecure_client: bool,
) -> Result<RouteResp, AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/route/{route_uuid}/internal");

    let json_str = serde_json::to_string(route).unwrap();

    let response = client
        .put(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    let resp: Result<RouteResp, AinariError> =
        handle_response(response, "route", &route_uuid.to_string()).await;
    resp
}

/**
Deletes a route from the gateway.

The packet filter guarding the route dies with it, and an encrypted route also
loses its fail-closed block policies.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `internal_api_key`: Internal API key for privileged operations
- `route_uuid`: UUID of the route to delete
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` indicating success or an `AinariError` if the operation fails.
*/
pub async fn delete_route(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    route_uuid: &Uuid,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/route/{route_uuid}/internal");

    let response = client
        .delete(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_empty_response(response, "route", &route_uuid.to_string()).await
}
