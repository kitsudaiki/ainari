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

#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode, Default)]
pub enum DataSetType {
    #[default]
    UndefinedType = 0,
    Uint8Type = 1,
    FloatType = 4,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetLink {
    pub onsen_address: String,
    pub remote_file_path: String,
    pub local_file_path: String,
    pub local_encrypted_file_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Column {
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct DataSetBaseHeader {
    pub type_identifier: String,
    pub version: String,
    pub minor_version: String,
}

impl Default for DataSetBaseHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSetBaseHeader {
    pub fn new() -> Self {
        DataSetBaseHeader {
            type_identifier: "sakura".to_string(),
            version: "1".to_string(),
            minor_version: "0alpha".to_string(),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct DataSetHeaderV1_0 {
    pub uuid: String, // HINT (kitsudaiki): String instead of Uuid, because Uuid doesn't implement Encode and Decode
    pub name: String,
    pub description: String,
    pub data_type: DataSetType,
    pub type_size: u8,
    pub row_size: u64,
    pub columns: HashMap<String, Column>,
}

impl DataSetHeaderV1_0 {
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

#[derive(Debug)]
pub struct DataSetFileWriteHandle {
    pub link: DatasetLink,
    pub header: DataSetHeaderV1_0,
    pub target_file: BufWriter<fs::File>,
    pub payload_offset: u64,
}

#[derive(Debug)]
pub struct DataSetFileReadHandle {
    pub link: DatasetLink,
    pub header: DataSetHeaderV1_0,
    pub target_file: BufReader<fs::File>,
    pub payload_offset: u64,
    pub selected_column: String,
    read_buffer: Vec<f32>,
    buffer_start_row: u64,
    buffer_end_row: u64,
}

impl DataSetFileReadHandle {
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

    fn get_data_from_buffer(&mut self, row: &u64) -> Result<(&[f32], u64), AinariError> {
        let column = &self.selected_column;
        let col_get = match self.header.columns.get(column) {
            Some(col) => col,
            _ => {
                let msg = format!("Column with name '{column}' not found in dataset.");
                return Err(AinariError::InternalError(msg));
            }
        };

        // cget pointer to the requested position in the buffer
        let row_col_size = col_get.end - col_get.start;
        let buffer_offset = ((row - self.buffer_start_row) * self.header.row_size) + col_get.start;
        let chunk: &[f32] =
            &self.read_buffer[(buffer_offset as usize)..((buffer_offset + row_col_size) as usize)];

        Ok((chunk, row_col_size))
    }

    pub fn get_data_from_file(&mut self, row: &u64) -> Result<(&[f32], u64), AinariError> {
        if row >= &self.buffer_start_row && row < &self.buffer_end_row {
            return self.get_data_from_buffer(row);
        }

        // calculate new buffer-dimensions
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

        // read selected block from file into the read-buffer
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

    // check give dataset-type
    if data_type == DataSetType::UndefinedType {
        return Err(Box::new(AinariError::InvalidInput(
            "Invalid dataset-type".to_string(),
        )));
    }

    // check if file already exist
    if Path::new(&link.local_file_path).exists() {
        let msg = format!("Dataset file '{}' already exists.", link.local_file_path);
        // HINT (kitsudaki): the path is defined by the backend itself and not by the user,
        // so here should be an internal error instand of an input-error
        return Err(Box::new(AinariError::InternalError(msg)));
    }

    // initialize file
    let file = fs::File::create(&link.local_file_path)?;

    // initialize header
    let base_header = DataSetBaseHeader::new();
    let header = DataSetHeaderV1_0::new(uuid, name, description, data_type, row_size, columns);

    // initialize resulting file-handle
    let mut result = DataSetFileWriteHandle {
        link: link.clone(),
        header,
        target_file: BufWriter::new(file),
        payload_offset: 0,
    };

    // write base-header to file
    let encoded_base = bincode::encode_to_vec(&base_header, bincode_config)?;
    result
        .target_file
        .write_all(&(encoded_base.len() as u64).to_le_bytes())?;
    result.target_file.write_all(&encoded_base)?;

    // write header to file
    let encoded_header = bincode::encode_to_vec(&result.header, bincode_config)?;
    result
        .target_file
        .write_all(&(encoded_header.len() as u64).to_le_bytes())?;
    result.target_file.write_all(&encoded_header)?;

    // flush file-buffer, to ensure, that the data are written to the disc
    result.target_file.flush()?;

    // get current byte-position within the file after writing the header
    result.payload_offset = result.target_file.stream_position()?;

    Ok(result)
}

pub fn read_data_set_file(
    local_file_path: &String,
) -> Result<DataSetFileReadHandle, Box<dyn std::error::Error>> {
    let bincode_config = config::standard();

    // check if file even exist
    if !Path::new(local_file_path).exists() {
        let msg = format!("Dataset file '{local_file_path}' does not exists.");
        // HINT (kitsudaki): the path comes from the database and not from the user,
        // so here should be an internal error instand of an input-error
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

    // read base-header-length
    let mut base_len_buf = [0u8; 8];
    result.target_file.read_exact(&mut base_len_buf)?;
    let base_len = u64::from_le_bytes(base_len_buf);

    // read base-header-length
    let mut base_buf = vec![0u8; base_len as usize];
    result.target_file.read_exact(&mut base_buf)?;
    // TODO: handle header
    let (_, _): (DataSetBaseHeader, usize) =
        bincode::decode_from_slice(&base_buf[..], bincode_config)?;

    // read header-length
    let mut header_len_buf = [0u8; 8];
    result.target_file.read_exact(&mut header_len_buf)?;
    let header_len = u64::from_le_bytes(header_len_buf);

    // read header-length
    let mut header_buf = vec![0u8; header_len as usize];
    result.target_file.read_exact(&mut header_buf)?;
    let (header, _): (DataSetHeaderV1_0, usize) =
        bincode::decode_from_slice(&header_buf[..], bincode_config)?;
    result.header = header;

    // TODO: make buffer-size configurable
    result.read_buffer.resize(
        ROWS_IN_READ_BUFFER as usize * result.header.row_size as usize,
        0f32,
    );

    // get current byte-position within the file after reading the header
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
