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

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use super::super::processing::worker_queue::*;
use super::axons::*;
use super::block_io::*;
use super::block_trait::*;

use crate::core::processing::finish_counter::FinishCounter;

use ainari_common::constants::*;
use ainari_common::enums::*;
use ainari_common::error::AinariError;

// ==================================================================================================

/// Represents an input block in the neural network model.
/// This block is responsible for receiving and processing input data.
///
/// # Fields
///
/// * `uuid` - Unique identifier for the block.
/// * `hexagon_uuid` - Identifier for the hexagon this block belongs to.
/// * `model_uuid` - Identifier for the model this block belongs to.
/// * `block_io` - Input/output buffer for the block.
/// * `name` - Name of the block.
/// * `input_links` - Vector of input links to other blocks.
/// * `fill_position` - Current fill position in the output buffer.
/// * `local_finish_counter` - Local counter for tracking completion status.
/// * `finish_counter_mutex` - Shared counter for tracking completion status across threads.
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct InputBlock {
    pub uuid: Uuid,
    pub hexagon_uuid: Uuid,
    pub model_uuid: Uuid,

    pub block_io: BlockIoBuffer,

    pub name: String,
    pub input_links: Vec<u64>,
    pub fill_position: u64,

    pub local_finish_counter: u64,
    #[serde(skip, default = "init_finish_counter")]
    pub finish_counter_mutex: Arc<Mutex<FinishCounter>>,
}

impl PartialEq for InputBlock {
    /// Compares two InputBlock instances for equality.
    /// Two blocks are considered equal if all their fields match.
    ///
    /// # Arguments
    ///
    /// * `other` - The other InputBlock to compare with.
    ///
    /// # Returns
    ///
    /// `true` if the blocks are equal, `false` otherwise.
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
            && self.hexagon_uuid == other.hexagon_uuid
            && self.model_uuid == other.model_uuid
            && self.block_io == other.block_io
            && self.name == other.name
            && self.input_links == other.input_links
            && self.fill_position == other.fill_position
            && self.local_finish_counter == other.local_finish_counter
    }
}

/// Initializes a new FinishCounter with default values.
/// This is used as the default value for the `finish_counter_mutex` field.
///
/// # Returns
///
/// A new Arc<Mutex<FinishCounter>> with default values.
fn init_finish_counter() -> Arc<Mutex<FinishCounter>> {
    Arc::new(Mutex::new(FinishCounter::default()))
}

impl InputBlock {
    /// Creates a new InputBlock instance.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the block.
    /// * `hexagon_uuid` - Identifier for the hexagon this block belongs to.
    /// * `model_uuid` - Identifier for the model this block belongs to.
    /// * `finish_counter` - Shared counter for tracking completion status across threads.
    ///
    /// # Returns
    ///
    /// A new InputBlock instance.
    pub fn new(
        name: &str,
        hexagon_uuid: &Uuid,
        model_uuid: &Uuid,
        finish_counter: &Arc<Mutex<FinishCounter>>,
    ) -> Self {
        let mut block = InputBlock {
            uuid: Uuid::new_v4(),
            hexagon_uuid: *hexagon_uuid,
            model_uuid: *model_uuid,

            name: name.to_owned(),

            block_io: BlockIoBuffer::default(),

            input_links: Vec::new(),

            fill_position: 0,

            local_finish_counter: 0,
            finish_counter_mutex: Arc::clone(finish_counter),
        };

        block.block_io.output_buffer.push(AxonSection::default());

        block
    }

    // ==================================================================================================

    /// Applies input data to the input block.
    ///
    /// # Arguments
    ///
    /// * `input_ptr` - Pointer to the input data.
    /// * `input_size` - Size of the input data.
    /// * `offset` - Offset to apply to the input data.
    /// * `time_length` - Length of the time dimension of the input data.
    /// * `allow_cration` - Whether to allow creation of new input links.
    pub fn apply_input(
        &mut self,
        input_ptr: &[f32],
        input_size: usize,
        offset: usize,
        time_length: usize,
        allow_cration: bool,
    ) {
        // resize links, if necessary
        let maximum_size = input_size * 2 * time_length;
        if self.input_links.len() < maximum_size {
            self.input_links.resize(maximum_size, UNINIT_STATE_64);
        }

        let mut is_negative;
        let mut total_position;

        if allow_cration {
            // update links
            for (i, val) in input_ptr.iter().enumerate().take(input_size) {
                total_position = (offset + i) * 2;

                if *val != 0.0f32 && self.input_links[total_position] == UNINIT_STATE_64 {
                    is_negative = (*val < 0.0f32) as usize;
                    self.input_links[total_position + is_negative] = self.fill_position;
                    self.fill_position += 1;
                }
            }

            // resize axon-bocks of the input, if necessary
            while self.fill_position > (self.block_io.output_buffer.len() * BLOCK_DIM) as u64 {
                self.block_io.output_buffer.push(AxonSection::default());
            }
        }

        // apply input to the axon-sections
        for (i, val) in input_ptr.iter().enumerate().take(input_size) {
            total_position = (offset + i) * 2;
            is_negative = (*val < 0.0f32) as usize;
            let target = self.input_links[total_position + is_negative];
            if target != UNINIT_STATE_64 {
                let block_pos = target as usize / BLOCK_DIM;
                let pos_in_block = target as usize % BLOCK_DIM;
                self.block_io.output_buffer[block_pos].data.axons[pos_in_block].potential =
                    val.abs();
            }
        }
    }
}

impl Block for InputBlock {
    /// Performs training on the block.
    ///
    /// # Arguments
    ///
    /// * `_` - Unused parameter (reserved for future use).
    /// * `_` - Unused parameter (reserved for future use).
    /// * `_` - Unused parameter (reserved for future use).
    ///
    /// # Returns
    ///
    /// Result containing an optional FinishCounter or an AinariError.
    fn train(
        &mut self,
        _: usize,
        _: Arc<Mutex<dyn Block>>,
        _: u64,
    ) -> Result<Option<Arc<Mutex<FinishCounter>>>, AinariError> {
        self.local_finish_counter = 0;
        Ok(None)
    }

    /// Processes the block.
    ///
    /// # Arguments
    ///
    /// * `_` - Unused parameter (reserved for future use).
    ///
    /// # Returns
    ///
    /// Result containing an optional FinishCounter or an AinariError.
    fn process(&mut self, _: u64) -> Result<Option<Arc<Mutex<FinishCounter>>>, AinariError> {
        self.local_finish_counter = 0;

        Ok(None)
    }

    /// Performs backpropagation on the block.
    ///
    /// # Arguments
    ///
    /// * `_` - Unused parameter (reserved for future use).
    ///
    /// # Returns
    ///
    /// Result containing an optional FinishCounter or an AinariError.
    fn backpropagate(&mut self, _: u64) -> Result<Option<Arc<Mutex<FinishCounter>>>, AinariError> {
        Ok(Some(self.finish_counter_mutex.clone()))
    }

    /// Finalizes the training process.
    ///
    /// # Arguments
    ///
    /// * `cycle_number` - The current cycle number.
    ///
    /// # Returns
    ///
    /// Result indicating success or failure.
    fn finalize_train(&mut self, cycle_number: u64) -> Result<(), AinariError> {
        connect_outputs(
            &mut self.block_io,
            &self.model_uuid,
            &self.hexagon_uuid,
            &self.uuid,
        )?;
        send_forward(&self.block_io, WorkerTaskType::Train, cycle_number);

        Ok(())
    }

    /// Finalizes the processing.
    ///
    /// # Arguments
    ///
    /// * `cycle_number` - The current cycle number.
    ///
    /// # Returns
    ///
    /// Result indicating success or failure.
    fn finalize_process(&mut self, cycle_number: u64) -> Result<(), AinariError> {
        connect_outputs(
            &mut self.block_io,
            &self.model_uuid,
            &self.hexagon_uuid,
            &self.uuid,
        )?;
        send_forward(&self.block_io, WorkerTaskType::Process, cycle_number);

        Ok(())
    }

    /// Finalizes the backpropagation.
    ///
    /// # Arguments
    ///
    /// * `_` - Unused parameter (reserved for future use).
    ///
    /// # Returns
    ///
    /// Result indicating whether the finalization was successful.
    fn finalize_backpropagate(&mut self, _: u64) -> Result<bool, AinariError> {
        Ok(true)
    }

    /// Gets a free input axon section.
    ///
    /// # Arguments
    ///
    /// * `_` - Unused parameter (reserved for future use).
    ///
    /// # Returns
    ///
    /// `true` if a free input was found, `false` otherwise.
    fn get_free_input(&mut self, _: &mut AxonSection) -> bool {
        false
    }

    /// Gets the UUID of the block.
    ///
    /// # Returns
    ///
    /// The UUID of the block.
    fn get_uuid(&self) -> Uuid {
        self.uuid
    }

    /// Gets the hexagon UUID of the block.
    ///
    /// # Returns
    ///
    /// The hexagon UUID of the block.
    fn get_hexagon_uud(&self) -> Uuid {
        self.hexagon_uuid
    }

    /// Gets the model UUID of the block.
    ///
    /// # Returns
    ///
    /// The model UUID of the block.
    fn get_model_uud(&self) -> Uuid {
        self.model_uuid
    }

    /// Gets the block I/O buffer.
    ///
    /// # Returns
    ///
    /// A mutable reference to the block I/O buffer.
    fn get_block_io(&mut self) -> &mut BlockIoBuffer {
        &mut self.block_io
    }

    /// Gets the type of the block.
    ///
    /// # Returns
    ///
    /// The type of the block.
    fn get_type(&self) -> ObjectType {
        ObjectType::InputBlock
    }

    /// Sets the model UUID of the block.
    ///
    /// # Arguments
    ///
    /// * `new_model_uuid` - The new model UUID.
    fn set_model_uuid(&mut self, new_model_uuid: &Uuid) {
        self.model_uuid = *new_model_uuid;
    }

    /// Serializes the block to a byte vector.
    ///
    /// # Returns
    ///
    /// A byte vector containing the serialized block.
    fn serailize(&self) -> Vec<u8> {
        let cfg = bincode::config::standard();
        bincode::serde::encode_to_vec(self, cfg).expect("Failed to serialize")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_input() {
        let finish_counter = Arc::new(Mutex::new(FinishCounter::default()));

        let name = "test-input".to_string();
        let hexagon_uuid = Uuid::new_v4();
        let model_uuid = Uuid::new_v4();
        let mut input_block = InputBlock::new(&name, &hexagon_uuid, &model_uuid, &finish_counter);

        let input_values = vec![1.0, 2.0, -3.0, 4.0];
        input_block.apply_input(&input_values, input_values.len(), 2, 2, true);

        // check size of the resized buffers
        assert_eq!(input_block.input_links.len(), 16);
        assert_eq!(input_block.block_io.output_buffer.len(), 1);

        // check input-links
        assert_eq!(input_block.input_links[4], 0);
        assert_eq!(input_block.input_links[5], UNINIT_STATE_64);
        assert_eq!(input_block.input_links[6], 1);
        assert_eq!(input_block.input_links[7], UNINIT_STATE_64);
        assert_eq!(input_block.input_links[8], UNINIT_STATE_64);
        assert_eq!(input_block.input_links[9], 2);
        assert_eq!(input_block.input_links[10], 3);
        assert_eq!(input_block.input_links[11], UNINIT_STATE_64);

        // check axons
        assert_eq!(
            input_block.block_io.output_buffer[0].data.axons[0].potential,
            1.0
        );
        assert_eq!(
            input_block.block_io.output_buffer[0].data.axons[1].potential,
            2.0
        );
        assert_eq!(
            input_block.block_io.output_buffer[0].data.axons[2].potential,
            3.0
        );
        assert_eq!(
            input_block.block_io.output_buffer[0].data.axons[3].potential,
            4.0
        );
    }

    #[test]
    fn test_serialize_deserialize() {
        let original = InputBlock::default();

        let cfg = bincode::config::standard();
        let serialized: Vec<u8> =
            bincode::serde::encode_to_vec(&original, cfg).expect("Failed to serialize");
        let deserialized: InputBlock = bincode::serde::decode_from_slice(&serialized, cfg)
            .expect("Failed to deserialize")
            .0;
        println!("size: {}", serialized.len());

        assert_eq!(original, deserialized);
    }
}
