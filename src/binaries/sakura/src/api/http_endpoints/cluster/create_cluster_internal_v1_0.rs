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

use actix_web::web::Json;
use apistos::actix::CreatedJson;
use apistos::api_operation;
use uuid::Uuid;
use validator::Validate;

use crate::core::cluster_handler::CLUSTER_HANDLER;
use crate::database::cluster_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::cluster_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_cluster_parser::cluster_parser::parse_cluster_template;

#[api_operation(
    tag = "cluster",
    summary = "Create new cluster",
    description = r###"Create new cluster based on a cluster-template."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn create_cluster_internal(
    body: Json<ClusterCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<ClusterResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let cluster_uuid = Uuid::new_v4();

    // parse cluster-template
    let mut parsed_cluster = match parse_cluster_template(&body.name, &body.template) {
        Ok(parsed) => parsed,
        Err(e) => {
            let msg = format!("Failed to parse cluster-template: {e:?}");
            return Err(ErrorResponse::BadRequest(msg));
        }
    };
    parsed_cluster.uuid = cluster_uuid;

    // parse cluster-template and create cluster from it
    let mut cluster_handler = CLUSTER_HANDLER.write().expect("mutex poisoned");
    cluster_handler
        .init_new_cluster(&cluster_uuid, &parsed_cluster)
        .map_err(map_ainari_error_to_api_response)?;

    // filter input-names
    let mut inputs: Vec<String> = Vec::new();
    for input in parsed_cluster.inputs {
        inputs.push(input.name);
    }

    // filter output-names
    let mut outputs: Vec<String> = Vec::new();
    for output in parsed_cluster.outputs {
        outputs.push(output.name);
    }

    // add new cluster to database
    match cluster_table::add_new_cluster(
        &cluster_uuid,
        &body.name,
        &body.template,
        &inputs,
        &outputs,
        &context,
    ) {
        Ok(_) => {}
        Err(e) => {
            let msg = format!(
                "Failed to add cluster with UUID '{cluster_uuid}' to database with error: {e}"
            );
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    // get new created cluster from database to get addtional information
    let cluster_data = cluster_table::get_cluster(&cluster_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("cluster", &cluster_uuid, e))?;

    let resp = ClusterResp {
        uuid: cluster_uuid,
        name: cluster_data.name,
        inputs,
        outputs,
        template: cluster_data.template,
        torii_port: 0,
        created_by: cluster_data.created_by,
        created_at: cluster_data.created_at,
        updated_by: cluster_data.updated_by,
        updated_at: cluster_data.updated_at,
    };

    Ok(CreatedJson(resp))
}
