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

pub mod abort_task_v1_0;
pub mod checkpoint_restore_task_v1_0;
pub mod checkpoint_save_task_v1_0;
pub mod create_request_task_v1_0;
pub mod create_train_task_v1_0;
pub mod get_task_v1_0;
pub mod list_task_v1_0;

use uuid::Uuid;

use crate::config;
use crate::core::cluster_handler;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::quota::get_quota;
use ainari_common::error::AinariError;

fn get_current_number_of_open_tasks(cluster_uuid: &Uuid) -> Result<usize, ErrorResponse> {
    // get cluster-handle
    let cluster_handler = cluster_handler::CLUSTER_HANDLER
        .read()
        .expect("mutex poisoned");
    let cluster_handle = match cluster_handler.clusters.get(cluster_uuid) {
        Some(cluster_handle) => cluster_handle,
        None => return Err(ErrorResponse::InternalError("".to_string())),
    };
    let cluster_interface = if let Some(interface) = &cluster_handle.cluster_interface {
        interface
    } else {
        let msg = format!("Cluster with UUID '{cluster_uuid}' has not interface on the host.");
        return Err(ErrorResponse::NotFound(msg));
    };

    Ok(cluster_interface
        .lock()
        .expect("mutex poisoned")
        .get_number_open_tasks())
}

async fn check_quota(cluster_uuid: &Uuid, context: &UserContext) -> Result<(), ErrorResponse> {
    let current_number_of_open_tasks = get_current_number_of_open_tasks(cluster_uuid)?;

    // check the maximum number of secrets defined in miko
    let miko_endpoint = &config::CONFIG.miko;
    let max_number_of_open_tasks = match get_quota(
        miko_endpoint,
        &context.token,
        &context.user_id,
        config::CONFIG.insecure_clients,
    )
    .await
    {
        Ok(body) => body.max_taskqueue as i64,
        Err(AinariError::Unauthorized(msg)) => {
            return Err(ErrorResponse::Unauthorized(msg));
        }
        Err(AinariError::InvalidInput(msg)) => {
            return Err(ErrorResponse::BadRequest(msg));
        }
        Err(AinariError::Error(msg)) => {
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // check if quota is already exceeded
    if current_number_of_open_tasks as i64 >= max_number_of_open_tasks {
        let msg = format!(
            "Maximum number of open tasks for cluster with UUID '{cluster_uuid}' exceeded."
        );
        return Err(ErrorResponse::Conflict(msg));
    }

    Ok(())
}
