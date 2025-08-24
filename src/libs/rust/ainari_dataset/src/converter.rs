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

use bytemuck::cast_slice;
use byteorder::{BigEndian, ReadBytesExt};
use csv::ReaderBuilder;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;
use uuid::Uuid;

use super::dataset_io::*;

#[derive(Debug)]
pub struct MnistImage {
    pub label: u8,
    pub pixels: Vec<u8>, // 28x28 = 784 pixels
}

fn convert_vec_u8_to_f32(vec_u8: &[u8]) -> Vec<f32> {
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
    let count = limit
        .unwrap_or(num_images as usize)
        .min(num_images as usize);
    let mut images = Vec::with_capacity(count);

    // read images and labels from files
    for _ in 0..count {
        let mut pixels = vec![0; (rows * cols) as usize];
        img_reader.read_exact(&mut pixels)?;

        let label = label_reader.read_u8()?;

        images.push(MnistImage { label, pixels });
    }

    let picture_size: u64 = (rows * cols) as u64;
    let mut columns: HashMap<String, Column> = HashMap::new();

    let pictures = Column {
        start: 0,
        end: picture_size,
    };
    columns.insert("picture".to_string(), pictures);

    let labels = Column {
        start: picture_size,
        end: picture_size + 10,
    };
    columns.insert("label".to_string(), labels);

    let row_size = picture_size + 10;
    let mut dataset_handle = init_new_data_set_file(
        target_filepath,
        uuid,
        name,
        "".to_string(),
        row_size,
        columns,
        DataSetType::FloatType,
    )?; // TODO: use u8-type

    let mut label_data: Vec<f32> = vec![
        0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32,
    ];
    for img in images.iter() {
        // println!("Image {}: Label = {}", i, img.label);
        label_data[usize::from(img.label)] = 1.0f32;

        let converted = convert_vec_u8_to_f32(&img.pixels);
        let image_bytes: &[u8] = cast_slice(&converted);
        let label_bytes: &[u8] = cast_slice(&label_data);

        dataset_handle.target_file.write_all(image_bytes)?;
        dataset_handle.target_file.write_all(label_bytes)?;

        label_data[usize::from(img.label)] = 0.0f32;
    }

    // disabled debug-output
    // for (i, img) in images.iter().enumerate() {
    //     println!("Image {}: Label = {}", i, img.label);
    // }

    Ok(())
}

pub fn load_csv_file(
    file_path: &PathBuf,
    target_filepath: &PathBuf,
    uuid: Uuid,
    name: String,
) -> Result<(), Box<dyn Error>> {
    let file = File::open(file_path)?;
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(file);

    // get number of columns from header
    let headers = rdr.headers()?;
    let num_columns = headers.len();

    // get column-names from header
    let mut columns: HashMap<String, Column> = HashMap::new();
    for (i, name) in headers.iter().enumerate() {
        let col = Column {
            start: i as u64,
            end: i as u64 + 1,
        };
        columns.insert(name.to_string(), col);
    }

    // init dataset
    let mut dataset_handle = init_new_data_set_file(
        target_filepath,
        uuid,
        name,
        "".to_string(),
        num_columns as u64,
        columns,
        DataSetType::FloatType,
    )?; // TODO: use u8-type

    // read body into the dataset-file
    for result in rdr.records() {
        let record = result?;

        let row = record
            .iter()
            .map(|field| field.parse::<f32>().unwrap_or(0.0))
            .collect::<Vec<f32>>();

        let row_bytes: &[u8] = cast_slice(&row);

        dataset_handle.target_file.write_all(row_bytes)?;
    }

    Ok(())
}
