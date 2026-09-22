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

use ainari_api_structs::network_interface_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::handle_response;
use crate::prepare_client;

/**
Configures a network-interface of the gateway.

This function assigns an address to the given interface and brings it up or down.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `internal_api_key`: Internal API key for privileged operations
- `iface_name`: Name of the interface to configure
- `ip_cidr`: Address of the interface in CIDR notation, or `None` to leave it unaddressed
- `up`: Whether the interface should be brought up
- `vni`: Tenant the interface belongs to
- `fip_port`: Whether the floating-ips are translated on this interface. Only a port, which
  faces the outside world, may be one.
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the resulting `IfaceConfigResp` or an `AinariError` if the operation fails.
*/
#[allow(clippy::too_many_arguments)]
pub async fn config_interface(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    iface_name: &str,
    ip_cidr: Option<String>,
    up: bool,
    vni: u32,
    fip_port: bool,
    insecure_client: bool,
) -> Result<IfaceConfigResp, AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/network_interface/config/internal");

    let body = IfaceConfigReq {
        iface_name: iface_name.to_owned(),
        ip_cidr,
        up,
        vni,
        fip_port,
    };
    let json_str = serde_json::to_string(&body).unwrap();

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    let resp: Result<IfaceConfigResp, AinariError> =
        handle_response(response, "network-interface", iface_name).await;
    resp
}

/**
Registers a TAP-device on the gateway.

The TAP-device connects a virtual_machine to the eBPF datapath, so the traffic of
the virtual_machine can be routed and filtered by the gateway.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `internal_api_key`: Internal API key for privileged operations
- `tap_name`: Name of the TAP-device to register
- `vni`: Tenant the TAP-device and everything behind it belongs to
- `vm_mac`: MAC-address of the virtual_machine behind the TAP-device, if it is already known
- `vm_ip`: Address of the virtual_machine behind the TAP-device, if it is already known
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the resulting `TapResp` or an `AinariError` if the operation fails.
*/
#[allow(clippy::too_many_arguments)]
pub async fn register_tap(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    tap_name: &str,
    vni: u32,
    vm_mac: Option<String>,
    vm_ip: Option<Ipv4Addr>,
    insecure_client: bool,
) -> Result<TapResp, AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/network_interface/tap/internal");

    let body = TapReq {
        tap_name: tap_name.to_owned(),
        vni,
        vm_mac,
        vm_ip,
    };
    let json_str = serde_json::to_string(&body).unwrap();

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    let resp: Result<TapResp, AinariError> =
        handle_response(response, "tap-device", tap_name).await;
    resp
}
