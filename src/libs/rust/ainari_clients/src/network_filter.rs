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

use ainari_api_structs::network_filter_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::handle_response;
use crate::prepare_client;

/**
Retrieves the packet filter of a route.

This function fetches the include-lists, which are currently applied to the
traffic of the given route.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `internal_api_key`: Internal API key for privileged operations
- `route_uuid`: UUID of the route, whose filter should be read
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the `FilterResp` with the filter-rules or an `AinariError` if the operation fails.
*/
pub async fn get_filter(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    route_uuid: &Uuid,
    insecure_client: bool,
) -> Result<FilterResp, AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/route/{route_uuid}/filter/internal");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    let resp: Result<FilterResp, AinariError> =
        handle_response(response, "network-filter", &route_uuid.to_string()).await;
    resp
}

/**
Lists the packet filters of all routes.

This function retrieves an overview of the filter-rules of every route, which is
currently known by the gateway.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the `FilterListResponse` with the list of filters or an `AinariError` if the operation fails.
*/
pub async fn list_filter(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    insecure_client: bool,
) -> Result<FilterListResponse, AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/network_filter");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .send()
        .await;

    let resp: Result<FilterListResponse, AinariError> =
        handle_response(response, "network-filter", "").await;
    resp
}

/**
Clears the packet filter of a route.

Dropping both include-lists makes the route unfiltered again, so all traffic to
its destination is passed.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `internal_api_key`: Internal API key for privileged operations
- `route_uuid`: UUID of the route, whose filter should be cleared
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the now empty `FilterResp` or an `AinariError` if the operation fails.
*/
pub async fn clear_filter(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    route_uuid: &Uuid,
    insecure_client: bool,
) -> Result<FilterResp, AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/route/{route_uuid}/filter/internal");

    let response = client
        .delete(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    let resp: Result<FilterResp, AinariError> =
        handle_response(response, "network-filter", &route_uuid.to_string()).await;
    resp
}

/**
Adds IP ranges to the packet filter of a route.

The ranges are added to the include-list of the route, so only traffic from and
to one of the listed addresses is passed.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `internal_api_key`: Internal API key for privileged operations
- `route_uuid`: UUID of the route, whose filter should be extended
- `ranges`: The address-ranges to add, each parsed from a single address, a CIDR subnet or a `first-last` range
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the updated `FilterResp` or an `AinariError` if the operation fails.
*/
pub async fn add_filter_ip_range(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    route_uuid: &Uuid,
    ranges: Vec<IpRangeRule>,
    insecure_client: bool,
) -> Result<FilterResp, AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/route/{route_uuid}/filter/ip_range/internal");

    let body = FilterIpRangeReq { ranges };
    let json_str = serde_json::to_string(&body).unwrap();

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    let resp: Result<FilterResp, AinariError> =
        handle_response(response, "network-filter", &route_uuid.to_string()).await;
    resp
}

/**
Removes IP ranges from the packet filter of a route.

Because a range is matched by its canonical form, it can be removed with any
notation, which describes the same range.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `internal_api_key`: Internal API key for privileged operations
- `route_uuid`: UUID of the route, whose filter should be reduced
- `ranges`: The address-ranges to remove
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the updated `FilterResp` or an `AinariError` if the operation fails.
*/
pub async fn delete_filter_ip_range(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    route_uuid: &Uuid,
    ranges: Vec<IpRangeRule>,
    insecure_client: bool,
) -> Result<FilterResp, AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/route/{route_uuid}/filter/ip_range/internal");

    let body = FilterIpRangeReq { ranges };
    let json_str = serde_json::to_string(&body).unwrap();

    let response = client
        .delete(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    let resp: Result<FilterResp, AinariError> =
        handle_response(response, "network-filter", &route_uuid.to_string()).await;
    resp
}

/**
Adds ports to the packet filter of a route.

The ports are added to the include-list of the route, so only traffic from and to
one of the listed ports is passed.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `internal_api_key`: Internal API key for privileged operations
- `route_uuid`: UUID of the route, whose filter should be extended
- `ports`: The ports to add, each parsed from a single port or a `first-last` range
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the updated `FilterResp` or an `AinariError` if the operation fails.
*/
pub async fn add_filter_port(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    route_uuid: &Uuid,
    ports: Vec<PortRangeRule>,
    insecure_client: bool,
) -> Result<FilterResp, AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/route/{route_uuid}/filter/port/internal");

    let body = FilterPortReq { ports };
    let json_str = serde_json::to_string(&body).unwrap();

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    let resp: Result<FilterResp, AinariError> =
        handle_response(response, "network-filter", &route_uuid.to_string()).await;
    resp
}

/**
Removes ports from the packet filter of a route.

Because a port-range is matched by its canonical form, it can be removed with any
notation, which describes the same range.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `internal_api_key`: Internal API key for privileged operations
- `route_uuid`: UUID of the route, whose filter should be reduced
- `ports`: The ports to remove
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the updated `FilterResp` or an `AinariError` if the operation fails.
*/
pub async fn delete_filter_port(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    route_uuid: &Uuid,
    ports: Vec<PortRangeRule>,
    insecure_client: bool,
) -> Result<FilterResp, AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/route/{route_uuid}/filter/port/internal");

    let body = FilterPortReq { ports };
    let json_str = serde_json::to_string(&body).unwrap();

    let response = client
        .delete(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    let resp: Result<FilterResp, AinariError> =
        handle_response(response, "network-filter", &route_uuid.to_string()).await;
    resp
}
