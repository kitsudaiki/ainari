// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

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

use ainari_api_structs::cluster_structs::*;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::prepare_client;
use crate::{handle_empty_response, handle_response};

pub async fn create_cluster(
    sakura_address: &String,
    token: &String,
    internal_api_key: &Secret,
    name: &str,
    template: &str,
    insecure_client: bool,
) -> Result<ClusterResp, AinariError> {
    let client = prepare_client(sakura_address, insecure_client);
    let url = format!("{sakura_address}/v1alpha/cluster/internal");

    let body = ClusterCreateReq {
        template: template.to_owned(),
        name: name.to_owned(),
    };
    let json_str = serde_json::to_string(&body).unwrap();

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    let resp: Result<ClusterResp, AinariError> = handle_response(response, "cluster", "").await;
    resp
}

pub async fn get_cluster(
    sakura_address: &String,
    token: &String,
    internal_api_key: &Secret,
    cluster_uuid: &Uuid,
    insecure_client: bool,
) -> Result<ClusterResp, AinariError> {
    let client = prepare_client(sakura_address, insecure_client);
    let url = format!("{sakura_address}/v1alpha/cluster/{cluster_uuid}");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    let resp: Result<ClusterResp, AinariError> =
        handle_response(response, "cluster", &cluster_uuid.to_string()).await;
    resp
}

pub async fn list_cluster(
    sakura_address: &String,
    token: &String,
    internal_api_key: &Secret,
    insecure_client: bool,
) -> Result<ClusterListResp, AinariError> {
    let client = prepare_client(sakura_address, insecure_client);
    let url = format!("{sakura_address}/v1alpha/cluster");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    let resp: Result<ClusterListResp, AinariError> = handle_response(response, "cluster", "").await;
    resp
}

pub async fn delete_cluster(
    sakura_address: &String,
    token: &String,
    internal_api_key: &Secret,
    cluster_uuid: &Uuid,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let client = prepare_client(sakura_address, insecure_client);
    let url = format!("{sakura_address}/v1alpha/cluster/{cluster_uuid}");

    let response = client
        .delete(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_empty_response(response, "cluster", &cluster_uuid.to_string()).await
}
