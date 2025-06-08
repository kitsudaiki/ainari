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

use hanami_structs::dataset_structs::{DatasetBasicResp, DatasetListResp};

#[api_operation(
    tag = "dataset",
    summary = "List dataset",
    description = r###"List basic information of all dataset from the database."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_dataset(context: UserContext) -> Result<Json<DatasetListResp>, ErrorResponse> {
    let datasets = match dataset_table::list_datasets(&context)
    {
        Ok(datasets) => datasets,
        Err(e) => {
            log::error!("Failed to get list of datasets form database: '{e}'");
            return Err(ErrorResponse::InternalError("".to_string()))
        }
    };

    let mut resp = DatasetListResp {
        datasets: Vec::new(),
    };

    for dataset in datasets {
        // parse-uuid-string coming from the database
        let uuid = match Uuid::parse_str(&dataset.uuid) {
            Ok(uuid) => uuid,
            Err(e) => {
                log::error!("Failed to convert dataset-uuid with error: '{e}'");
                return Err(ErrorResponse::InternalError("".to_string()))
            },
        };

        let obj = DatasetBasicResp {
            uuid: uuid,
            name: dataset.name.clone(),
        };

        resp.datasets.push(obj);
    }

    Ok(Json(resp))
}
