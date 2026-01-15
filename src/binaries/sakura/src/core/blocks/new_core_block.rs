// // Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>

// // Licensed under the Apache License, Version 2.0 (the "License");
// // you may not use this file except in compliance with the License.
// // You may obtain a copy of the License at

// //     http://www.apache.org/licenses/LICENSE-2.0

// // Unless required by applicable law or agreed to in writing, software
// // distributed under the License is distributed on an "AS IS" BASIS,
// // WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// // See the License for the specific language governing permissions and
// // limitations under the License.

// use rand::Rng;
// use std::sync::{Arc, Mutex};
// use uuid::Uuid;

// use ainari_common::constants::*;
// use ainari_common::enums::*;
// use ainari_common::error::AinariError;

// use crate::core::processing::finish_counter::FinishCounter;

// use super::axons::*;
// use super::block_io::*;
// use super::block_trait::*;

// use super::super::processing::worker_queue::*;

// // ==================================================================================================

// #[derive(Debug, Clone, Copy)]
// pub struct Synapse {
//     pub weight: f32,
//     pub upper: f32,
//     pub lower: f32,
//     pub layer: u8,

//     pub active_counter: u8,
//     pub source_axon: u8,

//     pub active_counter_top: u8,
//     pub active_counter_lower: u8,
//     pub active_counter_upper: u8,
// }

// impl Synapse {
//     pub fn default() -> Self {
//         Synapse {
//             weight: rand::rng().random_range(-0.5..=0.5),

//             active_counter: 0,
//             source_axon: UNINIT_STATE_8,

//             upper: 2.0f32,
//             lower: 0.0f32,
//             layer: 0,

//             active_counter_top: 0,
//             active_counter_lower: 0,
//             active_counter_upper: 0,
//         }
//     }
// }

// // ==================================================================================================

// #[derive(Debug, Clone, Copy)]
// pub struct Neuron {
//     pub input: f32,
//     // pub refraction_time: u8,
// }

// impl Neuron {
//     #[allow(dead_code)]
//     pub fn default() -> Self {
//         Neuron {
//             input: 0.0f32,
//             // refraction_time: 0,
//         }
//     }
// }

// // ==================================================================================================

// #[derive(Debug, Clone)]
// pub struct NewCoreBlock {
//     pub uuid: Uuid,
//     pub hexagon_uuid: Uuid,
//     pub model_uuid: Uuid,

//     // HINT (kitsudaiki): this has to be a Box instead of a static array to avoid a stack-overflow, because the object is too big
//     pub synapses: Box<[Synapse]>,
//     pub buffer: [Synapse; BLOCK_DIM * 3],
//     pub neurons: [Neuron; BLOCK_DIM * 3],
//     pub fill_size: [u8; BLOCK_DIM * 3],

//     pub synapse_counter: u64,

//     pub block_io: BlockIoBuffer,
// }

// impl NewCoreBlock {
//     #[allow(dead_code)]
//     pub fn new(hexagon_uuid: &Uuid, model_uuid: &Uuid) -> Self {
//         // internal visilization of the blocks:
//         //
//         // +---+ +---+ +---+
//         // | 1 | | 3 | | 5 |
//         // +---+ +---+ +---+
//         // | 0 | | 2 | | 4 |
//         // +---+ +---+ +---+
//         //
//         let capacity = BLOCK_DIM * 2 * BLOCK_DIM * 3;
//         let mut vec = Vec::with_capacity(capacity);
//         for _ in 0..capacity {
//             vec.push(Synapse::default());
//         }

//         // set initial state
//         let mut counter = 0_usize;
//         let mut fill_size = std::array::from_fn(|_| 0u8);
//         for (x_offset, fill_size) in fill_size.iter_mut().enumerate().take(BLOCK_DIM) {
//             *fill_size = 2;
//             for y_offset in 0..(*fill_size as usize) {
//                 let synapse_pos = (x_offset * 2 * BLOCK_DIM) + y_offset;
//                 let synapse = &mut vec[synapse_pos];
//                 synapse.source_axon = counter as u8;
//                 synapse.lower = 0.0f32;
//                 synapse.upper = 2.0f32;
//                 synapse.active_counter = 255;
//                 counter += 1;
//             }
//         }

//         let mut block = NewCoreBlock {
//             uuid: Uuid::new_v4(),
//             hexagon_uuid: *hexagon_uuid,
//             model_uuid: *model_uuid,

//             block_io: BlockIoBuffer::default(),

//             synapses: vec.into_boxed_slice(),
//             buffer: std::array::from_fn(|_| Synapse::default()),
//             neurons: std::array::from_fn(|_| Neuron::default()),
//             fill_size,

//             // pre-initialized number of synapses, one for each possible input-axon
//             synapse_counter: 256,
//         };

//         block.block_io.output_buffer.push(AxonSection::default());
//         block.block_io.input_buffer.push(AxonSection::default());
//         block.block_io.input_buffer.push(AxonSection::default());
//         block.block_io.inputs_in_use = 0;

//         block
//     }

//     fn handle_buffer(&mut self) {
//         for x_offset in 0..(self.block_io.output_buffer.len() * BLOCK_DIM) {
//             if self.buffer[x_offset].source_axon != UNINIT_STATE_8 {
//                 let current_fill_size = self.fill_size[x_offset];

//                 if current_fill_size < ((2 * BLOCK_DIM) - 2) as u8 {
//                     let synapse_offset = (x_offset * 2 * BLOCK_DIM) + current_fill_size as usize;
//                     self.synapses[synapse_offset] = self.buffer[x_offset];
//                     self.buffer[x_offset] = Synapse::default();

//                     self.fill_size[x_offset] += 1;
//                     self.synapse_counter += 1;
//                 }
//             }
//         }
//     }

//     fn check_and_resize_block(&mut self) {
//         // resize block in case it is filled enough
//         if self.block_io.output_buffer.len() == 1
//             && self.synapse_counter as f32 >= ((BLOCK_DIM * BLOCK_DIM) as f32 * 0.9f32)
//         {
//             self.block_io.output_buffer.push(AxonSection::default());
//         }
//         if self.block_io.output_buffer.len() == 2
//             && self.synapse_counter as f32 >= ((BLOCK_DIM * BLOCK_DIM * 2) as f32 * 0.9f32)
//         {
//             self.block_io.output_buffer.push(AxonSection::default());
//         }
//     }

//     fn apply_output(&mut self) {
//         let mut counter = 0;
//         for o in 0..self.block_io.output_buffer.len() {
//             for i in 0..BLOCK_DIM {
//                 self.block_io.output_buffer[o].axons[i].potential = self.neurons[counter].input;
//                 counter += 1;
//             }
//         }
//     }
// }

// // ==================================================================================================

// impl Block for NewCoreBlock {
//     fn train(
//         &mut self,
//         place_offset: usize,
//         _: Arc<Mutex<dyn Block>>,
//         cycle_number: u64,
//     ) -> Result<Option<Arc<Mutex<FinishCounter>>>, AinariError> {
//         self.handle_buffer();
//         self.check_and_resize_block();

//         // // debug-output
//         // println!("core-buffer-axons: ");
//         // for axon in self.input_buffer[0].axons.iter_mut() {
//         //     print!("{} ", axon.potential);
//         // }
//         // println!(" ");

//         let number_of_output_blocks = self.block_io.output_buffer.len();
//         for x_offset in 0..(number_of_output_blocks * BLOCK_DIM) {
//             let neuron = &mut self.neurons[x_offset];
//             neuron.input = 0.0f32;

//             for y_offset in 0..(self.fill_size[x_offset] as usize) {
//                 // get values
//                 let synapse_pos = (x_offset * 2 * BLOCK_DIM) + y_offset;
//                 let synapse = &mut self.synapses[synapse_pos];
//                 let source_potential = self.block_io.input_buffer
//                     [synapse.source_axon as usize / BLOCK_DIM]
//                     .axons[synapse.source_axon as usize % BLOCK_DIM]
//                     .potential;
//                 if source_potential <= 0.0f32 {
//                     continue;
//                 }
//                 let third = (synapse.upper - synapse.lower) / 3.0f32;

//                 // update current synapse
//                 let update = (source_potential > synapse.lower) as u8;
//                 neuron.input += synapse.weight * update as f32;
//                 synapse.active_counter += update * (synapse.active_counter < 254) as u8;

//                 if synapse.layer < 6 {
//                     // handling first layer split
//                     if synapse.layer == 0
//                         && synapse.active_counter_top == 0
//                         && source_potential > synapse.upper
//                     {
//                         let mut new_synapse = Synapse::default();
//                         new_synapse.source_axon = synapse.source_axon;
//                         new_synapse.upper =
//                             ((synapse.upper - new_synapse.lower) * 2.0f32) + synapse.upper;
//                         new_synapse.lower = synapse.upper;
//                         new_synapse.layer = 0;
//                         let pos = (place_offset + x_offset) % (number_of_output_blocks * BLOCK_DIM);
//                         if self.buffer[pos].source_axon == UNINIT_STATE_8 {
//                             self.buffer[pos] = new_synapse;
//                             synapse.active_counter_top = 1;
//                         }
//                     }

//                     // handle upper split
//                     if synapse.active_counter_upper == 0
//                         && source_potential <= synapse.upper
//                         && source_potential > synapse.upper - third
//                     {
//                         let mut new_synapse = Synapse::default();
//                         new_synapse.source_axon = synapse.source_axon;
//                         new_synapse.upper = synapse.upper;
//                         new_synapse.lower = synapse.upper - third;
//                         new_synapse.layer = synapse.layer + 1;
//                         let pos = (place_offset + x_offset) % (number_of_output_blocks * BLOCK_DIM);
//                         if self.buffer[pos].source_axon == UNINIT_STATE_8 {
//                             self.buffer[pos] = new_synapse;
//                             synapse.active_counter_upper = 1;
//                         }
//                     }

//                     // handle lower split
//                     if synapse.active_counter_lower == 0
//                         && source_potential > synapse.lower
//                         && source_potential <= synapse.lower + third
//                     {
//                         let mut new_synapse = Synapse::default();
//                         new_synapse.source_axon = synapse.source_axon;
//                         new_synapse.upper = synapse.lower + third;
//                         new_synapse.lower = synapse.lower;
//                         new_synapse.layer = synapse.layer + 1;
//                         let pos = (place_offset + x_offset) % (number_of_output_blocks * BLOCK_DIM);
//                         if self.buffer[pos].source_axon == UNINIT_STATE_8 {
//                             self.buffer[pos] = new_synapse;
//                             synapse.active_counter_lower = 1;
//                         }
//                     }
//                 }
//             }

//             // TODO: handle refraction-type and node-cooldown of neuron
//         }

//         self.apply_output();
//         connect_outputs(
//             &mut self.block_io,
//             &self.model_uuid,
//             &self.hexagon_uuid,
//             &self.uuid,
//         )?;
//         send_forward(&self.block_io, WorkerTaskType::Train, cycle_number);

//         Ok(None)
//     }

//     fn process(
//         &mut self,
//         cycle_number: u64,
//     ) -> Result<Option<Arc<Mutex<FinishCounter>>>, AinariError> {
//         // // debug-output
//         // println!("core-buffer-axons: ");
//         // for axon in self.input_buffer[0].axons.iter_mut() {
//         //     print!("{} ", axon.potential);
//         // }
//         // println!(" ");

//         let number_of_output_blocks = self.block_io.output_buffer.len();
//         for x_offset in 0..(number_of_output_blocks * BLOCK_DIM) {
//             let neuron = &mut self.neurons[x_offset];
//             neuron.input = 0.0f32;

//             for y_offset in 0..(self.fill_size[x_offset] as usize) {
//                 // get values
//                 let synapse_pos = (x_offset * 2 * BLOCK_DIM) + y_offset;
//                 let synapse = &mut self.synapses[synapse_pos];
//                 let source_weight = self.block_io.input_buffer
//                     [synapse.source_axon as usize / BLOCK_DIM]
//                     .axons[synapse.source_axon as usize % BLOCK_DIM]
//                     .potential;

//                 // update current synapse
//                 let update = (source_weight > synapse.lower) as u8;
//                 neuron.input += synapse.weight * update as f32;
//                 synapse.active_counter += update * (synapse.active_counter < 254) as u8;
//             }

//             // TODO: handle refraction-type and node-cooldown of neuron
//         }
//         self.apply_output();
//         send_forward(&self.block_io, WorkerTaskType::Process, cycle_number);

//         Ok(None)
//     }

//     fn backpropagate(
//         &mut self,
//         cycle_number: u64,
//     ) -> Result<Option<Arc<Mutex<FinishCounter>>>, AinariError> {
//         let train_value = 0.1f32;

//         let number_of_output_blocks = self.block_io.output_buffer.len();
//         for x_offset in 0..(number_of_output_blocks * BLOCK_DIM) {
//             let target_axon =
//                 &mut self.block_io.output_buffer[x_offset / BLOCK_DIM].axons[x_offset % BLOCK_DIM];
//             if target_axon.delta == 0.0f32 {
//                 continue;
//             }

//             for y_offset in 0..(self.fill_size[x_offset] as usize) {
//                 let synapse = &mut self.synapses[(x_offset * 2 * BLOCK_DIM) + y_offset];
//                 let source_axon = &mut self.block_io.input_buffer
//                     [synapse.source_axon as usize / BLOCK_DIM]
//                     .axons[synapse.source_axon as usize % BLOCK_DIM];
//                 let update = (source_axon.potential > synapse.lower) as u8;
//                 let delta = target_axon.delta * synapse.weight * update as f32;
//                 synapse.weight -= train_value * target_axon.delta;
//                 source_axon.delta += delta;
//             }

//             target_axon.delta = 0.0f32;
//         }

//         send_backward(&self.block_io, cycle_number);

//         Ok(None)
//     }

//     fn get_free_input(&mut self, axon_section: &mut AxonSection) -> bool {
//         if self.block_io.inputs_in_use == 0 {
//             axon_section.target_block_uuid = self.uuid;
//             axon_section.target_hexagon_uuid = self.hexagon_uuid;
//             axon_section.target_pos = 0;
//             self.block_io.input_buffer[0] = axon_section.clone();
//             self.block_io.inputs_in_use = 1;
//             return true;
//         }

//         if self.block_io.inputs_in_use == 1 {
//             axon_section.target_block_uuid = self.uuid;
//             axon_section.target_hexagon_uuid = self.hexagon_uuid;
//             axon_section.target_pos = 1;
//             self.block_io.input_buffer[1] = axon_section.clone();
//             self.block_io.inputs_in_use = 2;
//             return true;
//         }

//         false
//     }

//     fn get_uuid(&self) -> Uuid {
//         self.uuid
//     }

//     fn get_hexagon_uud(&self) -> Uuid {
//         self.hexagon_uuid
//     }
//     fn get_model_uud(&self) -> Uuid {
//         self.model_uuid
//     }

//     fn get_block_io(&mut self) -> &mut BlockIoBuffer {
//         &mut self.block_io
//     }

//     fn get_type(&self) -> ObjectType {
//         ObjectType::Unknown
//     }

//     fn set_model_uuid(&mut self, new_model_uuid: &Uuid) {
//         self.model_uuid = *new_model_uuid;
//     }

//     fn serailize(&self) -> Vec<u8> {
//         Vec::new()
//     }
// }

// #[cfg(test)]
// mod tests {
//     use uuid::Uuid;

//     use ainari_common::constants::*;

//     use super::*;

//     #[test]
//     fn test_process() {
//         let model_uuid = Uuid::new_v4();
//         let hexagon_uuid = Uuid::new_v4();
//         let mut test_block = NewCoreBlock::new(&hexagon_uuid, &model_uuid);
//         test_block.buffer[1].source_axon = 42;
//         let own: Arc<Mutex<dyn Block>> = Arc::new(Mutex::new(test_block.clone()));
//         let cycle_number = 0;

//         let mut test_axon = AxonSection::default();
//         test_axon.axons[0].potential = 10.0f32;

//         test_block.block_io.input_buffer[0] = test_axon;

//         let _ = test_block.train(42, Arc::clone(&own), cycle_number);

//         // check if buffer-processing works correctly
//         assert_eq!(test_block.fill_size[1], 3);
//         assert_eq!(test_block.buffer[1].source_axon, UNINIT_STATE_8);
//         assert_eq!(test_block.synapses[2 * BLOCK_DIM + 2].source_axon, 42);
//         assert_eq!(test_block.synapse_counter, 257);

//         // run processing in order to create new synapses
//         let _ = test_block.train(42, Arc::clone(&own), cycle_number);
//         assert_eq!(test_block.synapse_counter, 258);
//         let _ = test_block.train(42, Arc::clone(&own), cycle_number);
//         assert_eq!(test_block.synapse_counter, 259);
//         let _ = test_block.train(42, Arc::clone(&own), cycle_number);
//         assert_eq!(test_block.synapse_counter, 260);
//         let _ = test_block.train(42, Arc::clone(&own), cycle_number);
//         assert_eq!(test_block.synapse_counter, 261);
//         let _ = test_block.train(42, Arc::clone(&own), cycle_number);
//         assert_eq!(test_block.synapse_counter, 262);
//         let _ = test_block.train(42, Arc::clone(&own), cycle_number);
//         assert_eq!(test_block.synapse_counter, 263);
//         let _ = test_block.train(42, Arc::clone(&own), cycle_number);
//         assert_eq!(test_block.synapse_counter, 264);
//         let _ = test_block.train(42, Arc::clone(&own), cycle_number);
//         assert_eq!(test_block.synapse_counter, 265);
//         // after depth = 8, nothing more will be added
//         let _ = test_block.train(42, Arc::clone(&own), cycle_number);
//         assert_eq!(test_block.synapse_counter, 265);

//         // check fill-size
//         assert_eq!(test_block.fill_size[1], 3);
//         assert_eq!(test_block.fill_size[42], 3);
//         assert_eq!(test_block.fill_size[84], 3);
//         assert_eq!(test_block.fill_size[126], 3);
//         assert_eq!(test_block.fill_size[40], 3);
//         assert_eq!(test_block.fill_size[82], 3);
//         assert_eq!(test_block.fill_size[124], 3);
//         assert_eq!(test_block.fill_size[38], 3);
//         assert_eq!(test_block.fill_size[80], 3);
//         assert_eq!(test_block.fill_size[122], 2);

//         // check that affected neurons have input
//         assert_ne!(test_block.neurons[84].input, 0.0f32);
//         assert_ne!(test_block.neurons[126].input, 0.0f32);
//         assert_ne!(test_block.neurons[40].input, 0.0f32);
//         assert_ne!(test_block.neurons[82].input, 0.0f32);
//         assert_ne!(test_block.neurons[124].input, 0.0f32);
//         assert_ne!(test_block.neurons[38].input, 0.0f32);
//         assert_ne!(test_block.neurons[80].input, 0.0f32);
//         assert_ne!(
//             test_block.block_io.output_buffer[0].data.axons[84].potential,
//             0.0f32
//         );
//         assert_ne!(
//             test_block.block_io.output_buffer[0].data.axons[126].potential,
//             0.0f32
//         );
//         assert_ne!(
//             test_block.block_io.output_buffer[0].data.axons[40].potential,
//             0.0f32
//         );
//         assert_ne!(
//             test_block.block_io.output_buffer[0].data.axons[82].potential,
//             0.0f32
//         );
//         assert_ne!(
//             test_block.block_io.output_buffer[0].data.axons[124].potential,
//             0.0f32
//         );
//         assert_ne!(
//             test_block.block_io.output_buffer[0].data.axons[38].potential,
//             0.0f32
//         );
//         assert_ne!(
//             test_block.block_io.output_buffer[0].data.axons[80].potential,
//             0.0f32
//         );

//         // check random other ones, that they have not input
//         assert_eq!(test_block.neurons[1].input, 0.0f32);
//         assert_eq!(test_block.neurons[127].input, 0.0f32);
//         assert_eq!(test_block.neurons[10].input, 0.0f32);

//         // set test-deltas
//         test_block.block_io.output_buffer[0].data.axons[42].delta = 0.5f32;
//         test_block.block_io.output_buffer[0].data.axons[84].delta = 0.5f32;
//         test_block.block_io.output_buffer[0].data.axons[126].delta = 0.5f32;
//         test_block.block_io.output_buffer[0].data.axons[40].delta = 0.5f32;
//         test_block.block_io.output_buffer[0].data.axons[82].delta = 0.5f32;
//         test_block.block_io.output_buffer[0].data.axons[124].delta = 0.5f32;
//         test_block.block_io.output_buffer[0].data.axons[38].delta = 0.5f32;
//         test_block.block_io.output_buffer[0].data.axons[80].delta = 0.5f32;

//         let _ = test_block.backpropagate(cycle_number);

//         // check that delta of the source-axon was modified
//         assert_ne!(test_block.block_io.input_buffer[0].axons[0].delta, 0.0f32);
//         // println!("{}", test_block.input_buffer[0].axons[0].delta);

//         // check that all target-axons are reseted
//         assert_eq!(test_block.block_io.output_buffer[0].data.axons[42].delta, 0.0f32);
//         assert_eq!(test_block.block_io.output_buffer[0].data.axons[84].delta, 0.0f32);
//         assert_eq!(
//             test_block.block_io.output_buffer[0].data.axons[126].delta,
//             0.0f32
//         );
//         assert_eq!(test_block.block_io.output_buffer[0].data.axons[40].delta, 0.0f32);
//         assert_eq!(test_block.block_io.output_buffer[0].data.axons[82].delta, 0.0f32);
//         assert_eq!(
//             test_block.block_io.output_buffer[0].data.axons[124].delta,
//             0.0f32
//         );
//         assert_eq!(test_block.block_io.output_buffer[0].data.axons[38].delta, 0.0f32);
//         assert_eq!(test_block.block_io.output_buffer[0].data.axons[80].delta, 0.0f32);
//     }
// }
