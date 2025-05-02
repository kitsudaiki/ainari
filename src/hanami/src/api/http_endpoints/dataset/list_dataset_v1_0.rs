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
use apistos::api_operation;
use uuid::Uuid;

use crate::api::errors::ErrorResponse;
use crate::api::user_context::UserContext;
use crate::database::dataset_table;

use super::dataset_structs::{DatasetBasicResp, DatasetListResp};

#[api_operation(
    tag = "dataset",
    summary = "List dataset",
    description = r###"List basic information of all dataset from the database."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_dataset(context: UserContext) -> Result<Json<DatasetListResp>, ErrorResponse> {
    let datasets = dataset_table::list_datasets(&context).unwrap();

    let mut resp = DatasetListResp {
        datasets: Vec::new(),
    };

    for dataset in datasets {
        match Uuid::parse_str(&dataset.uuid) {
            Ok(uuid) => {
                let obj = DatasetBasicResp {
                    uuid: uuid,
                    name: dataset.name.clone(),
                };
        
                resp.datasets.push(obj);
            }
            Err(e) =>  return Err(ErrorResponse::InternalError("".to_string())),
        }
    }

    Ok(Json(resp))
}
