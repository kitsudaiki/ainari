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

use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};
use uuid::Uuid;

use ainari_common::enums::*;
use ainari_common::error::AinariError;
use ainari_model_parser::model_meta_structs::*;

use crate::core::blocks::core_block::*;
use crate::core::blocks::input_block::*;
use crate::core::blocks::output_block::*;
use crate::core::processing::finish_counter::FinishCounter;
use crate::core::processing::output_buffer::OutputBuffer;

use super::blocks::block_trait::Block;
use super::model_interface::ModelInterface;

lazy_static::lazy_static! {
    /// Global singleton for model data handling.
    ///
    /// This provides thread-safe access to all models and their components.
    pub static ref MODEL_HANDLER: RwLock<ModelDataHandler> = RwLock::new(init_model_data_handler());
}

// ==================================================================================================

/// Represents a hexagon in the model graph containing multiple blocks.
///
/// A hexagon groups related blocks together in the model's processing graph.
pub struct HexagonData {
    /// Map of block UUIDs to their corresponding block instances.
    pub blocks: HashMap<Uuid, Arc<Mutex<dyn Block>>>,
}

impl HexagonData {
    /// Creates a new empty HexagonData instance.
    pub fn new() -> Self {
        HexagonData {
            blocks: HashMap::new(),
        }
    }
}

// ==================================================================================================

/// Contains all data for a single model including its structure and components.
///
/// This struct holds the complete state of a model including its metadata,
/// processing blocks, input/output buffers, and interface.
pub struct ModelContent {
    /// Metadata describing the model's structure and configuration.
    pub model_meta: ModelMeta,
    /// Map of hexagon UUIDs to their corresponding HexagonData instances.
    pub hexagon_data: RwLock<HashMap<Uuid, Arc<Mutex<HexagonData>>>>,
    /// Map of input names to their corresponding InputBlock instances.
    pub inputs: RwLock<HashMap<String, Arc<Mutex<InputBlock>>>>,
    /// Map of output names to their corresponding OutputBuffer instances.
    pub outputs: RwLock<HashMap<String, Arc<Mutex<OutputBuffer>>>>,
    /// Optional interface for interacting with the model.
    pub model_interface: Option<Arc<Mutex<ModelInterface>>>,
}

impl ModelContent {
    /// Creates a new ModelContent instance with the given metadata.
    ///
    /// # Arguments
    /// * `model_meta` - Metadata describing the model's structure and configuration.
    pub fn new(model_meta: ModelMeta) -> Self {
        ModelContent {
            model_meta,
            hexagon_data: RwLock::new(HashMap::new()),
            inputs: RwLock::new(HashMap::new()),
            outputs: RwLock::new(HashMap::new()),
            model_interface: None,
        }
    }
}

// ==================================================================================================

/// Main handler for managing multiple models and their components.
///
/// This struct provides functionality for creating, accessing, and manipulating models
/// and their associated blocks, inputs, and outputs.
pub struct ModelDataHandler {
    /// Map of model UUIDs to their corresponding ModelContent instances.
    pub models: HashMap<Uuid, ModelContent>,
}

// ==================================================================================================

/// Initializes a new empty ModelDataHandler instance.
///
/// # Returns
/// A new ModelDataHandler with an empty models map.
pub fn init_model_data_handler() -> ModelDataHandler {
    ModelDataHandler {
        models: HashMap::new(),
    }
}

// ==================================================================================================

impl ModelDataHandler {
    /// Initializes a new model with the given metadata and UUID.
    ///
    /// This creates a complete model structure including all blocks, inputs, and outputs.
    ///
    /// # Arguments
    /// * `model_uuid` - UUID of the model to initialize.
    /// * `parsed_model` - Metadata describing the model's structure and configuration.
    ///
    /// # Returns
    /// * `Ok(())` on success.
    /// * `Err(AinariError)` if the model already exists or if initialization fails.
    pub fn init_new_model(
        &mut self,
        model_uuid: &Uuid,
        parsed_model: &ModelMeta,
    ) -> Result<(), AinariError> {
        // get and init finish-counter
        let finish_counter_mutex = Arc::new(Mutex::new(FinishCounter::default()));
        let mut finish_counter = finish_counter_mutex.lock().expect("mutex poisoned");
        let interface = Arc::new(Mutex::new(ModelInterface::new(
            model_uuid,
            &finish_counter_mutex,
        )));

        // add model to the model-handler
        self.register_model(parsed_model, Some(interface))?;

        // initialize input-blocks
        for input_meta in parsed_model.inputs.iter() {
            let input_block_mutex = Arc::new(Mutex::new(InputBlock::new(
                &input_meta.name,
                &input_meta.hexagon_uuid,
                model_uuid,
                &finish_counter_mutex,
            )));
            self.add_input_block(&input_block_mutex)?;
        }
        finish_counter.input_compare = parsed_model.inputs.len();

        // initilize output-buffer
        for output_meta in parsed_model.outputs.iter() {
            let output_buffer_mutex = Arc::new(Mutex::new(OutputBuffer::new(
                &output_meta.name,
                &output_meta.hexagon_uuid,
                model_uuid,
                &output_meta.output_type,
                &finish_counter_mutex,
            )));
            self.add_output_buffer(&output_buffer_mutex)?;
        }
        finish_counter.output_compare = parsed_model.outputs.len();

        Ok(())
    }

    /// Registers a new model with the given metadata and optional interface.
    ///
    /// # Arguments
    /// * `model_meta` - Metadata describing the model's structure and configuration.
    /// * `interface` - Optional interface for interacting with the model.
    ///
    /// # Returns
    /// * `Ok(())` on success.
    /// * `Err(AinariError)` if the model already exists.
    pub fn register_model(
        &mut self,
        model_meta: &ModelMeta,
        interface: Option<Arc<Mutex<ModelInterface>>>,
    ) -> Result<(), AinariError> {
        if self.models.contains_key(&model_meta.uuid) {
            let msg = format!("Model with uuid '{}' already exist.", model_meta.uuid);
            return Err(AinariError::InvalidInput(msg));
        }

        let model_uuid = model_meta.uuid;
        let mut content = ModelContent::new(model_meta.clone());
        content.model_interface = interface;

        self.models.insert(model_uuid, content);

        Ok(())
    }

    /// Gets an immutable reference to a model by its UUID.
    ///
    /// # Arguments
    /// * `model_uuid` - UUID of the model to retrieve.
    ///
    /// # Returns
    /// * `Ok(&ModelContent)` on success.
    /// * `Err(AinariError)` if the model doesn't exist.
    pub fn get_model(&self, model_uuid: &Uuid) -> Result<&ModelContent, AinariError> {
        if let Some(model) = self.models.get(model_uuid) {
            Ok(model)
        } else {
            let msg = format!("Model with uuid '{model_uuid}' not found.");
            Err(AinariError::InvalidInput(msg))
        }
    }

    /// Gets a mutable reference to a model by its UUID.
    ///
    /// # Arguments
    /// * `model_uuid` - UUID of the model to retrieve.
    ///
    /// # Returns
    /// * `Ok(&mut ModelContent)` on success.
    /// * `Err(AinariError)` if the model doesn't exist.
    pub fn get_model_mut(&mut self, model_uuid: &Uuid) -> Result<&mut ModelContent, AinariError> {
        if let Some(model) = self.models.get_mut(model_uuid) {
            Ok(model)
        } else {
            let msg = format!("Model with uuid '{model_uuid}' not found.");
            Err(AinariError::InvalidInput(msg))
        }
    }

    /// Adds a core block to the specified model.
    ///
    /// # Arguments
    /// * `block_mutex` - The core block to add, wrapped in an Arc<Mutex>.
    ///
    /// # Returns
    /// * `Ok(())` on success.
    /// * `Err(AinariError)` if the block already exists or if the model doesn't exist.
    pub fn add_core_block(
        &mut self,
        block_mutex: &Arc<Mutex<CoreBlock>>,
    ) -> Result<(), AinariError> {
        return self.add_block(&(block_mutex.clone() as Arc<Mutex<dyn Block>>));
    }

    /// Adds an output block to the model.
    ///
    /// This is a convenience method that wraps `add_block` for output blocks.
    /// It clones the block mutex and casts it to a generic block mutex before passing it to `add_block`.
    ///
    /// # Arguments
    ///
    /// * `block_mutex` - A reference to an Arc-wrapped Mutex containing the OutputBlock to add
    ///
    /// # Returns
    ///
    /// * `Result<(), AinariError>` - Ok if the block was added successfully, Err otherwise
    pub fn add_output_block(
        &mut self,
        block_mutex: &Arc<Mutex<OutputBlock>>,
    ) -> Result<(), AinariError> {
        return self.add_block(&(block_mutex.clone() as Arc<Mutex<dyn Block>>));
    }

    /// Adds an input block to the model.
    ///
    /// This method adds an input block to both the specified hexagon and the model's input collection.
    /// It performs several checks to ensure the block doesn't already exist in either location.
    ///
    /// # Arguments
    ///
    /// * `block_mutex` - A reference to an Arc-wrapped Mutex containing the InputBlock to add
    ///
    /// # Returns
    ///
    /// * `Result<(), AinariError>` - Ok if the block was added successfully, Err otherwise
    pub fn add_input_block(
        &mut self,
        block_mutex: &Arc<Mutex<InputBlock>>,
    ) -> Result<(), AinariError> {
        let input_block = block_mutex.lock().expect("mutex poisoned");
        let model_uuid = input_block.get_model_uud();
        let hexagon_uuid = input_block.get_hexagon_uud();
        let block_name = input_block.name.clone();
        let block_uuid = input_block.get_uuid();

        let model_link = self.get_model_mut(&model_uuid)?;

        // add hexagon, if not already exist
        let mut hexagon_data_map = model_link.hexagon_data.write().expect("mutex poisoned");
        hexagon_data_map
            .entry(hexagon_uuid)
            .or_insert_with(|| Arc::new(Mutex::new(HexagonData::new())));

        // get hexagon
        let mut hexgon_link = if let Some(h) = hexagon_data_map.get_mut(&hexagon_uuid) {
            h.lock().expect("mutex poisoned")
        } else {
            let msg = format!("Hexagon with uuid '{hexagon_uuid}' not found.");
            return Err(AinariError::InvalidInput(msg));
        };

        // check if block with name already exist
        let mut inputs = model_link.inputs.write().expect("mutex poisoned");
        if inputs.contains_key(&block_name) {
            let msg = format!("Input-block with name '{block_name}' already exist.");
            return Err(AinariError::InvalidInput(msg));
        }
        if hexgon_link.blocks.contains_key(&block_uuid) {
            let msg = format!("Input-block with name '{block_name}' already exist.");
            return Err(AinariError::InvalidInput(msg));
        }

        // add new block
        hexgon_link
            .blocks
            .insert(block_uuid, Arc::clone(block_mutex) as Arc<Mutex<dyn Block>>);
        inputs.insert(block_name.clone(), Arc::clone(block_mutex));

        Ok(())
    }

    /// Adds an output buffer to the model.
    ///
    /// This method adds an output buffer to the model's output collection.
    /// It checks if a buffer with the same name already exists.
    ///
    /// # Arguments
    ///
    /// * `block_mutex` - A reference to an Arc-wrapped Mutex containing the OutputBuffer to add
    ///
    /// # Returns
    ///
    /// * `Result<(), AinariError>` - Ok if the buffer was added successfully, Err otherwise
    pub fn add_output_buffer(
        &mut self,
        block_mutex: &Arc<Mutex<OutputBuffer>>,
    ) -> Result<(), AinariError> {
        let output_buffer = block_mutex.lock().expect("mutex poisoned");
        let model_uuid = output_buffer.model_uuid;
        let name = output_buffer.name.clone();

        let model_link = self.get_model_mut(&model_uuid)?;

        // get hexagon-io
        let mut outputs = model_link.outputs.write().expect("mutex poisoned");
        if outputs.contains_key(&name) {
            let msg = format!("Output-buffer with name '{name}' already exist.");
            return Err(AinariError::InvalidInput(msg));
        }

        outputs.insert(name.clone(), Arc::clone(block_mutex));

        Ok(())
    }

    /// Generic method to add a block to a hexagon in the model.
    ///
    /// This method adds any type of block (input, core, output) to a hexagon in the specified model.
    /// It performs checks to ensure the block doesn't already exist in the hexagon.
    ///
    /// # Arguments
    ///
    /// * `block_mutex` - A reference to an Arc-wrapped Mutex containing the Block to add
    ///
    /// # Returns
    ///
    /// * `Result<(), AinariError>` - Ok if the block was added successfully, Err otherwise
    fn add_block(&mut self, block_mutex: &Arc<Mutex<dyn Block>>) -> Result<(), AinariError> {
        let block = block_mutex.lock().expect("mutex poisoned");
        let model_uuid = block.get_model_uud();
        let hexagon_uuid = block.get_hexagon_uud();
        let block_uuid = block.get_uuid();

        let model_link = self.get_model_mut(&model_uuid)?;

        // get hexagon from model
        let mut hexagon_data = model_link.hexagon_data.write().expect("mutex poisoned");
        hexagon_data
            .entry(hexagon_uuid)
            .or_insert_with(|| Arc::new(Mutex::new(HexagonData::new())));

        let mut hexgon_link = if let Some(h) = hexagon_data.get_mut(&hexagon_uuid) {
            h.lock().expect("mutex poisoned")
        } else {
            let msg = format!("Hexagon with uuid '{hexagon_uuid}' not found.");
            return Err(AinariError::InvalidInput(msg));
        };

        // add new block
        if hexgon_link.blocks.contains_key(&block_uuid) {
            let msg = format!("Block with uuid '{block_uuid}' already exist.");
            return Err(AinariError::InvalidInput(msg));
        }

        hexgon_link
            .blocks
            .insert(block_uuid, Arc::clone(block_mutex));
        Ok(())
    }

    /// Retrieves the model interface for a given model.
    ///
    /// This method returns the model interface associated with the specified model UUID.
    /// The interface is used to manage the model's execution state and other control functions.
    ///
    /// # Arguments
    ///
    /// * `model_uuid` - A reference to the UUID of the model
    ///
    /// # Returns
    ///
    /// * `Result<Arc<Mutex<ModelInterface>>, AinariError>` - The model interface if found, Err otherwise
    pub fn get_model_interface(
        &self,
        model_uuid: &Uuid,
    ) -> Result<Arc<Mutex<ModelInterface>>, AinariError> {
        let model_link = self.get_model(model_uuid)?;
        if let Some(model_interface_mutex) = &model_link.model_interface {
            Ok(model_interface_mutex.clone())
        } else {
            let msg = format!("No interface for '{model_uuid}' not found.");
            Err(AinariError::InvalidInput(msg))
        }
    }

    /// Retrieves the finish counter for a given model.
    ///
    /// The finish counter is used to track the completion status of model execution.
    ///
    /// # Arguments
    ///
    /// * `model_uuid` - A reference to the UUID of the model
    ///
    /// # Returns
    ///
    /// * `Result<Arc<Mutex<FinishCounter>>, AinariError>` - The finish counter if found, Err otherwise
    #[allow(dead_code)]
    pub fn get_finish_counter(
        &self,
        model_uuid: &Uuid,
    ) -> Result<Arc<Mutex<FinishCounter>>, AinariError> {
        let model_interface_mutex = self.get_model_interface(model_uuid)?;
        let model_interface = model_interface_mutex.lock().expect("mutex poisoned");

        Ok(model_interface.finish_counter_mutex.clone())
    }

    /// Retrieves a block from a model's hexagon.
    ///
    /// This method finds a specific block within a hexagon of a model using the provided UUIDs.
    ///
    /// # Arguments
    ///
    /// * `model_uuid` - A reference to the UUID of the model
    /// * `hexagon_uuid` - A reference to the UUID of the hexagon containing the block
    /// * `block_uuid` - A reference to the UUID of the block to retrieve
    ///
    /// # Returns
    ///
    /// * `Result<Arc<Mutex<dyn Block>>, AinariError>` - The block if found, Err otherwise
    pub fn get_block(
        &self,
        model_uuid: &Uuid,
        hexagon_uuid: &Uuid,
        block_uuid: &Uuid,
    ) -> Result<Arc<Mutex<dyn Block>>, AinariError> {
        let model_link = self.get_model(model_uuid)?;

        let binding = model_link.hexagon_data.read().expect("mutex poisoned");
        let hexagon_link = if let Some(h) = binding.get(hexagon_uuid) {
            h.lock().expect("mutex poisoned")
        } else {
            let msg = format!("Hexagon with uuid '{hexagon_uuid}' not found.");
            return Err(AinariError::InvalidInput(msg));
        };

        if let Some(block_mutex) = hexagon_link.blocks.get(block_uuid) {
            return Ok(block_mutex.clone());
        }

        let msg = format!("Block with uuid '{block_uuid}' not found.");
        Err(AinariError::InvalidInput(msg))
    }

    /// Retrieves an input block from a model by its name.
    ///
    /// # Arguments
    ///
    /// * `model_uuid` - A reference to the UUID of the model
    /// * `name` - A reference to the name of the input block
    ///
    /// # Returns
    ///
    /// * `Result<Arc<Mutex<InputBlock>>, AinariError>` - The input block if found, Err otherwise
    pub fn get_input_block(
        &self,
        model_uuid: &Uuid,
        name: &String,
    ) -> Result<Arc<Mutex<InputBlock>>, AinariError> {
        let model_link = self.get_model(model_uuid)?;

        let binding = model_link.inputs.read().expect("mutex poisoned");
        if let Some(input_block_mutex) = binding.get(name) {
            Ok(input_block_mutex.clone())
        } else {
            let msg = format!("Input-Block with name '{name}' not found.");
            Err(AinariError::InvalidInput(msg))
        }
    }

    /// Retrieves an output buffer from a model by its name.
    ///
    /// # Arguments
    ///
    /// * `model_uuid` - A reference to the UUID of the model
    /// * `name` - A reference to the name of the output buffer
    ///
    /// # Returns
    ///
    /// * `Result<Arc<Mutex<OutputBuffer>>, AinariError>` - The output buffer if found, Err otherwise
    pub fn get_output_buffer(
        &self,
        model_uuid: &Uuid,
        name: &String,
    ) -> Result<Arc<Mutex<OutputBuffer>>, AinariError> {
        let model_link = self.get_model(model_uuid)?;

        let binding = model_link.outputs.read().expect("mutex poisoned");
        if let Some(output_buffer_mutex) = binding.get(name) {
            Ok(output_buffer_mutex.clone())
        } else {
            let msg = format!("Output-buffer with name '{name}' not found.");
            Err(AinariError::InvalidInput(msg))
        }
    }

    /// Deletes a block from a model's hexagon.
    ///
    /// This method removes a block from a hexagon and cleans up the hexagon if it becomes empty.
    ///
    /// # Arguments
    ///
    /// * `model_uuid` - A reference to the UUID of the model
    /// * `hexagon_uuid` - A reference to the UUID of the hexagon containing the block
    /// * `block_uuid` - A reference to the UUID of the block to delete
    ///
    /// # Returns
    ///
    /// * `Result<(), AinariError>` - Ok if the block was deleted successfully, Err otherwise
    #[allow(dead_code)]
    pub fn delete_block(
        &mut self,
        model_uuid: &Uuid,
        hexagon_uuid: &Uuid,
        block_uuid: &Uuid,
    ) -> Result<(), AinariError> {
        let model_link = self.get_model_mut(model_uuid)?;

        // get hexagon from model
        let mut binding = model_link.hexagon_data.write().expect("mutex poisoned");
        let mut hexagon_link = if let Some(h) = binding.get_mut(hexagon_uuid) {
            h.lock().expect("mutex poisoned")
        } else {
            let msg = format!("Hexagon with uuid '{hexagon_uuid}' not found.");
            return Err(AinariError::InvalidInput(msg));
        };

        // delete block
        if !hexagon_link.blocks.contains_key(block_uuid) {
            let msg = format!("Block with uuid '{block_uuid}' not found.");
            return Err(AinariError::InvalidInput(msg));
        }
        hexagon_link.blocks.remove(block_uuid);

        // remove hexagon, if it doesn't contain any more blocks
        if hexagon_link.blocks.is_empty() {
            model_link
                .hexagon_data
                .write()
                .expect("mutex poisoned")
                .remove(hexagon_uuid);
        }

        Ok(())
    }

    /// Deletes a model from the collection.
    ///
    /// This method removes a model and all its associated data from the collection.
    ///
    /// # Arguments
    ///
    /// * `model_uuid` - A reference to the UUID of the model to delete
    ///
    /// # Returns
    ///
    /// * `Result<(), AinariError>` - Ok if the model was deleted successfully, Err otherwise
    pub fn delete_model(&mut self, model_uuid: &Uuid) -> Result<(), AinariError> {
        if !self.models.contains_key(model_uuid) {
            let msg: String = format!("Model with uuid '{model_uuid}' not found.");
            return Err(AinariError::InvalidInput(msg));
        }

        self.models.remove(model_uuid);

        Ok(())
    }

    /// Resets all output buffers in a model.
    ///
    /// This method calls the reset method on all output buffers in the specified model.
    ///
    /// # Arguments
    ///
    /// * `model_uuid` - A reference to the UUID of the model
    ///
    /// # Returns
    ///
    /// * `Result<(), AinariError>` - Ok if all outputs were reset successfully, Err otherwise
    pub fn reset_outputs(&self, model_uuid: &Uuid) -> Result<(), AinariError> {
        let model_link = self.get_model(model_uuid)?;
        let outputs = model_link.outputs.read().expect("mutex poisoned");
        for output_mutex in outputs.values() {
            let mut output = output_mutex.lock().expect("mutex poisoned");
            output.reset_output();
        }

        Ok(())
    }

    /// Writes a serializable struct to a file using bincode serialization.
    ///
    /// This function serializes the given struct into a binary format and writes it to the specified
    /// file along with a type identifier and length prefix.
    ///
    /// # Arguments
    ///
    /// * `writer` - A mutable reference to a buffered writer for the output file
    /// * `struct_type` - The type identifier for the struct being written
    /// * `value` - A reference to the struct to be serialized and written
    ///
    /// # Returns
    ///
    /// * `Ok(())` on success
    /// * An error if serialization or writing fails
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails or if there are issues writing to the file.
    fn write_struct_to_file<T: Serialize>(
        &self,
        writer: &mut BufWriter<fs::File>,
        struct_type: ObjectType,
        value: &T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create bincode configuration
        let cfg = bincode::config::standard();
        // Serialize the struct to a byte vector
        let data = bincode::serde::encode_to_vec(value, cfg)?;
        // Get the length of the serialized data
        let len = data.len() as u32;

        // Write type identifier, length, and data to the file
        writer.write_all(&[struct_type.to_u8()])?;
        writer.write_all(&len.to_le_bytes())?;
        writer.write_all(&data)?;

        Ok(())
    }

    /// Writes a byte vector to a file with type identifier and length prefix.
    ///
    /// This function writes raw binary data to a file with a type identifier and length prefix
    /// similar to the struct writing function.
    ///
    /// # Arguments
    ///
    /// * `writer` - A mutable reference to a buffered writer for the output file
    /// * `struct_type` - The type identifier for the data being written
    /// * `data` - The byte vector to be written
    ///
    /// # Returns
    ///
    /// * `Ok(())` on success
    /// * An error if writing fails
    ///
    /// # Errors
    ///
    /// Returns an error if there are issues writing to the file.
    fn write_vec_to_file(
        &self,
        writer: &mut BufWriter<fs::File>,
        struct_type: ObjectType,
        data: Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Get the length of the data
        let len = data.len() as u32;

        // Write type identifier, length, and data to the file
        writer.write_all(&[struct_type.to_u8()])?;
        writer.write_all(&len.to_le_bytes())?;
        writer.write_all(&data)?;

        Ok(())
    }

    /// Creates a checkpoint of a model by serializing its components to a file.
    ///
    /// This function serializes the model's metadata, blocks, and output buffers to a checkpoint
    /// file for later restoration.
    ///
    /// # Arguments
    ///
    /// * `model_uuid` - The UUID of the model to checkpoint
    /// * `local_temp_file_path` - The path where the checkpoint file will be created
    ///
    /// # Returns
    ///
    /// * `Ok(())` on success
    /// * An error if the model is not found, the file already exists, or serialization fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The model with the specified UUID is not found
    /// * The checkpoint file already exists
    /// * There are issues creating or writing to the file
    pub fn create_checkpoint(
        &self,
        model_uuid: &Uuid,
        local_temp_file_path: &String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // get model
        let model_link = if let Some(c) = self.models.get(model_uuid) {
            c
        } else {
            let msg = format!("Model with uuid '{model_uuid}' not found.");
            return Err(Box::new(AinariError::InternalError(msg)));
        };

        // check if file already exist
        if Path::new(&local_temp_file_path).exists() {
            let msg = format!("Checkpoint file '{local_temp_file_path}' already exists.");
            // HINT (kitsudaki): the path is defined by the backend itself and not by the user,
            // so here should be an internal error instand of an input-error
            return Err(Box::new(AinariError::InternalError(msg)));
        }

        // initialize file
        let file = fs::File::create(local_temp_file_path)?;
        let mut target_file = BufWriter::new(file);

        // write model-meta into checkpoint-file
        self.write_struct_to_file(
            &mut target_file,
            ObjectType::ModelMeta,
            &model_link.model_meta,
        )?;

        // write blocks into checkpoint-file
        let hexagon_data = model_link.hexagon_data.read().expect("mutex poisoned");
        for hexagon in hexagon_data.values() {
            for block_mutex in hexagon.lock().expect("mutex poisoned").blocks.values() {
                let block = block_mutex.lock().expect("mutex poisoned");
                self.write_vec_to_file(&mut target_file, block.get_type(), block.serailize())?;
            }
        }

        // write output-buffers into checkpoint-file
        let outputs = model_link.outputs.read().expect("mutex poisoned");
        for output_mutex in outputs.values() {
            let output = output_mutex.lock().expect("mutex poisoned");
            self.write_vec_to_file(
                &mut target_file,
                ObjectType::OutputBuffer,
                output.serailize(),
            )?;
        }

        Ok(())
    }

    /// Restores a model from a checkpoint file.
    ///
    /// This function reads a checkpoint file and reconstructs a model with all its components.
    ///
    /// # Arguments
    ///
    /// * `model_uuid` - The UUID of the model to restore
    /// * `local_temp_file_path` - The path to the checkpoint file
    ///
    /// # Returns
    ///
    /// * `Ok(())` on success
    /// * An error if the checkpoint file is invalid or restoration fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The checkpoint file is invalid
    /// * The model cannot be restored from the checkpoint
    /// * There are issues reading the file
    pub fn restore_checkpoint(
        &mut self,
        model_uuid: &Uuid,
        local_temp_file_path: &String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // in model with this uuid already exist, it fill be removed first in order to create it new from the checkpoint
        let mut model_interface = None;
        if self.models.contains_key(model_uuid) {
            if let Some(model_link) = self.models.get_mut(model_uuid) {
                if let Some(interface) = &model_link.model_interface {
                    model_interface = Some(interface.clone());
                }
                model_link.model_interface = None;
            }
            self.models.remove(model_uuid);
        }

        // init file and other components for reading
        let file = fs::File::open(local_temp_file_path)?;
        let mut reader = BufReader::with_capacity(10 * 1024 * 1024, file);
        let cfg = bincode::config::standard();
        let mut len_buf = [0u8; 4];

        // init counter
        let mut finish_counter_mutex = Arc::new(Mutex::new(FinishCounter::default()));
        let mut model_meta_parsed = false;
        let mut number_of_input_blocks = 0;
        let mut number_of_output_buffer = 0;

        loop {
            // try to read the type byte
            let mut type_buf = [0u8; 1];
            match reader.read_exact(&mut type_buf) {
                Ok(()) => {} // Got a byte, continue
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                    // EOF reached, no more objects!
                    break;
                }
                Err(e) => return Err(e.into()),
            }

            let struct_type =
                ObjectType::from_u8(type_buf[0]).ok_or("Unknown struct type in stream")?;

            // read length (4 bytes)
            reader.read_exact(&mut len_buf)?;
            let len = u32::from_le_bytes(len_buf) as usize;

            // read data bytes
            let mut data_buf = vec![0u8; len];
            reader.read_exact(&mut data_buf)?;

            // deserialize based on type
            match struct_type {
                // Unknown
                ObjectType::Unknown => {
                    let msg = "Invalid object found in checkpoint-file.".to_string();
                    return Err(Box::new(AinariError::InvalidInput(msg)));
                }
                // ModelMeta
                ObjectType::ModelMeta => {
                    if model_meta_parsed {
                        let msg = "File has multiple model-meta objects.".to_string();
                        return Err(Box::new(AinariError::InvalidInput(msg)));
                    }
                    let mut model_meta: ModelMeta =
                        bincode::serde::decode_from_slice(&data_buf, cfg)
                            .expect("Failed to deserialize")
                            .0;
                    model_meta.uuid = *model_uuid;
                    if let Some(interface_mutex) = &model_interface {
                        {
                            let interface = interface_mutex.lock().expect("mutex poisoned");
                            finish_counter_mutex = Arc::clone(&interface.finish_counter_mutex);
                        }
                        self.register_model(&model_meta, Some(interface_mutex.clone()))?;
                    } else {
                        let interface = Arc::new(Mutex::new(ModelInterface::new(
                            model_uuid,
                            &finish_counter_mutex,
                        )));
                        self.register_model(&model_meta, Some(interface))?;
                    }
                    model_meta_parsed = true;
                }
                // HexagonData
                ObjectType::HexagonData => {
                    let msg = "Invalid object found in checkpoint-file.".to_string();
                    return Err(Box::new(AinariError::InvalidInput(msg)));
                }
                // InputBlock
                ObjectType::InputBlock => {
                    if !model_meta_parsed {
                        let msg =
                            "File has no model-meta object at the starting position.".to_string();
                        return Err(Box::new(AinariError::InvalidInput(msg)));
                    }
                    let mut input_block: InputBlock =
                        bincode::serde::decode_from_slice(&data_buf, cfg)
                            .expect("Failed to deserialize")
                            .0;
                    input_block.set_model_uuid(model_uuid);
                    self.add_input_block(&Arc::new(Mutex::new(input_block)))?;
                    number_of_input_blocks += 1;
                }
                // CoreBlock
                ObjectType::CoreBlock => {
                    if !model_meta_parsed {
                        let msg =
                            "File has no model-meta object at the starting position.".to_string();
                        return Err(Box::new(AinariError::InvalidInput(msg)));
                    }
                    let mut core_block: CoreBlock =
                        bincode::serde::decode_from_slice(&data_buf, cfg)
                            .expect("Failed to deserialize")
                            .0;
                    core_block.set_model_uuid(model_uuid);
                    self.add_core_block(&Arc::new(Mutex::new(core_block)))?;
                }
                // OutputBlock
                ObjectType::OutputBlock => {
                    if !model_meta_parsed {
                        let msg =
                            "File has no model-meta object at the starting position.".to_string();
                        return Err(Box::new(AinariError::InvalidInput(msg)));
                    }
                    let mut output_block: OutputBlock =
                        bincode::serde::decode_from_slice(&data_buf, cfg)
                            .expect("Failed to deserialize")
                            .0;
                    output_block.set_model_uuid(model_uuid);
                    self.add_output_block(&Arc::new(Mutex::new(output_block)))?;
                }
                // OutputBuffer
                ObjectType::OutputBuffer => {
                    if !model_meta_parsed {
                        let msg =
                            "File has no model-meta object at the starting position.".to_string();
                        return Err(Box::new(AinariError::InvalidInput(msg)));
                    }
                    let mut output_buffer: OutputBuffer =
                        bincode::serde::decode_from_slice(&data_buf, cfg)
                            .expect("Failed to deserialize")
                            .0;
                    output_buffer.model_uuid = *model_uuid;
                    self.add_output_buffer(&Arc::new(Mutex::new(output_buffer)))?;
                    number_of_output_buffer += 1;
                }
            };
        }

        // set initial values for the finish-counter
        let mut finish_counter = finish_counter_mutex.lock().expect("mutex poisoned");
        finish_counter.input_compare = number_of_input_blocks;
        finish_counter.output_compare = number_of_output_buffer;

        // get model
        let model_link = if let Some(c) = self.models.get(model_uuid) {
            c
        } else {
            let msg = format!("Model with uuid '{model_uuid}' not found after restore.");
            return Err(Box::new(AinariError::InternalError(msg)));
        };

        // connect new finish-counter to inputs
        let inputs = model_link.inputs.read().expect("mutex poisoned");
        for input_mutex in inputs.values() {
            let mut input = input_mutex.lock().expect("mutex poisoned");
            input.finish_counter_mutex = Arc::clone(&finish_counter_mutex);
        }

        // connect new finish-counter to outputs
        let outputs = model_link.outputs.read().expect("mutex poisoned");
        for output_mutex in outputs.values() {
            let mut output = output_mutex.lock().expect("mutex poisoned");
            output.finish_counter_mutex = Arc::clone(&finish_counter_mutex);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ainari_common::enums::*;
    use ainari_model_parser::model_meta_structs::Settings;
    use ainari_model_parser::model_parser::parse_model_template;
    use serial_test::serial;

    use super::*;

    #[test]
    #[serial]
    fn test_create_model() {
        let model_uuid = Uuid::new_v4();
        let name = "test_model".to_string();
        let template = "version: 1 
        settings:
            neuron_cooldown: 1000000000.0;
            refractory_time: 1;
            max_connection_distance: 1;
        hexagons: 
            1,1,1; 
            2,2,2; 
        axons: 
            1,1,1 -> 2,2,2; 
        inputs: 
            key1: 1,1,1; 
        outputs: 
            key2: 2,2,2;"
            .to_string();

        let mut root_handler = MODEL_HANDLER.write().expect("mutex poisoned");
        root_handler.models.clear();

        {
            let mut parsed_model = parse_model_template(&name, &template).unwrap();
            parsed_model.uuid = model_uuid;
            let ret = root_handler.init_new_model(&model_uuid, &parsed_model);
            assert!(ret.is_ok());
            assert_eq!(root_handler.models.len(), 1);
            assert!(root_handler.models.contains_key(&model_uuid));

            let model = root_handler.models.get(&model_uuid).unwrap();
            assert!(model.model_interface.is_some());
            assert_eq!(model.model_meta.uuid, model_uuid);

            // check initial state of hexagon-data
            let hexagons = model.hexagon_data.read().expect("mutex poisoned");
            assert_eq!(hexagons.len(), 1);
        }

        assert!(root_handler.prepare_delete_model(&model_uuid).is_ok());
        assert!(root_handler.prepare_delete_model(&model_uuid).is_err());
    }

    #[test]
    #[serial]
    fn test_add_blocks_to_model() {
        let finish_counter = Arc::new(Mutex::new(FinishCounter::default()));
        let model_uuid = Uuid::new_v4();
        let hexagon_uuid0;
        let hexagon_uuid1;
        let model_name = "test_model".to_string();
        let input_name = "test_input".to_string();
        let output_name = "test_output".to_string();
        let template = "version: 1 
        settings:
            neuron_cooldown: 1000000000.0;
            refractory_time: 1;
            max_connection_distance: 1;
        hexagons: 
            1,1,1; 
            2,2,2; 
        axons: 
            1,1,1 -> 2,2,2; 
        inputs: 
            test_input: 1,1,1; 
        outputs: 
            test_output: 2,2,2;"
            .to_string();

        let mut root_handler = MODEL_HANDLER.write().expect("mutex poisoned");
        root_handler.models.clear();
        let mut parsed_model = parse_model_template(&model_name, &template).unwrap();
        parsed_model.uuid = model_uuid;
        let _ = root_handler.init_new_model(&model_uuid, &parsed_model);

        {
            let model = root_handler.models.get(&model_uuid).unwrap();
            if model.model_meta.hexagons.values().next().unwrap().is_input {
                hexagon_uuid0 = *model.model_meta.hexagons.keys().next().unwrap();
                hexagon_uuid1 = *model.model_meta.hexagons.keys().nth(1).unwrap();
            } else {
                hexagon_uuid1 = *model.model_meta.hexagons.keys().next().unwrap();
                hexagon_uuid0 = *model.model_meta.hexagons.keys().nth(1).unwrap();
            }
        }

        // prepare new blocks
        let settings = Settings::default();
        let core_block = Arc::new(Mutex::new(CoreBlock::new(
            &hexagon_uuid0,
            &model_uuid,
            &settings,
        )));
        let input_block = Arc::new(Mutex::new(InputBlock::new(
            &input_name,
            &hexagon_uuid0,
            &model_uuid,
            &finish_counter,
        )));
        let output_block = Arc::new(Mutex::new(OutputBlock::new(
            &hexagon_uuid1,
            &model_uuid,
            &output_name,
        )));
        let output_buffer = Arc::new(Mutex::new(OutputBuffer::new(
            &output_name,
            &hexagon_uuid1,
            &model_uuid,
            &OutputType::PlainOutput,
            &finish_counter,
        )));

        // input-block and output-buffer are already added by initilizing of the model, so the names can not be added again
        assert!(root_handler.add_output_buffer(&output_buffer).is_err());
        assert!(root_handler.add_input_block(&input_block).is_err());

        // add blocks to model
        assert!(root_handler.add_core_block(&core_block).is_ok());
        assert!(root_handler.add_output_block(&output_block).is_ok());
        {
            let model = root_handler.models.get(&model_uuid).unwrap();
            let hexagons = model.hexagon_data.read().expect("mutex poisoned");
            assert_eq!(hexagons.len(), 2);
            // check hexagon 0
            {
                let hexagon0 = hexagons.get(&hexagon_uuid0).unwrap();
                assert_eq!(hexagon0.lock().expect("mutex poisoned").blocks.len(), 2);
                let inputs = model.inputs.read().expect("mutex poisoned");
                assert!(inputs.contains_key(&input_name));
            }

            // check hexagon 1
            {
                let hexagon1 = hexagons.get(&hexagon_uuid1).unwrap();
                assert_eq!(hexagon1.lock().expect("mutex poisoned").blocks.len(), 1);
                let outputs = model.outputs.read().expect("mutex poisoned");
                assert!(outputs.contains_key(&output_name));
            }
        }

        // check add blocks with the same ids again
        assert!(root_handler.add_core_block(&core_block).is_err());
        assert!(root_handler.add_input_block(&input_block).is_err());
        assert!(root_handler.add_output_block(&output_block).is_err());
        assert!(root_handler.add_output_buffer(&output_buffer).is_err());

        // check getter
        assert!(
            root_handler
                .get_input_block(&model_uuid, &input_name)
                .is_ok()
        );
        assert!(
            root_handler
                .get_input_block(&model_uuid, &output_name)
                .is_err()
        );
        assert!(
            root_handler
                .get_output_buffer(&model_uuid, &input_name)
                .is_err()
        );
        assert!(
            root_handler
                .get_output_buffer(&model_uuid, &output_name)
                .is_ok()
        );
        assert!(
            root_handler
                .get_block(
                    &model_uuid,
                    &hexagon_uuid0,
                    &core_block.lock().expect("mutex poisoned").uuid
                )
                .is_ok()
        );
        assert!(
            root_handler
                .get_block(&model_uuid, &hexagon_uuid1, &Uuid::new_v4())
                .is_err()
        );

        // delete block and check again
        {
            let _ = root_handler.delete_block(
                &model_uuid,
                &hexagon_uuid0,
                &core_block.lock().expect("mutex poisoned").uuid,
            );
            let model = root_handler.models.get(&model_uuid).unwrap();
            let hexagons = model.hexagon_data.read().expect("mutex poisoned");
            let hexagon0 = hexagons.get(&hexagon_uuid0).unwrap();
            assert_eq!(hexagon0.lock().expect("mutex poisoned").blocks.len(), 1);
        }
    }

    #[test]
    #[serial]
    fn test_create_restore_checkpoint() {
        let file_path = "/tmp/test_checkpoint".to_string();
        let _ = fs::remove_file(&file_path).is_ok();
        let finish_counter = Arc::new(Mutex::new(FinishCounter::default()));
        let model_uuid = Uuid::new_v4();
        let model_uuid_new = Uuid::new_v4();
        let hexagon_uuid0;
        let hexagon_uuid1;
        let model_name = "test_model".to_string();
        let input_name = "test_input".to_string();
        let output_name = "test_output".to_string();
        let template = "version: 1 
        settings:
            neuron_cooldown: 1000000000.0;
            refractory_time: 1;
            max_connection_distance: 1;
        hexagons: 
            1,1,1; 
            2,2,2; 
        axons: 
            1,1,1 -> 2,2,2; 
        inputs: 
            test_input: 1,1,1; 
        outputs: 
            test_output: 2,2,2;"
            .to_string();

        let mut root_handler = MODEL_HANDLER.write().expect("mutex poisoned");
        root_handler.models.clear();
        let mut parsed_model = parse_model_template(&model_name, &template).unwrap();
        parsed_model.uuid = model_uuid;
        let _ = root_handler.init_new_model(&model_uuid, &parsed_model);

        {
            let model = root_handler.models.get(&model_uuid).unwrap();
            if model.model_meta.hexagons.values().next().unwrap().is_input {
                hexagon_uuid0 = *model.model_meta.hexagons.keys().next().unwrap();
                hexagon_uuid1 = *model.model_meta.hexagons.keys().nth(1).unwrap();
            } else {
                hexagon_uuid1 = *model.model_meta.hexagons.keys().next().unwrap();
                hexagon_uuid0 = *model.model_meta.hexagons.keys().nth(1).unwrap();
            }
        }

        // prepare new blocks
        let settings = Settings::default();
        let core_block_mutex = Arc::new(Mutex::new(CoreBlock::new(
            &hexagon_uuid0,
            &model_uuid,
            &settings,
        )));
        let input_block_mutex = Arc::new(Mutex::new(InputBlock::new(
            &input_name,
            &hexagon_uuid0,
            &model_uuid,
            &finish_counter,
        )));
        let output_block_mutex = Arc::new(Mutex::new(OutputBlock::new(
            &hexagon_uuid1,
            &model_uuid,
            &output_name,
        )));
        let output_buffer_mutex = Arc::new(Mutex::new(OutputBuffer::new(
            &output_name,
            &hexagon_uuid1,
            &model_uuid,
            &OutputType::PlainOutput,
            &finish_counter,
        )));

        // add blocks to model
        let _ = root_handler.add_core_block(&core_block_mutex);
        let _ = root_handler.add_input_block(&input_block_mutex);
        let _ = root_handler.add_output_block(&output_block_mutex);
        let _ = root_handler.add_output_buffer(&output_buffer_mutex);

        // save and restore
        let _ = root_handler.create_checkpoint(&model_uuid, &file_path);
        let _ = root_handler.restore_checkpoint(&model_uuid_new, &file_path);

        {
            let model = root_handler.models.get(&model_uuid_new).unwrap();
            let hexagons = model.hexagon_data.read().expect("mutex poisoned");
            assert_eq!(hexagons.len(), 2);
            // check hexagon 0
            {
                let hexagon0 = hexagons.get(&hexagon_uuid0).unwrap();
                assert_eq!(hexagon0.lock().expect("mutex poisoned").blocks.len(), 2);
                let inputs = model.inputs.read().expect("mutex poisoned");
                assert!(inputs.contains_key(&input_name));
            }

            // check hexagon 1
            {
                let hexagon1 = hexagons.get(&hexagon_uuid1).unwrap();
                assert_eq!(hexagon1.lock().expect("mutex poisoned").blocks.len(), 1);
                let outputs = model.outputs.read().expect("mutex poisoned");
                assert!(outputs.contains_key(&output_name));
            }
        }

        // check getter
        assert!(
            root_handler
                .get_input_block(&model_uuid_new, &input_name)
                .is_ok()
        );
        assert!(
            root_handler
                .get_input_block(&model_uuid_new, &output_name)
                .is_err()
        );
        assert!(
            root_handler
                .get_output_buffer(&model_uuid_new, &input_name)
                .is_err()
        );
        assert!(
            root_handler
                .get_output_buffer(&model_uuid_new, &output_name)
                .is_ok()
        );
        assert!(
            root_handler
                .get_block(
                    &model_uuid_new,
                    &hexagon_uuid0,
                    &core_block_mutex.lock().expect("mutex poisoned").uuid
                )
                .is_ok()
        );
        assert!(
            root_handler
                .get_block(&model_uuid_new, &hexagon_uuid1, &Uuid::new_v4())
                .is_err()
        );
    }
}
