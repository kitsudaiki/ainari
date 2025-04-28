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

use actix_web::web::Path;
use actix_web::http::header::ContentDisposition;
use actix_multipart::Multipart;
use apistos::actix::CreatedJson;
use apistos::api_operation;
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;
use tokio::fs;
use log::error;
use std::error::Error;
use std::path::PathBuf;
use std::fs::File;
use std::io::{BufReader, Read};
use byteorder::{ReadBytesExt, BigEndian};
use uuid::Uuid;
use serde_json::{Value, Map};
use std::io::Write;

use crate::api::user_context::UserContext;
use crate::api::errors::ErrorResponse;
use crate::database::dataset_table;

use hanami_dataset::dataset_io::{DataSetType, init_new_data_set_file, DataSetFileWriteHandle_v1_0, Column};
use hanami_common::error::HanamiError;

use super::dataset_structs::DatasetResp;

#[derive(Debug)]
pub struct MnistImage {
    pub label: u8,
    pub pixels: Vec<u8>, // 28x28 = 784 pixels
}

#[api_operation(
    tag = "dataset",
    summary = "Create new cluster",
    description = r###"Create new cluster based on a cluster-template."###,
    error_code = 401,
    error_code = 500
)]
pub async fn upload_binary(mut payload: Multipart, path: Path<(String, String)>, context: UserContext) -> Result<CreatedJson<DatasetResp>, ErrorResponse> {
    let upload_dir_path = "./uploads";
    let upload_dir = PathBuf::from(&upload_dir_path);
    let dataset_uuid = Uuid::new_v4();
    let target_filepath: PathBuf = upload_dir.join(&dataset_uuid.to_string());

    let (dataset_type_str, name) = path.into_inner();
    let dataset_type = dataset_type_str.to_string();

    // check given type
    if ["mnist", "csv"].contains(&dataset_type.as_str()) == false {
        let msg = format!("Type '{}' is not in list [ mnist, csv ]", dataset_type);
        return Err(ErrorResponse::BadRequest(msg.to_string()));
    }

    // Ensure directory exists
    match fs::create_dir_all(&upload_dir).await {
        Ok(_) => (),
        Err(e) => {
            let msg = format!("Failed to create dataset-upload-directory '{}'.", upload_dir_path);
            error!("{}", msg);
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    }

    let mut filepaths = Vec::new();
    // process items from payload
    while let Some(item) = payload.next().await {
        let mut field = match item {
            Ok(value) => value,
            Err(_) => return Err(ErrorResponse::BadRequest("Failed to read next item from input.".to_string())),
        };

        // get file-name of item
        let content_disposition = field.content_disposition();
        let filename = match content_disposition {
            Some(ContentDisposition { parameters, .. }) => {
                parameters.iter().find_map(|param| {
                    if let actix_web::http::header::DispositionParam::Filename(ref filename) = *param {
                        Some(sanitize_filename::sanitize(filename))
                    } else {
                        None
                    }
                }).unwrap_or_else(|| "upload.bin".to_string())
            }
            None => "upload.bin".to_string(),
        };

        // create file
        let filepath: PathBuf = upload_dir.join(filename + &dataset_uuid.to_string());
        let mut f = match fs::File::create(&filepath).await {
            Ok(value) => value,
            Err(e) => {
                let path = filepath.as_os_str().to_str().unwrap();
                let msg = format!("Failed to create upload-file '{}' with error: {}.", path, e);
                error!("{}", msg);
                return Err(ErrorResponse::InternalError("".to_string()));
            }
        };

        filepaths.push(filepath);

        // fill content into file
        while let Some(chunk) = field.next().await {
            let data = match chunk {
                Ok(value) => value,
                Err(_) => return Err(ErrorResponse::BadRequest("Failed to read chunk.".to_string())),
            };
            
            let _ = f.write_all(&data).await;
        }
    }

    // process mnist-dataset
    if dataset_type == "mnist" {
        if filepaths.len() != 2 {
            let msg = format!("MNIST-dataset expect 2 uploaded files, but there were {} files found.", filepaths.len());
            return Err(ErrorResponse::BadRequest(msg));
        }
        match load_mnist_images(
            &filepaths[0], 
            &filepaths[1], 
            &target_filepath,
            dataset_uuid.clone(),
            name.clone(),
            None) 
        {
            Ok(()) => {},
            Err(e) => match e.downcast_ref::<HanamiError>() {
                Some(HanamiError::InputError(e)) => {
                    let msg = format!("{}", e);
                    return Err(ErrorResponse::BadRequest(msg));
                },
                _ => {
                    error!("{}", e);
                    return Err(ErrorResponse::InternalError("".to_string()));
                }
            },
        };
    }

    // add new dataset to datbase
    let file_path_str: String = target_filepath.to_string_lossy().into();
    match dataset_table::add_new_dataset(&dataset_uuid, &name, &file_path_str, &context) {
        Ok(_) => {},
        Err(_) => {
            let msg = format!("Failed to add dataset with ID '{}' to database.", dataset_uuid);
            error!("{}", msg);
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // get new created dataset from database to get addtional information
    match dataset_table::get_dataset(&dataset_uuid, &context) {
        Ok(dataset) => {
            let resp = DatasetResp {
                uuid: dataset_uuid.clone(),
                name: dataset.name.clone(),
                created_by: dataset.created_by.clone(),
                created_at: dataset.created_at.clone(),
                updated_by: dataset.updated_by.clone(),
                updated_at: dataset.updated_at.clone(),
            };
        
            return Ok(CreatedJson(resp));
        },
        Err(_) => 
        {
            let msg = format!("Failed to get dataset with ID '{}' from database, even the user should exist.", dataset_uuid);
            error!("{}", msg);
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };
}

fn convert_vec_u8_to_f32(vec_u8: &Vec<u8>) -> Vec<f32> {
    vec_u8.iter().map(|&x| x as f32).collect()
}

pub fn load_mnist_images(
    image_path: &PathBuf,
    label_path: &PathBuf,
    target_filepath: &PathBuf,
    uuid: Uuid,
    name: String,
    limit: Option<usize>,
) -> Result<(), Box<dyn Error>> {
    let mut img_reader = BufReader::new(File::open(image_path)?);
    let mut label_reader = BufReader::new(File::open(label_path)?);

    // check magic number of image-file
    let magic = img_reader.read_u32::<BigEndian>()?;
    if magic != 2051 {
        return Err("Invalid image file magic number!".into());
    }

    // get meta-information of image-file
    let num_images = img_reader.read_u32::<BigEndian>()?;
    let rows = img_reader.read_u32::<BigEndian>()?;
    let cols = img_reader.read_u32::<BigEndian>()?;

    // check magic number of label-file
    let label_magic = label_reader.read_u32::<BigEndian>()?;
    if label_magic != 2049 {
        return Err("Invalid label file magic number!".into());
    }

    // check if number of images and labes are the same
    let num_labels = label_reader.read_u32::<BigEndian>()?;
    if num_images != num_labels {
        return Err("Image and label count mismatch!".into());
    }
    // prepare buffer
    let count = limit.unwrap_or(num_images as usize).min(num_images as usize);
    let mut images = Vec::with_capacity(count);

    // read images and labels from files
    for _ in 0..count {
        let mut pixels = vec![0; (rows * cols) as usize];
        img_reader.read_exact(&mut pixels)?;

        let label = label_reader.read_u8()?;

        images.push(MnistImage { label, pixels });
    }

    let picture_size: u32 = rows * cols;
    let mut columns:Vec<Column> = Vec::new();

    let pictures = Column {
        name: "picture".to_string(),
        start: 0,
        end: picture_size,
    };
    let labels = Column {
        name: "label".to_string(),
        start: picture_size,
        end: picture_size + 10,
    };

    columns.push(pictures);
    columns.push(labels);

    let mut dataset_handle = init_new_data_set_file(
        &target_filepath, 
        uuid,
        name, 
        "".to_string(),
        columns,
        DataSetType::FloatType)?; // TODO: use u8-type

    let mut label_data: Vec<f32> = vec![0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32];
    for (_, img) in images.iter().enumerate() {
        // println!("Image {}: Label = {}", i, img.label);
        label_data[usize::from(img.label)] = 1.0f32;

        let converted = convert_vec_u8_to_f32(&img.pixels);

        let image_bytes = unsafe {
            std::slice::from_raw_parts(
                converted.as_ptr() as *const u8,
                converted.len() * std::mem::size_of::<f32>(),
            )
        };

        let label_bytes = unsafe {
            std::slice::from_raw_parts(
                label_data.as_ptr() as *const u8,
                label_data.len() * std::mem::size_of::<f32>(),
            )
        };
    
        dataset_handle.target_file.write_all(&image_bytes)?;
        dataset_handle.target_file.write_all(&label_bytes)?;

        label_data[usize::from(img.label)] = 0.0f32;
    }

    // disabled debug-output
    // for (i, img) in images.iter().enumerate() {
    //     println!("Image {}: Label = {}", i, img.label);
    // }

    Ok(())
}

