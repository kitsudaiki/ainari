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

use rand::RngExt;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use ainari_common::constants::*;
use ainari_common::enums::*;
use ainari_common::error::AinariError;
use ainari_common::functions::*;
use ainari_model_parser::model_meta_structs::Settings;

use crate::core::processing::finish_counter::FinishCounter;

use super::axons::*;
use super::block_io::*;
use super::block_trait::*;

use super::super::processing::worker_queue::*;

// ==================================================================================================

/// Represents a connection between neurons in the neural network.
///
/// This struct holds the synaptic weights, activation border, and target neuron information.
/// It's used to define how neurons are connected and how signals propagate through the network.
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub struct Synapse {
    /// The threshold value that must be exceeded for the synapse to activate.
    pub border: f32,
    /// synaptic weight values.
    pub weight_1: f32,
    pub weight_2: f32,

    /// Counter tracking how many times this synapse has been activated.
    pub active_counter: u8,
    /// ID of the neuron that this synapse connects to.
    pub target_neuron_id: u16,
}

impl Synapse {
    /// Creates a default Synapse instance with all values initialized to zero or uninitialized state.
    ///
    /// # Returns
    ///
    /// A new Synapse instance with default values.
    pub fn default() -> Self {
        Synapse {
            weight_1: 0.0f32,
            weight_2: 0.0f32,

            border: 1.0f32,
            active_counter: 0,
            target_neuron_id: UNINIT_STATE_16,
        }
    }
}

// ==================================================================================================

/// A collection of synapses organized in a section.
///
/// This struct is used to group synapses together for efficient processing and storage.
/// It contains an array of synapses with a fixed size defined by the BLOCK_DIM constant.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SynapseSection {
    /// Array of synapses in this section.
    ///
    /// The size of this array is determined by the BLOCK_DIM constant.
    #[serde(with = "BigArray")]
    pub synapses: [Synapse; BLOCK_DIM],
}

impl SynapseSection {
    /// Creates a default SynapseSection with random initial values for the first synapse.
    ///
    /// The first synapse in the section gets random target neuron ID and weight values,
    /// while the rest are initialized to default values.
    ///
    /// # Returns
    ///
    /// A new SynapseSection with randomized first synapse.
    pub fn default() -> Self {
        let mut section = SynapseSection {
            synapses: std::array::from_fn(|_| Synapse::default()),
        };

        // Initialize the first synapse with random values
        section.synapses[0].target_neuron_id = rand::rng().random_range(0..BLOCK_DIM) as u16;
        section.synapses[0].weight_1 = rand::rng().random_range(-0.5f32..0.5f32);
        section.synapses[0].weight_2 = rand::rng().random_range(-0.5f32..0.5f32);

        section
    }
}

// ==================================================================================================

/// Represents a connection between different parts of the neural network.
///
/// This struct is used to manage the flow of information between neurons and sections.
/// It tracks the lower bound of the connection, source input, next connection, and usage status.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct Connection {
    /// The lower bound value for this connection.
    pub lower_bound: f32,
    /// The source input ID for this connection.
    pub source_input: u16,
    /// The next connection ID.
    pub next: u16,
    /// Flag indicating whether this connection is currently in use.
    pub used: bool,
}

impl Connection {
    /// Creates a default Connection instance with all values initialized to zero or uninitialized state.
    ///
    /// # Returns
    ///
    /// A new Connection instance with default values.
    pub fn default() -> Self {
        Connection {
            lower_bound: 0.0f32,
            source_input: UNINIT_STATE_16,
            next: UNINIT_STATE_16,
            used: false,
        }
    }
}

// ==================================================================================================

/// Represents a neuron in the neural network.
///
/// This struct holds the current input value and refractory time of a neuron.
/// The refractory time prevents a neuron from firing immediately after it has fired.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Neuron {
    /// The current input value of the neuron.
    pub input: f32,
    /// The remaining refractory time of the neuron.
    pub refractory_time: u32,
}

impl Neuron {
    /// Creates a default Neuron instance with all values initialized to zero.
    ///
    /// # Returns
    ///
    /// A new Neuron instance with default values.
    pub fn default() -> Self {
        Neuron {
            input: 0.0f32,
            refractory_time: 0,
        }
    }
}

// ==================================================================================================

/// Represents a block of neurons and their connections in the neural network.
///
/// This is the core structural unit of the neural network, containing neurons, synapses,
/// and connections organized in a specific layout.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreBlock {
    /// Unique identifier for this block.
    pub uuid: Uuid,
    /// UUID of the hexagon this block belongs to.
    pub hexagon_uuid: Uuid,
    /// UUID of the model this block is part of.
    pub model_uuid: Uuid,

    /// Buffer for input/output operations of this block.
    pub block_io: BlockIoBuffer,
    /// Settings for the neural network model.
    model_settings: Settings,

    /// Collection of synapse sections in this block.
    ///
    /// Note: This is a Vec instead of a static array to prevent stack overflow
    /// due to the potentially large size of this object.
    pub synapse_sections: Vec<SynapseSection>,
    /// Array of neurons in this block.
    #[serde(with = "BigArray")]
    pub neurons: [Neuron; BLOCK_DIM * 3],
    /// Array of connections between different parts of this block.
    #[serde(with = "BigArray")]
    connections: [Connection; BLOCK_DIM * 6],

    /// Counter tracking the number of synapse sections used.
    pub section_counter: u64,
}

impl CoreBlock {
    /// Creates a new CoreBlock instance with the given parameters.
    ///
    /// This function initializes the block with default values and sets up the initial structure
    /// of neurons, synapses, and connections.
    ///
    /// # Arguments
    ///
    /// * `hexagon_uuid` - UUID of the hexagon this block belongs to
    /// * `model_uuid` - UUID of the model this block is part of
    /// * `model_settings` - Settings for the neural network model
    ///
    /// # Returns
    ///
    /// A new CoreBlock instance initialized with the given parameters.
    pub fn new(hexagon_uuid: &Uuid, model_uuid: &Uuid, model_settings: &Settings) -> Self {
        let mut block = CoreBlock {
            uuid: Uuid::new_v4(),
            hexagon_uuid: *hexagon_uuid,
            model_uuid: *model_uuid,

            block_io: BlockIoBuffer::default(),
            model_settings: model_settings.clone(),

            synapse_sections: Vec::new(),
            neurons: std::array::from_fn(|_| Neuron::default()),
            connections: std::array::from_fn(|_| Connection::default()),

            // pre-initialized number of synapses, one for each possible input-axon
            section_counter: 0,
        };

        // internal visilization of the blocks:
        //
        // +---+ +---+ +---+
        // | 1 | | 3 | | 5 |
        // +---+ +---+ +---+
        // | 0 | | 2 | | 4 |
        // +---+ +---+ +---+
        //
        let init_capacity = BLOCK_DIM * 2;
        block
            .synapse_sections
            .resize_with(init_capacity, SynapseSection::default);

        block.block_io.output_buffer.push(AxonSection::default());
        block.block_io.input_buffer.push(AxonSection::default());
        block.block_io.input_buffer.push(AxonSection::default());
        block.block_io.inputs_in_use = 0;

        for i in 0..(2 * BLOCK_DIM) {
            block.connections[i].lower_bound = 0.0f32;
            block.connections[i].source_input = i as u16;
        }

        block
    }

    /// Checks if the block needs to be resized and performs the resize if necessary.
    ///
    /// This function checks the current usage of the block and resizes it when it reaches
    /// 90% capacity. It increases both the number of synapse sections and output buffers.
    fn check_and_resize_block(&mut self) {
        // resize block in case it is filled enough
        if self.block_io.output_buffer.len() == 1
            && self.section_counter as f32 >= ((2 * BLOCK_DIM) as f32 * 0.9f32)
        {
            let new_capacity = BLOCK_DIM * 2 * 2;
            self.synapse_sections
                .resize_with(new_capacity, SynapseSection::default);
            self.block_io.output_buffer.push(AxonSection::default());
        }
        if self.block_io.output_buffer.len() == 2
            && self.section_counter as f32 >= ((4 * BLOCK_DIM) as f32 * 0.9f32)
        {
            let new_capacity = BLOCK_DIM * 2 * 3;
            self.synapse_sections
                .resize_with(new_capacity, SynapseSection::default);
            self.block_io.output_buffer.push(AxonSection::default());
        }
    }

    /// Applies the output values from the neurons to the axons.
    ///
    /// This function processes the output buffer, updating neuron states and axon potentials.
    /// It handles the refractory period of neurons and transfers the neuron input to axon potentials.
    fn apply_output(&mut self) {
        let mut counter = 0;
        let mut neuron;

        for buffer in self.block_io.output_buffer.iter_mut() {
            for axon in buffer.data.axons.iter_mut() {
                neuron = &mut self.neurons[counter];

                // Reduce axon potential based on neuron cooldown settings
                axon.potential /= self.model_settings.neuron_cooldown;
                // Reduce refractory time by half (right shift by 1)
                neuron.refractory_time >>= 1;

                if neuron.refractory_time == 0 {
                    // // experimental stuff
                    // axon.potential = neuron.input.signum() * (neuron.input.abs().log2() + 1.0f32);
                    // Set axon potential to neuron input when not refractory
                    axon.potential = neuron.input;
                    // Reset refractory time
                    neuron.refractory_time = self.model_settings.refractory_time;
                }

                // Reset neuron input and axon delta
                neuron.input = 0.0f32;
                axon.delta = 0.0f32;

                counter += 1;
            }
        }
    }
}

// ==================================================================================================

/// Creates a new synapse with random weights and initializes its properties.
///
/// # Arguments
///
/// * `synapse` - The synapse to initialize
/// * `remaining_weight` - The remaining weight to assign to the synapse's border
/// * `number_of_output_blocks` - The total number of output blocks in the network
/// * `random_seed` - A mutable reference to a random seed for generating random values
#[inline]
fn create_new_synapse(
    synapse: &mut Synapse,
    remaining_weight: f32,
    number_of_output_blocks: usize,
    random_seed: &mut u32,
) {
    let rand_max = RAND_MAX as f32;
    let sig_neg = 0.5f32;
    let mut sign_rand;

    // Generate random weight for weight_1 with potential sign flip
    synapse.weight_1 = ((pcg_hash(random_seed) as f32) / rand_max) / 10.0f32;
    sign_rand = (pcg_hash(random_seed) % 1000) as f32;
    synapse.weight_1 *= 1.0f32 - (1000.0f32 * sig_neg > sign_rand) as u8 as f32 * 2.0f32;

    // Generate random weight for weight_2 with potential sign flip
    synapse.weight_2 = ((pcg_hash(random_seed) as f32) / rand_max) / 10.0f32;
    sign_rand = (pcg_hash(random_seed) % 1000) as f32;
    synapse.weight_2 *= 1.0f32 - (1000.0f32 * sig_neg > sign_rand) as u8 as f32 * 2.0f32;

    // Initialize the synapse properties
    synapse.border = remaining_weight;
    synapse.active_counter = 50;
    synapse.target_neuron_id =
        (pcg_hash(random_seed) % (number_of_output_blocks * BLOCK_DIM) as u32) as u16
}

/// Searches for a free connection in the connections array.
///
/// # Arguments
///
/// * `connections` - A slice of connections to search through
///
/// # Returns
///
/// The index of the first free connection found, or UNINIT_STATE_16 as usize if none found
#[inline]
fn search_free_connection(connections: &[Connection; BLOCK_DIM * 6]) -> usize {
    for (i, conn) in connections.iter().enumerate() {
        if conn.source_input == UNINIT_STATE_16 {
            return i;
        }
    }

    UNINIT_STATE_16 as usize
}

/// Trains a synapse section using the given connection and neuron data.
///
/// # Arguments
///
/// * `section` - The synapse section to train
/// * `connection` - The connection containing input information
/// * `neurons` - A mutable slice of neurons to update
/// * `axon` - The axon containing the input potential
/// * `number_of_output_blocks` - The total number of output blocks in the network
/// * `random_seed` - A mutable reference to a random seed for generating random values
/// * `connections` - A mutable array of all connections in the network
///
/// # Returns
///
/// true if the section requires a next connection to be created, false otherwise
#[inline]
fn train_section(
    section: &mut SynapseSection,
    connection: &Connection,
    neurons: &mut [Neuron; BLOCK_DIM * 3],
    axon: &Axon,
    number_of_output_blocks: usize,
    random_seed: &mut u32,
    connections: &mut [Connection; BLOCK_DIM * 6],
) -> bool {
    let mut ratio;
    let mut potential = axon.potential - connection.lower_bound;
    let mut condition;
    let mut prev_border = 0.0f32;
    let mut target_neuron;

    // iterate over all synapses in the section
    for (pos, synapse) in section.synapses.iter_mut().enumerate() {
        if potential <= POTENTIAL_BORDER {
            break;
        }

        // create new synapse if necessary and training is active
        if synapse.target_neuron_id == UNINIT_STATE_16 {
            // because of the initialize of the section, the first position should
            // always be filled
            assert!(pos > 0);
            let remaining_weight = prev_border * 2.0f32;
            create_new_synapse(
                synapse,
                remaining_weight,
                number_of_output_blocks,
                random_seed,
            );
        }

        if potential < synapse.border {
            condition = potential < (1.0f32 - RELATIVE_CREATE_BORDER) * synapse.border
                && potential > RELATIVE_CREATE_BORDER * synapse.border
                && potential < synapse.border - ABSOLUTE_CREATE_BORDER
                && potential > ABSOLUTE_CREATE_BORDER;

            // Adjust the synapse border based on the condition
            synapse.border = synapse.border * (!condition) as u8 as f32
                + (synapse.border / 2.0f32) * (condition) as u8 as f32;
        }

        prev_border = synapse.border;

        // Calculate the ratio of potential to border
        ratio = 1.0f32;
        if potential < synapse.border {
            ratio = (1.0f32 / synapse.border) * potential;
        }

        // Update the input of the target neuron
        target_neuron = &mut neurons
            [(synapse.target_neuron_id % (number_of_output_blocks * BLOCK_DIM) as u16) as usize];
        target_neuron.input += synapse.weight_1 * ratio * (potential > synapse.border) as u8 as f32;

        // Update the input of the next target neuron
        target_neuron = &mut neurons[((synapse.target_neuron_id + 1)
            % (number_of_output_blocks * BLOCK_DIM) as u16)
            as usize];
        target_neuron.input += synapse.weight_2 * ratio * (potential > synapse.border) as u8 as f32;

        potential -= synapse.border;
    }

    if potential > POTENTIAL_BORDER {
        if connection.next == UNINIT_STATE_16 {
            return true;
        }

        if connection.next != UNINIT_STATE_16 {
            let used_potential = (axon.potential - connection.lower_bound) - potential;
            let next_connection = &mut connections[connection.next as usize];
            if next_connection.lower_bound < used_potential {
                next_connection.lower_bound = used_potential;
            }
        }
    }

    false
}

/// Processes a synapse section using the given connection and neuron data.
///
/// # Arguments
///
/// * `section` - The synapse section to process
/// * `connection` - The connection containing input information
/// * `neurons` - A mutable slice of neurons to update
/// * `axon` - The axon containing the input potential
/// * `number_of_output_blocks` - The total number of output blocks in the network
#[inline]
fn process_section(
    section: &mut SynapseSection,
    connection: &Connection,
    neurons: &mut [Neuron; BLOCK_DIM * 3],
    axon: &Axon,
    number_of_output_blocks: usize,
) {
    let mut ratio;
    let mut potential = axon.potential - connection.lower_bound;
    let mut target_neuron;

    // iterate over all synapses in the section
    for synapse in section.synapses.iter_mut() {
        if potential <= POTENTIAL_BORDER {
            break;
        }

        if synapse.target_neuron_id == UNINIT_STATE_16 {
            break;
        }

        // Calculate the ratio of potential to border
        ratio = 1.0f32;
        if potential < synapse.border {
            ratio = (1.0f32 / synapse.border) * potential;
        }

        // Update the input of the target neuron
        target_neuron = &mut neurons
            [(synapse.target_neuron_id % (number_of_output_blocks * BLOCK_DIM) as u16) as usize];
        target_neuron.input += synapse.weight_1 * ratio * (potential > synapse.border) as u8 as f32;

        // Update the input of the next target neuron
        target_neuron = &mut neurons[((synapse.target_neuron_id + 1)
            % (number_of_output_blocks * BLOCK_DIM) as u16)
            as usize];
        target_neuron.input += synapse.weight_2 * ratio * (potential > synapse.border) as u8 as f32;

        potential -= synapse.border;
    }
}

/// Backpropagates errors through a synapse section.
///
/// # Arguments
///
/// * `section` - The synapse section to backpropagate through
/// * `connection` - The connection containing input information
/// * `source_axon` - The source axon whose delta needs to be updated
/// * `output_buffer` - The buffer containing output axons
#[inline]
fn backpropagate_section(
    section: &mut SynapseSection,
    connection: &mut Connection,
    source_axon: &mut Axon,
    output_buffer: &[AxonSection],
) {
    let mut potential = source_axon.potential - connection.lower_bound;
    let mut delta;
    let mut target_axon;
    let mut output_block_id;
    let mut output_axon_id;

    // iterate over all synapses in the section
    for synapse in section.synapses.iter_mut() {
        if potential <= POTENTIAL_BORDER {
            break;
        }

        if synapse.target_neuron_id == UNINIT_STATE_16 {
            break;
        }

        // Calculate the output block and axon IDs for the target neuron
        output_block_id =
            ((synapse.target_neuron_id / BLOCK_DIM as u16) as usize) % output_buffer.len();
        output_axon_id = (synapse.target_neuron_id % BLOCK_DIM as u16) as usize;

        // Update the first weight and propagate the delta
        target_axon = &output_buffer[output_block_id].data.axons[output_axon_id];
        delta = target_axon.delta * synapse.weight_1;
        synapse.weight_1 -= CORE_TRAIN_VALUE * target_axon.delta;
        source_axon.delta += delta;

        // Calculate the output block and axon IDs for the next target neuron
        output_block_id =
            (((synapse.target_neuron_id + 1) / BLOCK_DIM as u16) as usize) % output_buffer.len();
        output_axon_id = ((synapse.target_neuron_id + 1) % BLOCK_DIM as u16) as usize;

        // Update the second weight and propagate the delta
        target_axon = &output_buffer[output_block_id].data.axons[output_axon_id];
        delta = target_axon.delta * synapse.weight_2;
        synapse.weight_2 -= CORE_TRAIN_VALUE * target_axon.delta;
        source_axon.delta += delta;

        potential -= synapse.border;
    }
}

// ==================================================================================================

/// Implementation of the Block trait for CoreBlock.
impl Block for CoreBlock {
    /// Trains the neural network block.
    ///
    /// # Arguments
    ///
    /// * `_` - Unused parameter (block index)
    /// * `_` - Unused parameter (previous block)
    /// * `_` - Unused parameter (cycle number)
    ///
    /// # Returns
    ///
    /// Result containing an optional finish counter or an AinariError
    fn train(
        &mut self,
        _: usize,
        _: Arc<Mutex<dyn Block>>,
        _: u64,
    ) -> Result<Option<Arc<Mutex<FinishCounter>>>, AinariError> {
        self.check_and_resize_block();
        let number_of_output_blocks = self.block_io.output_buffer.len();
        let mut random_seed = rand::rng().random_range(1..(RAND_MAX - 1)) as u32;

        // HINT (kitsudaki): used a normal for-loop instead of an iterator over the array here, to
        //                   avoid problems with the borrow-checker
        for i in 0..self.synapse_sections.len() {
            let conn = self.connections[i];
            if conn.source_input == UNINIT_STATE_16 {
                continue;
            }

            let input_block_id = (conn.source_input / BLOCK_DIM as u16) as usize;
            let axon_id = (conn.source_input % BLOCK_DIM as u16) as usize;
            let axon = &self.block_io.input_buffer[input_block_id].data.axons[axon_id];
            if axon.potential != 0.0f32 {
                if !conn.used {
                    self.section_counter += 1;
                    let temp_conn = &mut self.connections[i];
                    temp_conn.used = true;
                }
                let section = &mut self.synapse_sections[i];
                let need_next = train_section(
                    section,
                    &conn,
                    &mut self.neurons,
                    axon,
                    number_of_output_blocks,
                    &mut random_seed,
                    &mut self.connections,
                );
                if need_next {
                    let next = search_free_connection(&self.connections) as u16;
                    if next != UNINIT_STATE_16 {
                        let mut temp_conn = &mut self.connections[i];
                        temp_conn.next = next;
                        temp_conn = &mut self.connections[next as usize];
                        temp_conn.source_input = conn.source_input;
                    }
                }
            }
        }

        for axon_block in self.block_io.output_buffer.iter_mut() {
            for axon in axon_block.data.axons.iter_mut() {
                axon.delta = 0.0f32;
            }
        }

        self.apply_output();

        Ok(None)
    }

    /// Processes the neural network block.
    ///
    /// # Arguments
    ///
    /// * `_` - Unused parameter (cycle number)
    ///
    /// # Returns
    ///
    /// Result containing an optional finish counter or an AinariError
    fn process(&mut self, _: u64) -> Result<Option<Arc<Mutex<FinishCounter>>>, AinariError> {
        self.check_and_resize_block();
        let number_of_output_blocks = self.block_io.output_buffer.len();

        for (i, conn) in self.connections.iter().enumerate() {
            if i >= self.synapse_sections.len() {
                break;
            }
            if conn.source_input == UNINIT_STATE_16 {
                continue;
            }

            let input_block_id = (conn.source_input / BLOCK_DIM as u16) as usize;
            let axon_id = (conn.source_input % BLOCK_DIM as u16) as usize;
            let axon = &self.block_io.input_buffer[input_block_id].data.axons[axon_id];
            if axon.potential != 0.0f32 {
                if !conn.used {
                    continue;
                }
                let section = &mut self.synapse_sections[i];
                process_section(
                    section,
                    conn,
                    &mut self.neurons,
                    axon,
                    number_of_output_blocks,
                );
            }
        }

        self.apply_output();

        Ok(None)
    }

    /// Backpropagates errors through the neural network block.
    ///
    /// # Arguments
    ///
    /// * `_` - Unused parameter (cycle number)
    ///
    /// # Returns
    ///
    /// Result containing an optional finish counter or an AinariError
    fn backpropagate(&mut self, _: u64) -> Result<Option<Arc<Mutex<FinishCounter>>>, AinariError> {
        // // experimental stuff
        // for axon_section in self.block_io.input_buffer.iter_mut() {
        //     for axon in axon_section.axons.iter_mut() {
        //         axon.delta *= 1.4427f32 * (0.5f32).powf(axon.potential);
        //     }
        // }
        for (i, conn) in self.connections.iter_mut().enumerate() {
            if i >= self.synapse_sections.len() {
                break;
            }
            if conn.source_input == UNINIT_STATE_16 {
                continue;
            }

            let input_block_id = (conn.source_input / BLOCK_DIM as u16) as usize;
            let axon_id = (conn.source_input % BLOCK_DIM as u16) as usize;
            let source_axon = &mut self.block_io.input_buffer[input_block_id].data.axons[axon_id];
            if source_axon.potential > 0.0f32 {
                let section = &mut self.synapse_sections[i];
                backpropagate_section(section, conn, source_axon, &self.block_io.output_buffer);
            }
        }

        Ok(None)
    }

    /// Finalizes the training phase of the neural network block.
    ///
    /// # Arguments
    ///
    /// * `cycle_number` - The current cycle number
    ///
    /// # Returns
    ///
    /// Result indicating success or an AinariError
    fn finalize_train(&mut self, cycle_number: u64) -> Result<(), AinariError> {
        send_forward(
            &mut self.block_io,
            WorkerTaskType::Train,
            cycle_number,
            &self.model_uuid,
            &self.hexagon_uuid,
            &self.uuid,
        );

        Ok(())
    }

    /// Finalizes the processing of this block for a given cycle.
    ///
    /// This function connects outputs and sends a forward processing task to the worker.
    /// It is part of the neural network processing pipeline.
    ///
    /// # Arguments
    ///
    /// * `cycle_number` - The current cycle number in the processing sequence
    ///
    /// # Returns
    ///
    /// * `Ok(())` on success
    /// * `Err(AinariError)` if any operation fails
    fn finalize_process(&mut self, cycle_number: u64) -> Result<(), AinariError> {
        send_forward(
            &mut self.block_io,
            WorkerTaskType::Process,
            cycle_number,
            &self.model_uuid,
            &self.hexagon_uuid,
            &self.uuid,
        );

        Ok(())
    }

    /// Finalizes the backpropagation process for a given cycle.
    ///
    /// This function sends a backward propagation task with retry mechanism.
    ///
    /// # Arguments
    ///
    /// * `cycle_number` - The current cycle number in the processing sequence
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` indicating success of the operation
    /// * `Err(AinariError)` if any operation fails
    fn finalize_backpropagate(&mut self, cycle_number: u64) -> Result<bool, AinariError> {
        // Send backward propagation task with automatic retry
        let ret = send_backward_with_retry(&mut self.block_io, cycle_number);

        Ok(ret)
    }

    /// Gets a free input slot in the block's input buffer.
    ///
    /// This function allocates an available input slot for an axon section.
    /// It manages the input buffer and tracks used slots.
    ///
    /// # Arguments
    ///
    /// * `axon_section` - The axon section to be assigned to a free input slot
    ///
    /// # Returns
    ///
    /// * `true` if an input slot was successfully allocated
    /// * `false` if no input slots are available
    fn get_free_input(&mut self, axon_section: &mut AxonSection) -> bool {
        // Check and use the first input slot if available
        if self.block_io.inputs_in_use == 0 {
            axon_section.target_block_uuid = self.uuid;
            axon_section.target_hexagon_uuid = self.hexagon_uuid;
            axon_section.target_pos = 0;
            self.block_io.input_buffer[0] = axon_section.clone();
            self.block_io.inputs_in_use = 1;
            return true;
        }

        // Check and use the second input slot if available
        if self.block_io.inputs_in_use == 1 {
            axon_section.target_block_uuid = self.uuid;
            axon_section.target_hexagon_uuid = self.hexagon_uuid;
            axon_section.target_pos = 1;
            self.block_io.input_buffer[1] = axon_section.clone();
            self.block_io.inputs_in_use = 2;
            return true;
        }

        // No input slots available
        false
    }

    /// Gets the unique identifier of this block.
    ///
    /// # Returns
    ///
    /// The UUID of this block
    fn get_uuid(&self) -> Uuid {
        self.uuid
    }

    /// Gets the hexagon UUID this block belongs to.
    ///
    /// # Returns
    ///
    /// The UUID of the hexagon containing this block
    fn get_hexagon_uud(&self) -> Uuid {
        self.hexagon_uuid
    }

    /// Gets the model UUID this block belongs to.
    ///
    /// # Returns
    ///
    /// The UUID of the model containing this block
    fn get_model_uud(&self) -> Uuid {
        self.model_uuid
    }

    /// Gets a mutable reference to the block's I/O buffer.
    ///
    /// # Returns
    ///
    /// A mutable reference to the BlockIoBuffer
    fn get_block_io(&mut self) -> &mut BlockIoBuffer {
        &mut self.block_io
    }

    /// Gets the type of this object.
    ///
    /// # Returns
    ///
    /// The ObjectType of this block (always CoreBlock)
    fn get_type(&self) -> ObjectType {
        ObjectType::CoreBlock
    }

    /// Sets a new model UUID for this block.
    ///
    /// # Arguments
    ///
    /// * `new_model_uuid` - The new UUID of the model this block belongs to
    fn set_model_uuid(&mut self, new_model_uuid: &Uuid) {
        self.model_uuid = *new_model_uuid;
    }

    /// Serializes this block to a byte vector.
    ///
    /// Uses bincode for serialization with standard configuration.
    ///
    /// # Returns
    ///
    /// A byte vector containing the serialized block
    ///
    /// # Panics
    ///
    /// Panics if serialization fails
    fn serailize(&self) -> Vec<u8> {
        let cfg = bincode::config::standard();
        bincode::serde::encode_to_vec(self, cfg).expect("Failed to serialize")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize() {
        let settings = Settings::default();
        let original = CoreBlock::new(&Uuid::new_v4(), &Uuid::new_v4(), &settings);

        let cfg = bincode::config::standard().with_variable_int_encoding();
        let serialized: Vec<u8> =
            bincode::serde::encode_to_vec(&original, cfg).expect("Failed to serialize");
        let deserialized: CoreBlock = bincode::serde::decode_from_slice(&serialized, cfg)
            .expect("Failed to deserialize")
            .0;

        println!("size: {}", serialized.len());

        assert_eq!(original, deserialized);
    }
}
