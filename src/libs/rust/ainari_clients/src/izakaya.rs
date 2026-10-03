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

//! Clients of the izakaya.
//!
//! Only services talk to the izakaya, which authorizes them by the internal API-key alone, so the
//! requests carry no token of a user.

use serde::Serialize;

use ainari_api_structs::mls_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::prepare_client;
use crate::{handle_empty_response, handle_response};

/// Sends a JSON-body with a POST-request to an internal endpoint of the izakaya.
///
/// # Arguments
/// * `izakaya_endpoint` - The endpoint configuration for the Izakaya service
/// * `internal_api_key` - Internal API key for privileged operations
/// * `path` - Path of the endpoint below `/v1alpha`
/// * `body` - The body of the request
/// * `insecure_client` - Whether to use an insecure (HTTP) client or secure (HTTPS) client
///
/// # Returns
/// The response of the request
async fn post_json(
    izakaya_endpoint: &ainari_config::Endpoint,
    internal_api_key: &Secret,
    path: &str,
    body: &impl Serialize,
    insecure_client: bool,
) -> Result<
    awc::ClientResponse<actix_web::dev::Decompress<actix_web::dev::Payload>>,
    awc::error::SendRequestError,
> {
    let address = izakaya_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/{path}");
    let json_str = serde_json::to_string(body).unwrap();

    client
        .post(url)
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await
}

/**
Uploads MLS key-packages of a client to the izakaya.

Other clients claim them later to invite the client into their groups.

# Arguments
- `izakaya_endpoint`: The endpoint configuration for the Izakaya service
- `internal_api_key`: Internal API key for privileged operations
- `client_id`: Identity of the client, which owns the key-packages
- `key_packages`: Base64-encoded TLS-serialized key-packages
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the `KeyPackageUploadResp` or an `AinariError` if the operation fails.
*/
pub async fn upload_key_packages(
    izakaya_endpoint: &ainari_config::Endpoint,
    internal_api_key: &Secret,
    client_id: &str,
    key_packages: Vec<String>,
    insecure_client: bool,
) -> Result<KeyPackageUploadResp, AinariError> {
    let body = KeyPackageUploadReq {
        client_id: client_id.to_string(),
        key_packages,
    };
    let response = post_json(
        izakaya_endpoint,
        internal_api_key,
        "key_package/internal",
        &body,
        insecure_client,
    )
    .await;

    handle_response(response, "key-package", "").await
}

/**
Claims one MLS key-package of a client from the izakaya.

A key-package can only be used once, so it is removed from the izakaya with the claim.

# Arguments
- `izakaya_endpoint`: The endpoint configuration for the Izakaya service
- `internal_api_key`: Internal API key for privileged operations
- `client_id`: Identity of the client, whose key-package is claimed
- `signature_key`: Base64-encoded signature-key, which the key-package has to have
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the claimed `KeyPackageResp` or an `AinariError` if the operation fails.
*/
pub async fn claim_key_package(
    izakaya_endpoint: &ainari_config::Endpoint,
    internal_api_key: &Secret,
    client_id: &str,
    signature_key: Option<String>,
    insecure_client: bool,
) -> Result<KeyPackageResp, AinariError> {
    let body = KeyPackageClaimReq { signature_key };
    let response = post_json(
        izakaya_endpoint,
        internal_api_key,
        &format!("key_package/{client_id}/claim/internal"),
        &body,
        insecure_client,
    )
    .await;

    handle_response(response, "key-package", client_id).await
}

/**
Counts the MLS key-packages of a client, which are still available in the izakaya.

# Arguments
- `izakaya_endpoint`: The endpoint configuration for the Izakaya service
- `internal_api_key`: Internal API key for privileged operations
- `client_id`: Identity of the client, whose key-packages are counted
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the `KeyPackageCountResp` or an `AinariError` if the operation fails.
*/
pub async fn get_key_package_count(
    izakaya_endpoint: &ainari_config::Endpoint,
    internal_api_key: &Secret,
    client_id: &str,
    insecure_client: bool,
) -> Result<KeyPackageCountResp, AinariError> {
    let address = izakaya_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/key_package/{client_id}/count/internal");

    let response = client
        .get(url)
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_response(response, "key-package", client_id).await
}

/**
Hands a MLS-message over to the izakaya, which delivers it to all of its recipients.

# Arguments
- `izakaya_endpoint`: The endpoint configuration for the Izakaya service
- `internal_api_key`: Internal API key for privileged operations
- `message`: The message together with its recipients
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the `MlsMessageCreateResp` or an `AinariError` if the operation fails.
*/
pub async fn send_mls_message(
    izakaya_endpoint: &ainari_config::Endpoint,
    internal_api_key: &Secret,
    message: &MlsMessageReq,
    insecure_client: bool,
) -> Result<MlsMessageCreateResp, AinariError> {
    let response = post_json(
        izakaya_endpoint,
        internal_api_key,
        "mls_message/internal",
        message,
        insecure_client,
    )
    .await;

    handle_response(response, "mls-message", "").await
}

/**
Lists the MLS-messages, which are waiting in the izakaya for a client.

The messages are ordered by their group and epoch, so they can be processed one after another.
Every poll also tells the izakaya, that the client is alive.

# Arguments
- `izakaya_endpoint`: The endpoint configuration for the Izakaya service
- `internal_api_key`: Internal API key for privileged operations
- `client_id`: Identity of the client, whose messages are listed
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing the `MlsMessageListResp` or an `AinariError` if the operation fails.
*/
pub async fn list_mls_messages(
    izakaya_endpoint: &ainari_config::Endpoint,
    internal_api_key: &Secret,
    client_id: &str,
    insecure_client: bool,
) -> Result<MlsMessageListResp, AinariError> {
    let address = izakaya_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/mls_message/{client_id}/internal");

    let response = client
        .get(url)
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_response(response, "mls-message", client_id).await
}

/**
Acknowledges a MLS-message, which removes it from the izakaya.

# Arguments
- `izakaya_endpoint`: The endpoint configuration for the Izakaya service
- `internal_api_key`: Internal API key for privileged operations
- `client_id`: Identity of the client, which received the message
- `message_uuid`: UUID of the message
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` indicating success or an `AinariError` if the operation fails.
*/
pub async fn delete_mls_message(
    izakaya_endpoint: &ainari_config::Endpoint,
    internal_api_key: &Secret,
    client_id: &str,
    message_uuid: &uuid::Uuid,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let address = izakaya_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/mls_message/{client_id}/{message_uuid}/internal");

    let response = client
        .delete(url)
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_empty_response(response, "mls-message", &message_uuid.to_string()).await
}

/**
Hands a membership-grant of hanami over to the izakaya.

A grant, which adds a gateway, allows it to join the group of a network, a grant, which removes
it, takes it out of the group again.

# Arguments
- `izakaya_endpoint`: The endpoint configuration for the Izakaya service
- `internal_api_key`: Internal API key for privileged operations
- `grant`: The signed grant
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` indicating success or an `AinariError` if the operation fails.
*/
pub async fn store_mls_grant(
    izakaya_endpoint: &ainari_config::Endpoint,
    internal_api_key: &Secret,
    grant: &MlsGrant,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let body = MlsGrantReq {
        grant: grant.clone(),
    };
    let response = post_json(
        izakaya_endpoint,
        internal_api_key,
        "mls_grant/internal",
        &body,
        insecure_client,
    )
    .await;

    handle_empty_response(response, "mls-grant", "").await
}

/**
Subscribes a gateway to the MLS-group of a network.

# Arguments
- `izakaya_endpoint`: The endpoint configuration for the Izakaya service
- `internal_api_key`: Internal API key for privileged operations
- `vni`: Tenant of the network
- `client_id`: Identity of the gateway
- `has_group`: Whether the gateway still holds the group
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` containing what the gateway has to do or an `AinariError` if the operation fails.
*/
pub async fn subscribe_mls_group(
    izakaya_endpoint: &ainari_config::Endpoint,
    internal_api_key: &Secret,
    vni: u32,
    client_id: &str,
    has_group: bool,
    insecure_client: bool,
) -> Result<MlsSubscribeResp, AinariError> {
    let body = MlsSubscribeReq {
        client_id: client_id.to_string(),
        has_group,
    };
    let response = post_json(
        izakaya_endpoint,
        internal_api_key,
        &format!("mls_group/{vni}/subscribe/internal"),
        &body,
        insecure_client,
    )
    .await;

    handle_response(response, "mls-group", &vni.to_string()).await
}

/**
Unsubscribes a gateway from the MLS-group of a network.

# Arguments
- `izakaya_endpoint`: The endpoint configuration for the Izakaya service
- `internal_api_key`: Internal API key for privileged operations
- `vni`: Tenant of the network
- `client_id`: Identity of the gateway
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` indicating success or an `AinariError` if the operation fails.
*/
pub async fn unsubscribe_mls_group(
    izakaya_endpoint: &ainari_config::Endpoint,
    internal_api_key: &Secret,
    vni: u32,
    client_id: &str,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let body = MlsUnsubscribeReq {
        client_id: client_id.to_string(),
    };
    let response = post_json(
        izakaya_endpoint,
        internal_api_key,
        &format!("mls_group/{vni}/unsubscribe/internal"),
        &body,
        insecure_client,
    )
    .await;

    handle_empty_response(response, "mls-group", &vni.to_string()).await
}

/**
Reports the result of a change of a MLS-group, which the committer made.

# Arguments
- `izakaya_endpoint`: The endpoint configuration for the Izakaya service
- `internal_api_key`: Internal API key for privileged operations
- `vni`: Tenant of the network
- `result`: The result of the change
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` indicating success or an `AinariError` if the operation fails.
*/
pub async fn finish_mls_operation(
    izakaya_endpoint: &ainari_config::Endpoint,
    internal_api_key: &Secret,
    vni: u32,
    result: &MlsOperationDoneReq,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let response = post_json(
        izakaya_endpoint,
        internal_api_key,
        &format!("mls_group/{vni}/operation/internal"),
        result,
        insecure_client,
    )
    .await;

    handle_empty_response(response, "mls-operation", &result.op_uuid.to_string()).await
}

/**
Acknowledges a phase of the key-rotation of a MLS-group.

# Arguments
- `izakaya_endpoint`: The endpoint configuration for the Izakaya service
- `internal_api_key`: Internal API key for privileged operations
- `vni`: Tenant of the network
- `ack`: The acknowledgement
- `insecure_client`: Whether to use an insecure (HTTP) client or secure (HTTPS) client

# Returns
A `Result` indicating success or an `AinariError` if the operation fails.
*/
pub async fn ack_mls_round(
    izakaya_endpoint: &ainari_config::Endpoint,
    internal_api_key: &Secret,
    vni: u32,
    ack: &MlsRoundAckReq,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let response = post_json(
        izakaya_endpoint,
        internal_api_key,
        &format!("mls_group/{vni}/round/internal"),
        ack,
        insecure_client,
    )
    .await;

    handle_empty_response(response, "mls-round", &vni.to_string()).await
}
