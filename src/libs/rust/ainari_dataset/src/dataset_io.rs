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

use bincode::{Decode, Encode, config};
use bytemuck::cast_slice_mut;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::SeekFrom;
use std::io::Write;
use std::io::{BufReader, BufWriter, Read, Seek};
use std::path::Path;
use std::str;
use uuid::Uuid;

use ainari_common::constants::*;
use ainari_common::error::AinariError;

/// Represents the type of data stored in a dataset.
/// This enum defines the possible data types that can be stored in a dataset.
/// The values are encoded as u8 for efficient storage and serialization.
#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode, Default)]
pub enum DataSetType {
    /// Undefined type, serves as a default value.
    #[default]
    UndefinedType = 0,
    /// 8-bit unsigned integer type.
    Uint8Type = 1,
    /// 32-bit floating point type.
    FloatType = 4,
}

/// Represents a link to a dataset file, containing paths for local and remote storage.
/// This struct holds information about where a dataset file is stored and its encrypted version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetLink {
    /// Address of the Onsen node where the dataset is stored.
    pub onsen_address: String,
    /// Path to the dataset file on the remote storage.
    pub remote_file_path: String,
    /// Path to the dataset file on the local storage.
    pub local_file_path: String,
    /// Path to the encrypted version of the dataset file on local storage.
    pub local_encrypted_file_path: String,
}

/// Represents a column in a dataset, defining its start and end positions.
/// This struct is used to specify the boundaries of a column within the dataset file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Column {
    /// The starting position of the column in bytes.
    pub start: u64,
    /// The ending position of the column in bytes.
    pub end: u64,
}

/// Base header for dataset files, containing basic identification information.
/// This struct serves as the initial header in the dataset file format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct DataSetBaseHeader {
    /// Identifier for the dataset type.
    pub type_identifier: String,
    /// Major version of the dataset format.
    pub version: String,
    /// Minor version of the dataset format.
    pub minor_version: String,
}

impl Default for DataSetBaseHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSetBaseHeader {
    /// Creates a new DataSetBaseHeader with default values.
    ///
    /// # Returns
    /// A new instance of DataSetBaseHeader with default values.
    pub fn new() -> Self {
        DataSetBaseHeader {
            type_identifier: "sakura".to_string(),
            version: "1".to_string(),
            minor_version: "0alpha".to_string(),
        }
    }
}

/// Header for version 1.0 of the dataset format.
/// This struct contains metadata about the dataset including its UUID, name, description,
/// data type, size information, and column definitions.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct DataSetHeaderV1_0 {
    /// Unique identifier for the dataset, stored as a string for serialization compatibility.
    pub uuid: String, // HINT (kitsudaiki): String instead of Uuid, because Uuid doesn't implement Encode and Decode
    /// Name of the dataset.
    pub name: String,
    /// Description of the dataset contents.
    pub description: String,
    /// Type of data stored in the dataset.
    pub data_type: DataSetType,
    /// Size of each data element in bytes.
    pub type_size: u8,
    /// Size of each row in the dataset in elements.
    pub row_size: u64,
    /// Mapping of column names to their positions in the dataset.
    pub columns: HashMap<String, Column>,
}

impl DataSetHeaderV1_0 {
    /// Creates a new DataSetHeaderV1_0 with the provided parameters.
    ///
    /// # Arguments
    /// * `uuid` - Unique identifier for the dataset.
    /// * `name` - Name of the dataset.
    /// * `description` - Description of the dataset contents.
    /// * `dataset_type` - Type of data stored in the dataset.
    /// * `row_size` - Size of each row in the dataset in elements.
    /// * `columns` - Mapping of column names to their positions.
    ///
    /// # Returns
    /// A new instance of DataSetHeaderV1_0 with the provided values.
    pub fn new(
        uuid: Uuid,
        name: &str,
        description: String,
        dataset_type: DataSetType,
        row_size: u64,
        columns: HashMap<String, Column>,
    ) -> Self {
        DataSetHeaderV1_0 {
            uuid: uuid.to_string(),
            name: name.to_owned(),
            description,
            data_type: dataset_type.clone(),
            type_size: dataset_type as u8,
            row_size,
            columns,
        }
    }
}

/// Handle for writing to a dataset file.
/// This struct provides access to write operations for a dataset file.
#[derive(Debug)]
pub struct DataSetFileWriteHandle {
    /// Link to the dataset file locations.
    pub link: DatasetLink,
    /// Header containing metadata about the dataset.
    pub header: DataSetHeaderV1_0,
    /// Buffered writer for the dataset file.
    pub target_file: BufWriter<fs::File>,
    /// Offset in bytes where the payload data begins.
    pub payload_offset: u64,
}

/// Handle for reading from a dataset file.
/// This struct provides access to read operations for a dataset file.
#[derive(Debug)]
pub struct DataSetFileReadHandle {
    /// Link to the dataset file locations.
    pub link: DatasetLink,
    /// Header containing metadata about the dataset.
    pub header: DataSetHeaderV1_0,
    /// Buffered reader for the dataset file.
    pub target_file: BufReader<fs::File>,
    /// Offset in bytes where the payload data begins.
    pub payload_offset: u64,
    /// Name of the currently selected column for reading.
    pub selected_column: String,
    /// Buffer for storing read data.
    read_buffer: Vec<f32>,
    /// Starting row index of the current buffer.
    buffer_start_row: u64,
    /// Ending row index of the current buffer.
    buffer_end_row: u64,
}

impl DataSetFileReadHandle {
    /// Calculates the total number of rows in the dataset.
    ///
    /// # Returns
    /// The number of rows in the dataset, or 0 if an error occurs.
    pub fn get_number_of_rows(&self) -> u64 {
        match self.target_file.get_ref().metadata() {
            Ok(metadata) => {
                let content_size = metadata.len() - self.payload_offset;
                content_size / (self.header.row_size * 4)
            }
            Err(_) => {
                // TODO: handle error-case better
                0
            }
        }
    }

    /// Retrieves data from the read buffer for a specific row and column.
    ///
    /// # Arguments
    /// * `row` - The row index to retrieve data from.
    ///
    /// # Returns
    /// A tuple containing a slice of f32 values and the size of the data,
    /// or an error if the column is not found.
    fn get_data_from_buffer(&mut self, row: &u64) -> Result<(&[f32], u64), AinariError> {
        let column = &self.selected_column;
        let col_get = match self.header.columns.get(column) {
            Some(col) => col,
            _ => {
                let msg = format!("Column with name '{column}' not found in dataset.");
                return Err(AinariError::InternalError(msg));
            }
        };

        // Calculate pointer to the requested position in the buffer
        let row_col_size = col_get.end - col_get.start;
        let buffer_offset = ((row - self.buffer_start_row) * self.header.row_size) + col_get.start;
        let chunk: &[f32] =
            &self.read_buffer[(buffer_offset as usize)..((buffer_offset + row_col_size) as usize)];

        Ok((chunk, row_col_size))
    }

    /// Retrieves data from the file for a specific row and column.
    /// If the data is not in the current buffer, it reads the appropriate block from the file.
    ///
    /// # Arguments
    /// * `row` - The row index to retrieve data from.
    ///
    /// # Returns
    /// A tuple containing a slice of f32 values and the size of the data,
    /// or an error if the row is out of bounds or if reading fails.
    pub fn get_data_from_file(&mut self, row: &u64) -> Result<(&[f32], u64), AinariError> {
        // Check if data is already in the buffer
        if row >= &self.buffer_start_row && row < &self.buffer_end_row {
            return self.get_data_from_buffer(row);
        }

        // Calculate new buffer dimensions
        let max_rows = self.get_number_of_rows();
        if row >= &max_rows {
            let msg = format!("Row-number {row} is too big for the dataset.");
            return Err(AinariError::InternalError(msg));
        }
        self.buffer_start_row = row - (row % ROWS_IN_READ_BUFFER);
        self.buffer_end_row = self.buffer_start_row + ROWS_IN_READ_BUFFER;
        if self.buffer_end_row > max_rows {
            self.buffer_end_row = max_rows;
        }

        // Read selected block from file into the read buffer
        let offset_bytes = (self.header.row_size) * self.buffer_start_row * 4;
        let byte_slice_input: &mut [u8] = cast_slice_mut(self.read_buffer.as_mut_slice());
        let file_offset = self.payload_offset + offset_bytes;
        match self.target_file.seek(SeekFrom::Start(file_offset)) {
            Ok(_) => {}
            Err(_) => {
                let msg = ("Failed to read data from the dataset-file.").to_string();
                return Err(AinariError::InternalError(msg));
            }
        }
        let _ = self.target_file.read_exact(byte_slice_input);

        self.get_data_from_buffer(row)
    }
}

/// Initializes a new dataset file with the given parameters.
///
/// # Arguments
/// * `link` - Information about the dataset file locations.
/// * `uuid` - Unique identifier for the dataset.
/// * `name` - Name of the dataset.
/// * `description` - Description of the dataset contents.
/// * `row_size` - Size of each row in the dataset in elements.
/// * `columns` - Mapping of column names to their positions.
/// * `data_type` - Type of data stored in the dataset.
///
/// # Returns
/// A DataSetFileWriteHandle for writing to the new dataset file,
/// or an error if initialization fails.
pub fn init_new_data_set_file(
    link: &DatasetLink,
    uuid: Uuid,
    name: &str,
    description: String,
    row_size: u64,
    columns: HashMap<String, Column>,
    data_type: DataSetType,
) -> Result<DataSetFileWriteHandle, Box<dyn std::error::Error>> {
    let bincode_config = config::standard();

    // Check given dataset type
    if data_type == DataSetType::UndefinedType {
        return Err(Box::new(AinariError::InvalidInput(
            "Invalid dataset-type".to_string(),
        )));
    }

    // Check if file already exists
    if Path::new(&link.local_file_path).exists() {
        let msg = format!("Dataset file '{}' already exists.", link.local_file_path);
        // HINT (kitsudaki): the path is defined by the backend itself and not by the user,
        // so here should be an internal error instead of an input-error
        return Err(Box::new(AinariError::InternalError(msg)));
    }

    // Initialize file
    let file = fs::File::create(&link.local_file_path)?;

    // Initialize header
    let base_header = DataSetBaseHeader::new();
    let header = DataSetHeaderV1_0::new(uuid, name, description, data_type, row_size, columns);

    // Initialize resulting file handle
    let mut result = DataSetFileWriteHandle {
        link: link.clone(),
        header,
        target_file: BufWriter::new(file),
        payload_offset: 0,
    };

    // Write base header to file
    let encoded_base = bincode::encode_to_vec(&base_header, bincode_config)?;
    result
        .target_file
        .write_all(&(encoded_base.len() as u64).to_le_bytes())?;
    result.target_file.write_all(&encoded_base)?;

    // Write header to file
    let encoded_header = bincode::encode_to_vec(&result.header, bincode_config)?;
    result
        .target_file
        .write_all(&(encoded_header.len() as u64).to_le_bytes())?;
    result.target_file.write_all(&encoded_header)?;

    // Flush file buffer to ensure data is written to disk
    result.target_file.flush()?;

    // Get current byte position within the file after writing the header
    result.payload_offset = result.target_file.stream_position()?;

    Ok(result)
}

/// Reads an existing dataset file and returns a handle for reading it.
///
/// # Arguments
/// * `local_file_path` - Path to the dataset file to read.
///
/// # Returns
/// A DataSetFileReadHandle for reading the dataset file,
/// or an error if reading fails.
pub fn read_data_set_file(
    local_file_path: &String,
) -> Result<DataSetFileReadHandle, Box<dyn std::error::Error>> {
    let bincode_config = config::standard();

    // Check if file exists
    if !Path::new(local_file_path).exists() {
        let msg = format!("Dataset file '{local_file_path}' does not exists.");
        // HINT (kitsudaki): the path comes from the database and not from the user,
        // so here should be an internal error instead of an input-error
        return Err(Box::new(AinariError::InternalError(msg)));
    }

    let file = fs::File::open(local_file_path)?;

    let mut result = DataSetFileReadHandle {
        link: DatasetLink {
            onsen_address: "".to_string(),
            remote_file_path: "".to_string(),
            local_file_path: local_file_path.clone(),
            local_encrypted_file_path: local_file_path.to_owned(),
        },
        header: DataSetHeaderV1_0::default(),
        target_file: BufReader::with_capacity(4096, file),
        payload_offset: 0,
        selected_column: "".to_string(),
        read_buffer: Vec::new(),
        buffer_start_row: 0,
        buffer_end_row: 0,
    };

    // Read base header length
    let mut base_len_buf = [0u8; 8];
    result.target_file.read_exact(&mut base_len_buf)?;
    let base_len = u64::from_le_bytes(base_len_buf);

    // Read base header
    let mut base_buf = vec![0u8; base_len as usize];
    result.target_file.read_exact(&mut base_buf)?;
    // TODO: handle header
    let (_, _): (DataSetBaseHeader, usize) =
        bincode::decode_from_slice(&base_buf[..], bincode_config)?;

    // Read header length
    let mut header_len_buf = [0u8; 8];
    result.target_file.read_exact(&mut header_len_buf)?;
    let header_len = u64::from_le_bytes(header_len_buf);

    // Read header
    let mut header_buf = vec![0u8; header_len as usize];
    result.target_file.read_exact(&mut header_buf)?;
    let (header, _): (DataSetHeaderV1_0, usize) =
        bincode::decode_from_slice(&header_buf[..], bincode_config)?;
    result.header = header;

    // TODO: make buffer size configurable
    result.read_buffer.resize(
        ROWS_IN_READ_BUFFER as usize * result.header.row_size as usize,
        0f32,
    );

    // Get current byte position within the file after reading the header
    result.payload_offset = result.target_file.stream_position()?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::{cast_slice, cast_slice_mut};
    use std::io::SeekFrom;

    #[test]
    fn test_dataset() {
        let uuid = Uuid::new_v4();
        let link = DatasetLink {
            onsen_address: "127.0.0.1".to_string(),
            remote_file_path: format!("{uuid}"),
            local_file_path: format!("/tmp/{uuid}"),
            local_encrypted_file_path: format!("/tmp/{uuid}_encrypted"),
        };

        let _ = fs::remove_file(&link.local_file_path).is_ok();

        let name = "test_dataset".to_string();
        let description = "This is a test-dataset".to_string();
        let mut columns: HashMap<String, Column> = HashMap::new();
        let data_type = DataSetType::FloatType;

        let test_col1 = Column { start: 0, end: 10 };
        columns.insert("col1".to_string(), test_col1);

        let test_col2 = Column { start: 10, end: 15 };
        columns.insert("col2".to_string(), test_col2);
        let row_size = 15;

        let mut write_dataset_handle = init_new_data_set_file(
            &link,
            uuid,
            &name,
            description.clone(),
            row_size,
            columns.clone(),
            data_type.clone(),
        )
        .unwrap();

        // write date for first column
        let col1: Vec<f32> = vec![
            4.0f32, 0.0f32, 2.0f32, 0.0f32, 1.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32,
        ];
        let col1_bytes = cast_slice(&col1);
        write_dataset_handle
            .target_file
            .write_all(col1_bytes)
            .unwrap();

        // write date for second column
        let col2: Vec<f32> = vec![0.0f32, 0.0f32, 1.0f32, 0.0f32, 0.0f32];
        let col2_bytes = cast_slice(&col2);
        write_dataset_handle
            .target_file
            .write_all(col2_bytes)
            .unwrap();

        // buffer must be flush, so the written columns are in the file on the disc and can be read again
        let _ = write_dataset_handle.target_file.flush();

        assert!(Path::new(&link.local_file_path).exists());

        // check single fields of the created header
        assert_eq!(write_dataset_handle.header.uuid, uuid.to_string());
        assert_eq!(write_dataset_handle.header.name.clone(), name);
        assert_eq!(write_dataset_handle.header.description.clone(), description);
        assert_eq!(write_dataset_handle.header.data_type.clone(), data_type);
        assert_eq!(write_dataset_handle.header.type_size, 4);
        assert_eq!(write_dataset_handle.header.columns.clone(), columns);

        let mut read_dataset_handle = read_data_set_file(&link.local_file_path).unwrap();

        // compare written and read header
        assert_eq!(write_dataset_handle.header, read_dataset_handle.header);
        assert_eq!(
            write_dataset_handle.payload_offset,
            read_dataset_handle.payload_offset
        );

        // read and compare frist column
        let col_get1 = match read_dataset_handle.header.columns.get("col1") {
            Some(col) => col,
            _ => {
                panic!();
            }
        };
        let col_get2 = match read_dataset_handle.header.columns.get("col2") {
            Some(col) => col,
            _ => {
                panic!();
            }
        };

        let size_col1 = (col_get1.end - col_get1.start) as usize;
        let mut col1_read = vec![0.0f32; size_col1];
        let byte_slice_col1: &mut [u8] = cast_slice_mut(col1_read.as_mut_slice());
        read_dataset_handle
            .target_file
            .seek(SeekFrom::Start(read_dataset_handle.payload_offset))
            .unwrap();
        let _ = read_dataset_handle.target_file.read_exact(byte_slice_col1);
        assert_eq!(col1_read, col1);

        // read and compare second column
        let size_col2 = (col_get2.end - col_get2.start) as usize;
        let mut col2_read = vec![0.0f32; size_col2];
        let byte_slice_col2: &mut [u8] = cast_slice_mut(col2_read.as_mut_slice());
        read_dataset_handle
            .target_file
            .seek(SeekFrom::Start(read_dataset_handle.payload_offset + 40))
            .unwrap();
        read_dataset_handle
            .target_file
            .read_exact(byte_slice_col2)
            .unwrap();
        assert_eq!(col2_read, col2);
    }
}
