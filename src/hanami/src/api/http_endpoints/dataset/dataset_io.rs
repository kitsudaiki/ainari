use std::str;
use serde::{Serialize, Serializer};
use std::fs;
use std::path::Path;
use serde_json::Value as Json;
use std::io::{self, Write};
use std::convert::TryFrom;
use std::io::BufWriter;
use std::convert::TryInto;
use std::path::PathBuf;
use serde_json::{Value, Map};
use std::mem;

use crate::common::error::{ErrorContainer, ErrorType};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DataSetType {
    UndefinedType = 0,
    Uint8Type = 1,
    FloatType = 4,
}

impl Default for DataSetType {
    fn default() -> Self {
        DataSetType::UndefinedType
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NameEntry {
    pub name: [u8; 255],
    pub name_size: u8,
}

impl NameEntry {
    pub fn new() -> Self {
        NameEntry {
            name: [0; 255],
            name_size: 0,
        }
    }

    pub fn get_name(&self) -> String {
        if self.name_size == 0 || self.name_size > 254 {
            return "".to_string();
        }

        match str::from_utf8(&self.name[..self.name_size as usize]) {
            Ok(s) => s.to_string(),
            Err(_) => "".to_string(),
        }
    }

    pub fn set_name(&mut self, new_name: &str) -> bool {
        let bytes = new_name.as_bytes();
        let len = bytes.len();

        if len == 0 || len > 254 {
            return false;
        }

        self.name[..len].copy_from_slice(bytes);
        self.name[len] = 0;
        self.name_size = len as u8;

        true
    }
}

impl PartialEq for NameEntry {
    fn eq(&self, other: &Self) -> bool {
        self.name_size == other.name_size &&
            &self.name[..self.name_size as usize] == &other.name[..other.name_size as usize]
    }
}
impl Eq for NameEntry {}

impl Serialize for NameEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        serializer.serialize_str(&self.get_name())
    }
}

#[repr(C)]
pub struct DataSetHeader {
    pub type_identifier: [u8; 8],
    pub file_identifier: [u8; 32],
    pub version: [u8; 8],
    pub minor_version: [u8; 8],

    pub data_type: DataSetType,
    pub type_size: u8,
    pub padding1: [u8; 2],
    pub description_size: u32,

    pub file_size: u64,
    pub number_of_rows: u64,
    pub number_of_columns: u64,

    pub name: NameEntry,

    pub padding2: [u8; 3752],
}

impl DataSetHeader {
    pub fn new() -> Self {
        DataSetHeader {
            type_identifier: *b"hanami\0\0",
            file_identifier: {
                let mut buf = [0u8; 32];
                buf[..8].copy_from_slice(b"data-set");
                buf
            },
            version: *b"v1\0\0\0\0\0\0",
            minor_version: *b"0alpha\0\0",

            data_type: DataSetType::UndefinedType,
            type_size: 1,
            padding1: [0; 2],
            description_size: 0,

            file_size: 0,
            number_of_rows: 0,
            number_of_columns: 0,

            name: NameEntry::new(),

            padding2: [0; 3752],
        }
    }
}

const _: () = assert!(std::mem::size_of::<NameEntry>() == 256);
const _: () = assert!(std::mem::size_of::<DataSetHeader>() == 4096);


pub struct DataSetFileHandle {
    pub header: DataSetHeader,
    pub target_file: BufWriter<fs::File>,
    pub description: Json,
}

pub fn init_new_data_set_file(
    file_path: &PathBuf,
    name: &String,
    description: &Json,
    data_type: DataSetType,
    number_of_columns: u64,
) -> Result<DataSetFileHandle, ErrorContainer> {

    let file_path_str: String = file_path.to_string_lossy().into();

    // check give dataset-type
    if data_type == DataSetType::UndefinedType {
        return Err(ErrorContainer {
            error_type: ErrorType::InvalidInput,
            msg: "Invalid dataset-type".to_string(),
        });
    }

    // check if file already exist
    if Path::new(file_path).exists() {
        return Err(ErrorContainer {
            error_type: ErrorType::InvalidInput,
            msg: format!("Data-set file '{}' already exists.", file_path_str),
        });
    }

    // convert description into string
    let description_str = match serde_json::to_string(description) {
        Ok(s) => s,
        Err(_) => {
            return Err(ErrorContainer {
                error_type: ErrorType::InvalidInput,
                msg: "Failed to serialize description".to_string(),
            });
        }
    };

    // initialize file
    let file = match fs::File::create(file_path) {
        Ok(f) => f,
        Err(e) => {
            return Err(ErrorContainer {
                error_type: ErrorType::Error,
                msg: format!("Failed to open data-set file '{}' with error: {}.", file_path_str, e),
            });
        }
    };

    let mut result = DataSetFileHandle {
        header: DataSetHeader::new(),
        target_file: BufWriter::new(file),
        description: description.clone(),
    };

    // update header
    result.header.description_size = description_str.len().try_into().expect("usize doesn't fit into u32");
    result.header.data_type = data_type;
    result.header.number_of_columns = number_of_columns;
    result.header.type_size = data_type as u8;

    // add name to header
    if !result.header.name.set_name(name) {
        return Err(ErrorContainer {
            error_type: ErrorType::InvalidInput,
            msg: format!("New data-set name '{}' is invalid", name),
        });
    }

    // convert header into a byte-array to write it into the file
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(
            &result.header as *const DataSetHeader as *const u8,
            mem::size_of::<DataSetHeader>(),
        )
    };

    // write header to file
    match result.target_file.write_all(bytes) {
        Ok(_) => {},
        Err(e) => {
            return Err(ErrorContainer {
                error_type: ErrorType::Error,
                msg: format!("Failed to write data-set file '{}' with error: {}.", file_path_str, e),
            });
        }
    };

    // write description into file
    let json_string = result.description.to_string();
    match result.target_file.write_all(json_string.as_bytes()) {
        Ok(_) => {},
        Err(e) => {
            return Err(ErrorContainer {
                error_type: ErrorType::Error,
                msg: format!("Failed to write data-set file '{}' with error: {}.", file_path_str, e),
            });
        }
    };

    Ok(result)
}

