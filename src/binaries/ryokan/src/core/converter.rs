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

use bytemuck::cast_slice;
use byteorder::{BigEndian, ReadBytesExt};
use csv::ReaderBuilder;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;
use uuid::Uuid;

use ainari_dataset::dataset_io::*;

/// Represents a single MNIST image with its label and pixel data.
///
/// The MNIST dataset consists of 28x28 pixel grayscale images of handwritten digits.
/// Each image is represented as a flat vector of 784 bytes (28 * 28).
#[derive(Debug)]
pub struct MnistImage {
    /// The label of the digit in the image (0-9).
    pub label: u8,
    /// The pixel data of the image as a flat vector of 784 bytes.
    /// Each byte represents the grayscale value of a pixel (0-255).
    pub pixels: Vec<u8>, // 28x28 = 784 pixels
}

/// Converts a vector of u8 values to a vector of f32 values.
///
/// This function takes a slice of u8 values and converts each value to its f32 equivalent.
/// This is useful for converting pixel data from u8 to f32 for machine learning applications.
///
/// # Arguments
///
/// * `vec_u8` - A slice of u8 values to be converted.
///
/// # Returns
///
/// A new Vec<f32> containing the converted values.
fn convert_vec_u8_to_f32(vec_u8: &[u8]) -> Vec<f32> {
    vec_u8.iter().map(|&x| x as f32).collect()
}

/// Loads MNIST images and labels from binary files and writes them to a dataset file.
///
/// This function reads MNIST image and label files, validates their structure,
/// and writes the data to a new dataset file in the specified format.
///
/// # Arguments
///
/// * `image_path` - Path to the MNIST image file.
/// * `label_path` - Path to the MNIST label file.
/// * `target_filepath` - Path where the new dataset file will be created.
/// * `uuid` - Unique identifier for the dataset.
/// * `name` - Name of the dataset.
/// * `limit` - Optional limit on the number of images to process.
///
/// # Returns
///
/// * `Ok(())` if the operation was successful.
/// * An error if any step fails (file operations, data validation, etc.).
///
/// # Errors
///
/// * Invalid magic numbers in the input files.
/// * Mismatch between image and label counts.
/// * File operation failures.
/// * Dataset initialization failures.
pub fn load_mnist_images(
    image_path: &PathBuf,
    label_path: &PathBuf,
    target_filepath: &str,
    uuid: Uuid,
    name: &str,
    limit: Option<usize>,
) -> Result<(), Box<dyn Error>> {
    let mut img_reader = BufReader::new(File::open(image_path)?);
    let mut label_reader = BufReader::new(File::open(label_path)?);

    // Check magic number of image-file (should be 2051 for MNIST images)
    let magic = img_reader.read_u32::<BigEndian>()?;
    if magic != 2051 {
        return Err("Invalid image file magic number!".into());
    }

    // Read meta-information from image-file header
    let num_images = img_reader.read_u32::<BigEndian>()?;
    let rows = img_reader.read_u32::<BigEndian>()?;
    let cols = img_reader.read_u32::<BigEndian>()?;

    // Check magic number of label-file (should be 2049 for MNIST labels)
    let label_magic = label_reader.read_u32::<BigEndian>()?;
    if label_magic != 2049 {
        return Err("Invalid label file magic number!".into());
    }

    // Verify that the number of images matches the number of labels
    let num_labels = label_reader.read_u32::<BigEndian>()?;
    if num_images != num_labels {
        return Err("Image and label count mismatch!".into());
    }

    // Prepare buffer for images, respecting the limit parameter
    let count = limit
        .unwrap_or(num_images as usize)
        .min(num_images as usize);
    let mut images = Vec::with_capacity(count);

    // Read images and labels from files
    for _ in 0..count {
        let mut pixels = vec![0; (rows * cols) as usize];
        img_reader.read_exact(&mut pixels)?;

        let label = label_reader.read_u8()?;

        images.push(MnistImage { label, pixels });
    }

    // Define column structure for the dataset
    let picture_size: u64 = (rows * cols) as u64;
    let mut columns: HashMap<String, Column> = HashMap::new();

    // Picture data column (784 values)
    let pictures = Column {
        start: 0,
        end: picture_size,
    };
    columns.insert("picture".to_string(), pictures);

    // Label data column (10 values for one-hot encoding)
    let labels = Column {
        start: picture_size,
        end: picture_size + 10,
    };
    columns.insert("label".to_string(), labels);

    // Initialize dataset file
    let row_size = picture_size + 10;
    let link = DatasetLink {
        onsen_address: "".to_string(),
        remote_file_path: "".to_string(),
        local_file_path: target_filepath.to_owned(),
        local_encrypted_file_path: target_filepath.to_owned(),
    };
    let mut dataset_handle = init_new_data_set_file(
        &link,
        uuid,
        name,
        "".to_string(),
        row_size,
        columns,
        DataSetType::FloatType,
    )?; // TODO: use u8-type

    // Prepare buffer for one-hot encoded labels
    let mut label_data: Vec<f32> = vec![
        0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32,
    ];

    // Process each image and write to dataset file
    for img in images.iter() {
        // Set the appropriate label position to 1.0 (one-hot encoding)
        label_data[usize::from(img.label)] = 1.0f32;

        // Convert pixel data from u8 to f32
        let converted = convert_vec_u8_to_f32(&img.pixels);
        let image_bytes: &[u8] = cast_slice(&converted);
        let label_bytes: &[u8] = cast_slice(&label_data);

        // Write image and label data to the dataset file
        dataset_handle.target_file.write_all(image_bytes)?;
        dataset_handle.target_file.write_all(label_bytes)?;

        // Reset the label buffer for the next image
        label_data[usize::from(img.label)] = 0.0f32;
    }

    // Debug output is disabled but could be useful for verification
    // for (i, img) in images.iter().enumerate() {
    //     println!("Image {}: Label = {}", i, img.label);
    // }

    Ok(())
}

/// Loads data from a CSV file and writes it to a dataset file.
///
/// This function reads a CSV file, extracts the column headers,
/// and writes the data to a new dataset file in the specified format.
///
/// # Arguments
///
/// * `csv_file_path` - Path to the CSV file.
/// * `target_filepath` - Path where the new dataset file will be created.
/// * `uuid` - Unique identifier for the dataset.
/// * `name` - Name of the dataset.
///
/// # Returns
///
/// * `Ok(())` if the operation was successful.
/// * An error if any step fails (file operations, CSV parsing, dataset initialization).
///
/// # Errors
///
/// * File operation failures.
/// * CSV parsing failures.
/// * Dataset initialization failures.
pub fn load_csv_file(
    csv_file_path: &PathBuf,
    target_filepath: &str,
    uuid: Uuid,
    name: &str,
) -> Result<(), Box<dyn Error>> {
    // Open the CSV file
    let file = File::open(csv_file_path)?;
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(file);

    // Get number of columns from header
    let headers = rdr.headers()?;
    let num_columns = headers.len();

    // Create column definitions based on header names
    let mut columns: HashMap<String, Column> = HashMap::new();
    for (i, name) in headers.iter().enumerate() {
        let col = Column {
            start: i as u64,
            end: i as u64 + 1,
        };
        columns.insert(name.to_string(), col);
    }

    // Initialize dataset file
    let link = DatasetLink {
        onsen_address: "".to_string(),
        remote_file_path: "".to_string(),
        local_file_path: target_filepath.to_owned(),
        local_encrypted_file_path: target_filepath.to_owned(),
    };
    let mut dataset_handle = init_new_data_set_file(
        &link,
        uuid,
        name,
        "".to_string(),
        num_columns as u64,
        columns,
        DataSetType::FloatType,
    )?; // TODO: use u8-type

    // Process each record in the CSV file
    for result in rdr.records() {
        let record = result?;

        // Convert each field to f32, defaulting to 0.0 if parsing fails
        let row = record
            .iter()
            .map(|field| field.parse::<f32>().unwrap_or(0.0))
            .collect::<Vec<f32>>();

        // Convert the row data to bytes and write to the dataset file
        let row_bytes: &[u8] = cast_slice(&row);
        dataset_handle.target_file.write_all(row_bytes)?;
    }

    Ok(())
}
