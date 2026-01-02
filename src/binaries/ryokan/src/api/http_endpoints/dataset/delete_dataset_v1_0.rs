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

use actix_web::web::Path;
use apistos::actix::NoContent;
use apistos::api_operation;
use uuid::Uuid;

use crate::config;
use crate::database::dataset_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::get_endpoints;
use ainari_clients::secret::delete_secret;

#[api_operation(
    tag = "dataset",
    summary = "Delete dataset",
    description = r###"Delete a dataset from the database and files from the storage."###,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_dataset(
    dataset_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    let dataset_data = dataset_table::get_dataset(&dataset_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("dataset", &dataset_uuid, e))?;

    // delete dataset from database
    dataset_table::delete_dataset(&dataset_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("dataset", &dataset_uuid, e))?;

    // delete dataset-payload from onsen
    delete_file_from_onsen(&dataset_data.onsen_address, &dataset_data.file_path).await?;

    // delete secret from omamori
    let miko_endpoint = &config::CONFIG.miko;
    let endpoints = get_endpoints(miko_endpoint, config::CONFIG.skip_tls_verification)
        .await
        .map_err(map_ainari_error_to_api_response)?;
    let secret_uuid = convert_uuid(&dataset_data.secret_uuid)?;
    delete_secret(
        &endpoints.omamori,
        &context.token,
        &secret_uuid,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    Ok(NoContent)
}
