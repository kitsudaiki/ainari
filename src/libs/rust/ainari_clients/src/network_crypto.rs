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

use ainari_api_structs::network_crypto_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::prepare_client;
use crate::{handle_empty_response, handle_response};

/**
Registers a new crypto-key on the gateway.

This function installs one AES-256-GCM key for one direction of a
virtual-machine-to-virtual-machine connection. An `egress` key becomes the key
used for new outgoing packets, while an `ingress` key only accepts incoming
traffic, so a rotation never has a gap in which traffic would leave unprotected.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `internal_api_key`: Internal API key for privileged operations
- `direction`: Direction of the key, either `egress` or `ingress`
- `local_ip`: Address of the local virtual_machine of the connection
- `remote_ip`: Address of the remote virtual_machine of the connection
- `peer_gateway_ip`: Address of the gateway, which handles the remote virtual_machine
- `spi`: Security-Parameter-Index, which identifies the security-association of the key
- `key`: The key-material of the security-association
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the created `CryptoKeyResp` or an `AinariError` if the operation fails.
*/
#[allow(clippy::too_many_arguments)]
pub async fn create_crypto_key(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    direction: &str,
    local_ip: &Ipv4Addr,
    remote_ip: &Ipv4Addr,
    peer_gateway_ip: &Ipv4Addr,
    spi: u32,
    key: &Secret,
    insecure_client: bool,
) -> Result<CryptoKeyResp, AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/network_crypto/key/internal");

    let body = CryptoKeyReq {
        direction: direction.to_owned(),
        local_ip: *local_ip,
        remote_ip: *remote_ip,
        peer_gateway_ip: *peer_gateway_ip,
        spi,
        key: key.clone(),
    };
    let json_str = serde_json::to_string(&body).unwrap();

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    let resp: Result<CryptoKeyResp, AinariError> =
        handle_response(response, "crypto-key", "").await;
    resp
}

/**
Lists all crypto-keys of the gateway.

This function retrieves the security-associations, which are currently installed,
without ever exposing their key-material.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `internal_api_key`: Internal API key for privileged operations
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the `CryptoKeyListResp` with the list of keys or an `AinariError` if the operation fails.
*/
pub async fn list_crypto_key(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    insecure_client: bool,
) -> Result<CryptoKeyListResp, AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/network_crypto/key/internal");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    let resp: Result<CryptoKeyListResp, AinariError> =
        handle_response(response, "crypto-key", "").await;
    resp
}

/**
Deletes a crypto-key from the gateway.

The security-association behind the given SPI is removed, which retires the key
after a rotation has moved the traffic to its successor.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `internal_api_key`: Internal API key for privileged operations
- `direction`: Direction of the key, either `egress` or `ingress`
- `spi`: Security-Parameter-Index of the key to delete
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` indicating success or an `AinariError` if the operation fails.
*/
pub async fn delete_crypto_key(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    direction: &str,
    spi: u32,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/network_crypto/key/{direction}/{spi}/internal");

    let response = client
        .delete(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_empty_response(response, "crypto-key", &spi.to_string()).await
}

/**
Enables or disables the encryption of a connection.

Toggling a connection switches the traffic of the virtual-machine-pair between
the encrypted tunnel and the plain datapath.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `internal_api_key`: Internal API key for privileged operations
- `local_ip`: Address of the local virtual_machine of the connection
- `remote_ip`: Address of the remote virtual_machine of the connection
- `peer_gateway_ip`: Address of the gateway of the remote virtual_machine, if the connection is not registered yet
- `enabled`: Whether the connection should be encrypted
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the resulting `CryptoToggleResp` or an `AinariError` if the operation fails.
*/
#[allow(clippy::too_many_arguments)]
pub async fn toggle_crypto(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    local_ip: &Ipv4Addr,
    remote_ip: &Ipv4Addr,
    peer_gateway_ip: Option<Ipv4Addr>,
    enabled: bool,
    insecure_client: bool,
) -> Result<CryptoToggleResp, AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/network_crypto/toggle/internal");

    let body = CryptoToggleReq {
        local_ip: *local_ip,
        remote_ip: *remote_ip,
        peer_gateway_ip,
        enabled,
    };
    let json_str = serde_json::to_string(&body).unwrap();

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    let resp: Result<CryptoToggleResp, AinariError> =
        handle_response(response, "crypto-connection", "").await;
    resp
}

/**
Lists all encrypted connections of the gateway.

This function retrieves every known virtual-machine-pair together with its
encryption-state and the SPI, which is currently used for outgoing traffic.

# Arguments
- `torii_endpoint`: The endpoint configuration for the Torii service
- `token`: Authentication token for accessing the API
- `internal_api_key`: Internal API key for privileged operations
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the `ConnectionListResp` with the list of connections or an `AinariError` if the operation fails.
*/
pub async fn list_connection(
    torii_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    insecure_client: bool,
) -> Result<ConnectionListResp, AinariError> {
    let address = torii_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/network_crypto/connection/internal");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    let resp: Result<ConnectionListResp, AinariError> =
        handle_response(response, "crypto-connection", "").await;
    resp
}
