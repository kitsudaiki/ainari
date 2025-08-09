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

use std::sync::{Arc, Mutex, RwLock};
use std::collections::HashMap;
use uuid::Uuid;
use rand::Rng;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::io::{self, Write, Read, Seek, BufWriter, BufReader};
use std::path::PathBuf;

use hanami_cluster_parser::cluster_parser::parse_cluster_template;
use hanami_cluster_parser::cluster_meta_structs::*;
use hanami_common::error::HanamiError;
use hanami_common::constants::*;
use hanami_common::enums::*;

use crate::core::blocks::axons::AxonSection;
use crate::core::blocks::input_block::*;
use crate::core::blocks::core_block::*;
use crate::core::blocks::output_block::*;
use crate::core::processing::output_buffer::OutputBuffer;

use super::cluster_interface::ClusterInterface;
use super::blocks::block_trait::Block;

lazy_static::lazy_static! {
    pub static ref CLUSTER_HANDLER: RwLock<ClusterDataHandler> = RwLock::new(init_cluster_data_handler());
}

// ==================================================================================================

#[derive(Default, Debug)]
pub struct FinishCounter {
    pub counter: usize,
    pub input_compare: usize, 
    pub output_compare: usize, 
}

// ==================================================================================================

pub struct HexagonData {
    pub blocks: HashMap<Uuid, Arc<Mutex<dyn Block>>>,
}

impl HexagonData {
    pub fn new() -> Self {
        HexagonData {
            blocks: HashMap::new(),
        }
    }
}

// ==================================================================================================

pub struct ClusterContent {
    pub cluster_meta: ClusterMeta,
    pub hexagon_data: RwLock<HashMap<Uuid, HexagonData>>,
    pub inputs: RwLock<HashMap<String, Arc<Mutex<InputBlock>>>>,
    pub outputs: RwLock<HashMap<String, Arc<Mutex<OutputBuffer>>>>,
    pub cluster_interface: Option<Arc<Mutex<ClusterInterface>>>,
}

impl ClusterContent {
    pub fn new(cluster_meta: ClusterMeta) -> Self {
        ClusterContent {
            cluster_meta: cluster_meta,
            hexagon_data: RwLock::new(HashMap::new()),
            inputs: RwLock::new(HashMap::new()),
            outputs: RwLock::new(HashMap::new()),
            cluster_interface: None,
        }
    }
}

// ==================================================================================================

pub struct ClusterDataHandler {
    pub clusters: HashMap<Uuid, ClusterContent>,
}

// ==================================================================================================

pub fn init_cluster_data_handler() -> ClusterDataHandler {
    let cluster_handler = ClusterDataHandler {
        clusters: HashMap::new(),
    };
    cluster_handler
}

// ==================================================================================================

impl ClusterDataHandler {
    
    /**
     * 
     */
    pub fn init_new_cluster(&mut self, cluster_uuid: &Uuid, name: &String, cluster_template: String) -> Result<(), HanamiError> {
        // parse cluster-template
        let mut parsed_cluster: ClusterMeta = match parse_cluster_template(name, cluster_template.as_str()) {
            Ok(parsed) => parsed,
            Err(e) => {
                let msg = format!("Can not create cluster: {:?}", e);
                return Err(HanamiError::InputError(msg));
            }
        };

        // get and init finish-counter
        let finish_counter_mutex = Arc::new(Mutex::new(FinishCounter::default()));
        let mut finish_counter = finish_counter_mutex.lock().unwrap();
        let interface = Arc::new(Mutex::new(ClusterInterface::new(&cluster_uuid, &finish_counter_mutex)));

        // add cluster to the cluster-handler
        parsed_cluster.uuid = cluster_uuid.clone();
        if self.register_cluster(&parsed_cluster, Some(interface)) == false {
            let msg = format!("Failed to add cluster with UUID '{cluster_uuid}' to cluster-handler");
            return Err(HanamiError::Error(msg));
        }

        // initialize input-blocks
        for input_meta in parsed_cluster.inputs.iter() {
            let input_block_mutex = Arc::new(Mutex::new(InputBlock::new(&input_meta.name, &input_meta.hexagon_uuid, &cluster_uuid, &finish_counter_mutex)));
            if self.add_input_block(&input_block_mutex) == false {
                let msg = format!("Failed to add input-block with name '{}' to cluster-handler", input_meta.name);
                return Err(HanamiError::Error(msg));
            }
        }
        finish_counter.input_compare = parsed_cluster.inputs.len();

        // initilize output-buffer
        for output_meta in parsed_cluster.outputs.iter() {
            let output_buffer_mutex = Arc::new(Mutex::new(OutputBuffer::new(&output_meta.name, &output_meta.hexagon_uuid, &cluster_uuid, &output_meta.output_type, &finish_counter_mutex)));
            if self.add_output_buffer(&output_buffer_mutex) == false {
                let msg = format!("Failed to add output-buffer with name '{}' to cluster-handler", output_meta.name);
                return Err(HanamiError::Error(msg));
            }
        }
        finish_counter.output_compare = parsed_cluster.outputs.len();

        Ok(())
    }

    /**
     * 
     */
    pub fn register_cluster(&mut self, cluster_meta: &ClusterMeta, interface: Option<Arc<Mutex<ClusterInterface>>>) -> bool {
        if self.clusters.contains_key(&cluster_meta.uuid) {
            return false;
        } 

        let cluster_uuid = cluster_meta.uuid.clone();
        let mut content = ClusterContent::new(cluster_meta.clone());
        content.cluster_interface = interface;

        if self.clusters.insert(cluster_uuid, content).is_none() == false {
            return false;
        }

        true
    }

    /**
     * 
     */
    pub fn add_core_block(&mut self, block_mutex: &Arc<Mutex<CoreBlock>>) -> bool {
        return self.add_block(&(block_mutex.clone() as Arc<Mutex<dyn Block>>));
    }

    /**
     * 
     */
    pub fn add_output_block(&mut self, block_mutex: &Arc<Mutex<OutputBlock>>) -> bool {
        return self.add_block(&(block_mutex.clone() as Arc<Mutex<dyn Block>>));
    }
    
    /**
     * 
     */
    fn add_input_block(&mut self, block_mutex: &Arc<Mutex<InputBlock>>) -> bool {
        let input_block = block_mutex.lock().unwrap();
        let cluster_uuid = input_block.get_cluster_uud();
        let hexagon_uuid = input_block.get_hexagon_uud();
        let block_name = input_block.name.clone();

        // get cluster
        let cluster_link = if let Some(c) = self.clusters.get_mut(&cluster_uuid)  {
            c
        } else {
            return false;
        };

        // get hexagon from cluster
        let mut hexagon_data_map = cluster_link.hexagon_data.write().unwrap();
        if hexagon_data_map.contains_key(&hexagon_uuid) == false {
            hexagon_data_map.insert(hexagon_uuid.clone(), HexagonData::new());
        }

        let hexgon_link = if let Some(h) = hexagon_data_map.get_mut(&hexagon_uuid) {
            h
        } else {
            return false;
        };

        // get hexagon-io
        let mut inputs = cluster_link.inputs.write().unwrap();
        if inputs.contains_key(&block_name) {
            return false;
        }

        // add new block
        let block_uuid = input_block.get_uuid();
        if hexgon_link.blocks.contains_key(&block_uuid) == false {
            hexgon_link.blocks.insert(block_uuid.clone(), Arc::clone(block_mutex) as Arc<Mutex<dyn Block>>);
            inputs.insert(block_name.clone(),Arc::clone(block_mutex));
            return true;
        } 

        false
    }

    /**
     * 
     */  
    fn add_output_buffer(&mut self, block_mutex: &Arc<Mutex<OutputBuffer >>) -> bool {
        let output_buffer = block_mutex.lock().unwrap();
        let cluster_uuid = output_buffer.cluster_uuid.clone();
        let name = output_buffer.name.clone();

        // get cluster
        let cluster_link = if let Some(c) = self.clusters.get_mut(&cluster_uuid)  {
            c
        } else {
            return false;
        };

        // get hexagon-io
        let mut outputs = cluster_link.outputs.write().unwrap();
        if outputs.contains_key(&name) {
            return false;
        }
        
        outputs.insert(name.clone(), Arc::clone(block_mutex));

        true
    }
      
    /**
     * 
     */
    fn add_block(&mut self, block_mutex: &Arc<Mutex<dyn Block>>) -> bool {
        let block = block_mutex.lock().unwrap();
        let cluster_uuid = block.get_cluster_uud();
        let hexagon_uuid = block.get_hexagon_uud();
        let block_uuid = block.get_uuid();

        // get cluster
        let cluster_link = if let Some(c) = self.clusters.get_mut(&cluster_uuid)  {
            c
        } else {
            return false;
        };

        // get hexagon from cluster
        let mut hexagon_data = cluster_link.hexagon_data.write().unwrap();
        if hexagon_data.contains_key(&hexagon_uuid) == false {
            hexagon_data.insert(hexagon_uuid.clone(), HexagonData::new());
        }

        let hexgon_link = if let Some(h) = hexagon_data.get_mut(&hexagon_uuid) {
            h
        } else {
            return false;
        };

        // add new block
        if hexgon_link.blocks.contains_key(&block_uuid) == false {
            hexgon_link.blocks.insert(block_uuid.clone(), Arc::clone(block_mutex));
            return true;
        } 

        false
    }

    /**
     * 
     */
    pub fn get_cluster_interface(&self, cluster_uuid: &Uuid) -> Option<Arc<Mutex<ClusterInterface>>> {
        let cluster_handle = if let Some(c) = self.clusters.get(&cluster_uuid) {
            c
        } else {
            return None;
        };

        if let Some(interface) = &cluster_handle.cluster_interface {
            return Some(Arc::clone(interface));
        } else {
            return None;
        }
    }

    /**
     * 
     */
    #[allow(dead_code)]
    pub fn get_finish_counter(&self, cluster_uuid: &Uuid) -> Option<Arc<Mutex<FinishCounter>>> {
        let cluster_handle = if let Some(c) = self.clusters.get(&cluster_uuid) {
            c
        } else {
            return None;
        };

        if let Some(cluster_interface_mutex) = &cluster_handle.cluster_interface {
            let cluster_interface = cluster_interface_mutex.lock().unwrap();
            return Some(Arc::clone(&cluster_interface.finish_counter));
        } else {
            return None;
        }
    }

    /**
     * 
     */
    pub fn get_block(&self, cluster_uuid: &Uuid, hexagon_uuid: &Uuid, block_uuid: &Uuid) -> Option<Arc<Mutex<dyn Block>>> {
        let cluster_link = if let Some(c) = self.clusters.get(cluster_uuid) {
            c
        } else {
            return None;
        };

        let binding = cluster_link.hexagon_data.read().unwrap();
        let hexagon_link = if let Some(h) = binding.get(&hexagon_uuid) {
            h
        } else {
            return None;
        };

        if let Some(block_link) = hexagon_link.blocks.get(block_uuid) {
            return Some(Arc::clone(block_link));
        }

        None
    }

    /**
     * 
     */
    pub fn get_input_block(&self, cluster_uuid: &Uuid, name: &String) -> Option<Arc<Mutex<InputBlock>>> {
        let cluster_link = if let Some(c) = self.clusters.get(cluster_uuid) {
            c
        } else {
            return None;
        };

        let binding = cluster_link.inputs.read().unwrap();
        if let Some(input_block_mutex) = binding.get(name) {
            return Some(input_block_mutex.clone());
        } else {
            return None;
        };
    }

    /**
     * 
     */
    pub fn get_output_buffer(&self, cluster_uuid: &Uuid, name: &String) -> Option<Arc<Mutex<OutputBuffer>>> {
        let cluster_link = if let Some(c) = self.clusters.get(cluster_uuid) {
            c
        } else {
            return None;
        };

        let binding = cluster_link.outputs.read().unwrap();
        if let Some(output_buffer_mutex) = binding.get(name) {
            return Some(output_buffer_mutex.clone());
        } else {
            return None;
        };
    }

    /**
     * 
     */
    pub fn delete_block(&mut self, cluster_uuid: &Uuid, hexagon_uuid: &Uuid, block_uuid: &Uuid) -> bool {
        // get cluster
        let cluster_link = if let Some(c) = self.clusters.get_mut(cluster_uuid) {
            c
        }
        else {
            return false;
        };

        // get hexagon from cluster
        let mut binding = cluster_link.hexagon_data.write().unwrap();
        let hexagon_link = if let Some(h) = binding.get_mut(&hexagon_uuid) {
            h
        }
        else {
            return false;
        };

        // delete block
        if hexagon_link.blocks.contains_key(block_uuid) == false {
            return false;
        } 
        hexagon_link.blocks.remove(block_uuid);
        
        // remove hexagon, if it doesn't contain any more blocks
        if hexagon_link.blocks.len() == 0 {
            cluster_link.hexagon_data.write().unwrap().remove(hexagon_uuid);
        }

        return true;
    }

    /**
     * 
     */
    pub fn delete_cluster(&mut self, cluster_uuid: &Uuid) -> bool {
        if self.clusters.contains_key(&cluster_uuid) == false {
            return false;
        }

        self.clusters.remove(cluster_uuid);

        true
    }

    /**
     * 
     */
    fn write_struct_to_file<T: Serialize>(
        &self, 
        writer: &mut BufWriter<fs::File>,
        struct_type: ObjectType,
        value: &T) -> Result<(), Box<dyn std::error::Error>> 
    {
        let cfg = bincode::config::standard();
        let data = bincode::serde::encode_to_vec(value, cfg)?;
        let len = data.len() as u32;

        writer.write_all(&[struct_type.to_u8()])?;
        writer.write_all(&len.to_le_bytes())?;
        writer.write_all(&data)?;

        Ok(())
    }

    /**
     * 
     */
    fn write_vec_to_file(
        &self, 
        writer: &mut BufWriter<fs::File>,
        struct_type: ObjectType,
        data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> 
    {
        let len = data.len() as u32;

        writer.write_all(&[struct_type.to_u8()])?;
        writer.write_all(&len.to_le_bytes())?;
        writer.write_all(&data)?;

        Ok(())
    }

    /**
     * 
     */
    pub fn create_checkpoint(&self, cluster_uuid: &Uuid, file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        // get cluster
        let cluster_link = if let Some(c) = self.clusters.get(cluster_uuid) {
            c
        }
        else {
            let msg = format!("Cluster with uuid '{cluster_uuid}' not found.");
            return Err(Box::new(HanamiError::Error(msg)));
        };

        let file_path_str: String = file_path.to_string_lossy().into();

        // check if file already exist
        if Path::new(file_path).exists() {
            let msg = format!("Checkpoint file '{file_path_str}' already exists.");
            // HINT (kitsudaki): the path is defined by the backend itself and not by the user, 
            // so here should be an internal error instand of an input-error
            return Err(Box::new(HanamiError::Error(msg)));
        }

        // initialize file
        let file = fs::File::create(file_path)?;
        let mut target_file = BufWriter::new(file);

        // write cluster-meta into checkpoint-file
        self.write_struct_to_file(&mut target_file, ObjectType::ClusterMeta, &cluster_link.cluster_meta)?;

        // write blocks into checkpoint-file
        let hexagon_data = cluster_link.hexagon_data.read().unwrap();
        for hexagon in hexagon_data.values() {
            for block_mutex in hexagon.blocks.values() {
                let block = block_mutex.lock().unwrap();
                self.write_vec_to_file(&mut target_file, block.get_type(), block.serailize())?;
            }
        }

        // write output-buffers into checkpoint-file
        let outputs = cluster_link.outputs.read().unwrap();
        for output_mutex in outputs.values() {
            let output = output_mutex.lock().unwrap();
            self.write_vec_to_file(&mut target_file, ObjectType::OutputBuffer, output.serailize())?;
        }

        Ok(())
    }

    /**
     * 
     */
    pub fn restore_checkpoint(&mut self, cluster_uuid: &Uuid, file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        // in cluster with this uuid already exist, it fill be removed first in order to create it new from the checkpoint
        let mut cluster_interface = None;
        if self.clusters.contains_key(&cluster_uuid) {
            if let Some(cluster_link) = self.clusters.get_mut(&cluster_uuid) {
                if let Some(interface) = &cluster_link.cluster_interface {
                    cluster_interface = Some(interface.clone());
                }
                cluster_link.cluster_interface = None;
            }
            self.clusters.remove(cluster_uuid);
        }

        // init file and other components for reading
        let file = fs::File::open(file_path)?;
        let mut reader = BufReader::with_capacity(10*1024*1024, file);
        let cfg = bincode::config::standard();
        let mut len_buf = [0u8; 4];

        // init counter
        let mut finish_counter_mutex = Arc::new(Mutex::new(FinishCounter::default()));
        let mut cluster_meta_parsed = false;
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

            let struct_type = ObjectType::from_u8(type_buf[0])
                .ok_or("Unknown struct type in stream")?;

            // read  (4 bytes)
            reader.read_exact(&mut len_buf)?;
            let len = u32::from_le_bytes(len_buf) as usize;

            // read data bytes
            let mut data_buf = vec![0u8; len];
            reader.read_exact(&mut data_buf)?;

            // deserialize based on type
            match struct_type {
                // Unknown
                ObjectType::Unknown => {
                    let msg = format!("Invalid object found in checkpoint-file.");
                    return Err(Box::new(HanamiError::InputError(msg)));
                }
                // ClusterMeta
                ObjectType::ClusterMeta => {
                    if cluster_meta_parsed {
                        let msg = format!("File has multiple cluster-meta objects.");
                        return Err(Box::new(HanamiError::InputError(msg)));
                    }
                    let mut cluster_meta: ClusterMeta = bincode::serde::decode_from_slice(&data_buf, cfg).expect("Failed to deserialize").0;
                    cluster_meta.uuid = cluster_uuid.clone();
                    if let Some(interface_mutex) = &cluster_interface {
                        {
                            let interface = interface_mutex.lock().unwrap();
                            finish_counter_mutex = Arc::clone(&interface.finish_counter);
                        }
                        if self.register_cluster(&cluster_meta, Some(interface_mutex.clone())) == false {
                            let msg = format!("Failed to add cluster with UUID '{cluster_uuid}' to cluster-handler");
                            return Err(Box::new(HanamiError::Error(msg)));
                        }
                    }
                    else {
                        let interface = Arc::new(Mutex::new(ClusterInterface::new(&cluster_uuid, &finish_counter_mutex)));
                        if self.register_cluster(&cluster_meta, Some(interface)) == false {
                            let msg = format!("Failed to add cluster with UUID '{cluster_uuid}' to cluster-handler");
                            return Err(Box::new(HanamiError::Error(msg)));
                        }
                    }
                    cluster_meta_parsed = true;
                }
                // HexagonData
                ObjectType::HexagonData => {
                    let msg = format!("Invalid object found in checkpoint-file.");
                    return Err(Box::new(HanamiError::InputError(msg)));
                }
                // InputBlock
                ObjectType::InputBlock => {
                    if cluster_meta_parsed == false {
                        let msg = format!("File has no cluster-meta object at the starting position.");
                        return Err(Box::new(HanamiError::InputError(msg)));
                    }
                    let mut input_block: InputBlock = bincode::serde::decode_from_slice(&data_buf, cfg).expect("Failed to deserialize").0;
                    input_block.set_cluster_uuid(cluster_uuid);
                    self.add_input_block(&Arc::new(Mutex::new(input_block)));
                    number_of_input_blocks += 1;
                }
                // CoreBlock
                ObjectType::CoreBlock => {
                    if cluster_meta_parsed == false {
                        let msg = format!("File has no cluster-meta object at the starting position.");
                        return Err(Box::new(HanamiError::InputError(msg)));
                    }
                    let mut core_block: CoreBlock = bincode::serde::decode_from_slice(&data_buf, cfg).expect("Failed to deserialize").0;
                    core_block.set_cluster_uuid(cluster_uuid);
                    self.add_core_block(&Arc::new(Mutex::new(core_block)));
                }
                // OutputBlock
                ObjectType::OutputBlock => {
                    if cluster_meta_parsed == false {
                        let msg = format!("File has no cluster-meta object at the starting position.");
                        return Err(Box::new(HanamiError::InputError(msg)));
                    }
                    let mut output_block: OutputBlock = bincode::serde::decode_from_slice(&data_buf, cfg).expect("Failed to deserialize").0;
                    output_block.set_cluster_uuid(cluster_uuid);
                    self.add_output_block(&Arc::new(Mutex::new(output_block)));
                }
                // OutputBuffer
                ObjectType::OutputBuffer => {
                    if cluster_meta_parsed == false {
                        let msg = format!("File has no cluster-meta object at the starting position.");
                        return Err(Box::new(HanamiError::InputError(msg)));
                    }
                    let mut output_buffer: OutputBuffer = bincode::serde::decode_from_slice(&data_buf, cfg).expect("Failed to deserialize").0;
                    output_buffer.cluster_uuid = cluster_uuid.clone();
                    self.add_output_buffer(&Arc::new(Mutex::new(output_buffer)));
                    number_of_output_buffer += 1;
                }
            };
        }

        // set initial values for the finish-counter
        let mut finish_counter = finish_counter_mutex.lock().unwrap();
        finish_counter.input_compare = number_of_input_blocks;
        finish_counter.output_compare = number_of_output_buffer;

        // get cluster
        let cluster_link = if let Some(c) = self.clusters.get(cluster_uuid) {
            c
        }
        else {
            let msg = format!("Cluster with uuid '{cluster_uuid}' not found after restore.");
            return Err(Box::new(HanamiError::Error(msg)));
        };
        
        // connect new finish-counter to inputs
        let inputs = cluster_link.inputs.read().unwrap();
        for input_mutex in inputs.values() {
            let mut input = input_mutex.lock().unwrap();
            input.finish_counter = Arc::clone(&finish_counter_mutex);
        }

        // connect new finish-counter to outputs
        let outputs = cluster_link.outputs.read().unwrap();
        for output_mutex in outputs.values() {
            let mut output = output_mutex.lock().unwrap();
            output.finish_counter = Arc::clone(&finish_counter_mutex);
        }

        Ok(())
    }

    /**
     * 
     */
    pub fn delete_all_cluster(&mut self) {
        self.clusters.clear();
    }

    /**
     * 
     */
    pub fn get_target(&mut self, axon_section: &mut AxonSection) -> bool {
        // pre-check
        if axon_section.cluster_uuid == Uuid::nil() 
            || axon_section.source_block_uuid == Uuid::nil() 
            || axon_section.source_hexagon_uuid == Uuid::nil() 
            || axon_section.source_pos == UNINIT_STATE_8 
        {
            return false;
        }

        let target_hexagon_uuid;
        let is_output_hexagon;
        let output_hexagon_name;


        // get source-block
        let source_block = if let Some(s) = self.get_block(&axon_section.cluster_uuid, &axon_section.source_hexagon_uuid, &axon_section.source_block_uuid) {
            s
        } else {
            return false;
        };

        {
            // get cluster
            let cluster_link = if let Some(c) = self.clusters.get_mut(&axon_section.cluster_uuid) {
                c
            }
            else {
                return false;
            };

            // get the uuid of the target-hexagon
            {
                let source_hexagon = if let Some(s_h) = cluster_link.cluster_meta.hexagons.get(&axon_section.source_hexagon_uuid) {
                    s_h
                } else {
                    return false;
                };

                let random_pos = rand::rng().random_range(0..NUMBER_OF_POSSIBLE_NEXT) as usize;
                target_hexagon_uuid = source_hexagon.possible_hexagon_target_ids[random_pos].clone();

                let target_hexagon = if let Some(t_h) = cluster_link.cluster_meta.hexagons.get(&target_hexagon_uuid) {
                    t_h
                } else {
                    return false;
                };
                is_output_hexagon = target_hexagon.is_output;
                output_hexagon_name = target_hexagon.name.clone();

                // input-hexagons are not allowed to be a target
                if target_hexagon.is_input {
                    return false;
                }
            }

            // get target-hexagon from cluster
            let mut binding = cluster_link.hexagon_data.write().unwrap();
            let target_hexagon_link = if let Some(h) = binding.get_mut(&target_hexagon_uuid) {
                h
            }
            else {
                binding.insert(target_hexagon_uuid.clone(), HexagonData::new());
                if let Some(h) = binding.get_mut(&target_hexagon_uuid) {
                    h
                }
                else {
                    return false
                }
            };

            // search for a block, which has a free slot
            for block_mutex in target_hexagon_link.blocks.values() {
                let mut block = block_mutex.lock().unwrap();
                if block.get_free_input(axon_section) != false {
                    axon_section.target_block = Some(block_mutex.clone());
                    axon_section.source_block = Some(source_block);
                    return true;
                }
            }
        }

        // create new block
        if is_output_hexagon {
            let output_block_mutex = Arc::new(Mutex::new(OutputBlock::new(&target_hexagon_uuid, &axon_section.cluster_uuid, &output_hexagon_name)));
            if self.add_output_block(&output_block_mutex) == false {
                return false;
            }
            let mut output_block = output_block_mutex.lock().unwrap();
            if output_block.get_free_input(axon_section) {
                axon_section.target_block = Some(output_block_mutex.clone());
                axon_section.source_block = Some(source_block);
                return true;
            }
        } else {
            let core_block_mutex = Arc::new(Mutex::new(CoreBlock::new(&target_hexagon_uuid, &axon_section.cluster_uuid)));
            if self.add_core_block(&core_block_mutex) == false {
                return false;
            }
            let mut core_block = core_block_mutex.lock().unwrap();
            if core_block.get_free_input(axon_section) {
                axon_section.target_block = Some(core_block_mutex.clone());
                axon_section.source_block = Some(source_block);
                return true;
            }
        }

        false
    }
}


#[cfg(test)]
mod tests {
    use hanami_common::enums::*;
    use serial_test::serial;

    use super::*;

    #[test]
    #[serial]
    fn test_create_cluster() {
        let cluster_uuid = Uuid::new_v4();
        let name = "test_cluster".to_string();
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
            key2: 2,2,2;".to_string();

        let mut root_handler = CLUSTER_HANDLER.write().unwrap();
        root_handler.delete_all_cluster();

        {
            let ret = root_handler.init_new_cluster(&cluster_uuid, &name, template);
            assert!(ret.is_ok());
            assert_eq!(root_handler.clusters.len(), 1);
            assert!(root_handler.clusters.contains_key(&cluster_uuid));

            let cluster = root_handler.clusters.get(&cluster_uuid).unwrap();
            assert!(cluster.cluster_interface.is_none() == false);
            assert_eq!(cluster.cluster_meta.uuid, cluster_uuid);

            // check initial state of hexagon-data
            let hexagons = cluster.hexagon_data.read().unwrap();
            assert_eq!(hexagons.len(), 1);
        }

        assert_eq!(root_handler.delete_cluster(&cluster_uuid), true);
        assert_eq!(root_handler.delete_cluster(&cluster_uuid), false);
    }

    #[test]
    #[serial]
    fn test_add_blocks_to_cluster() {
        let finish_counter = Arc::new(Mutex::new(FinishCounter::default()));
        let cluster_uuid = Uuid::new_v4();
        let hexagon_uuid0;
        let hexagon_uuid1;
        let cluster_name = "test_cluster".to_string();
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
            test_output: 2,2,2;".to_string();

        let mut root_handler = CLUSTER_HANDLER.write().unwrap();
        root_handler.delete_all_cluster();
        let _ = root_handler.init_new_cluster(&cluster_uuid, &cluster_name, template);

        {
            let cluster = root_handler.clusters.get(&cluster_uuid).unwrap();
            if cluster.cluster_meta.hexagons.values().nth(0).unwrap().is_input {
                hexagon_uuid0 = cluster.cluster_meta.hexagons.keys().nth(0).unwrap().clone();
                hexagon_uuid1 = cluster.cluster_meta.hexagons.keys().nth(1).unwrap().clone();
            } else {
                hexagon_uuid1 = cluster.cluster_meta.hexagons.keys().nth(0).unwrap().clone();
                hexagon_uuid0 = cluster.cluster_meta.hexagons.keys().nth(1).unwrap().clone();
            }
        }

        // prepare new blocks
        let core_block = Arc::new(Mutex::new(CoreBlock::new(&hexagon_uuid0, &cluster_uuid)));
        let input_block = Arc::new(Mutex::new(InputBlock::new(&input_name, &hexagon_uuid0, &cluster_uuid, &finish_counter)));
        let output_block = Arc::new(Mutex::new(OutputBlock::new( &hexagon_uuid1, &cluster_uuid, &output_name)));
        let output_buffer = Arc::new(Mutex::new(OutputBuffer::new(&output_name, &hexagon_uuid1, &cluster_uuid, &OutputType::PlainOutput, &finish_counter)));

        // input-block and output-buffer are already added by initilizing of the cluster, so the names can not be added again
        assert_eq!(root_handler.add_output_buffer(&output_buffer), false);
        assert_eq!(root_handler.add_input_block(&input_block), false);

        // add blocks to cluster
        assert_eq!(root_handler.add_core_block(&core_block), true);
        assert_eq!(root_handler.add_output_block(&output_block), true);
        {
            let cluster = root_handler.clusters.get(&cluster_uuid).unwrap();
            let hexagons = cluster.hexagon_data.read().unwrap();
            assert_eq!(hexagons.len(), 2);
            // check hexagon 0
            {
                let hexagon0 = hexagons.get(&hexagon_uuid0).unwrap();
                assert_eq!(hexagon0.blocks.len(), 2);
                let inputs = cluster.inputs.read().unwrap();
                assert!(inputs.contains_key(&input_name));
            }

            // check hexagon 1
            {
                let hexagon1 = hexagons.get(&hexagon_uuid1).unwrap();
                assert_eq!(hexagon1.blocks.len(), 1);
                let outputs = cluster.outputs.read().unwrap();
                assert!(outputs.contains_key(&output_name));
            }
        }

        // check add blocks with the same ids again
        assert_eq!(root_handler.add_core_block(&core_block), false);
        assert_eq!(root_handler.add_input_block(&input_block), false);
        assert_eq!(root_handler.add_output_block(&output_block), false);
        assert_eq!(root_handler.add_output_buffer(&output_buffer), false);

        // check getter
        assert_eq!(root_handler.get_input_block(&cluster_uuid, &input_name).is_none(), false);
        assert_eq!(root_handler.get_input_block(&cluster_uuid, &output_name).is_none(), true);
        assert_eq!(root_handler.get_output_buffer(&cluster_uuid, &input_name).is_none(), true);
        assert_eq!(root_handler.get_output_buffer(&cluster_uuid, &output_name).is_none(), false);
        assert_eq!(root_handler.get_block(&cluster_uuid, &hexagon_uuid0,&core_block.lock().unwrap().uuid).is_none(), false);
        assert_eq!(root_handler.get_block(&cluster_uuid, &hexagon_uuid1, &Uuid::new_v4()).is_none(), true);

        // delete block and check again
        {
            root_handler.delete_block(&cluster_uuid, &hexagon_uuid0, &core_block.lock().unwrap().uuid);
            let cluster = root_handler.clusters.get(&cluster_uuid).unwrap();
            let hexagons = cluster.hexagon_data.read().unwrap();
            let hexagon0 = hexagons.get(&hexagon_uuid0).unwrap();
            assert_eq!(hexagon0.blocks.len(), 1);
        }
    }

    #[test]
    #[serial]
    fn test_resize() {
        let finish_counter = Arc::new(Mutex::new(FinishCounter::default()));
        let cluster_uuid = Uuid::new_v4();
        let hexagon_uuid0;
        let hexagon_uuid1;
        let cluster_name = "test_cluster".to_string();
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
            key1: 1,1,1; 
        outputs: 
            key2: 2,2,2;".to_string();

        let mut root_handler = CLUSTER_HANDLER.write().unwrap();
        root_handler.delete_all_cluster();
        let _ = root_handler.init_new_cluster(&cluster_uuid, &cluster_name, template);

        {
            let cluster = root_handler.clusters.get(&cluster_uuid).unwrap();
            if cluster.cluster_meta.hexagons.values().nth(0).unwrap().is_input {
                hexagon_uuid0 = cluster.cluster_meta.hexagons.keys().nth(0).unwrap().clone();
                hexagon_uuid1 = cluster.cluster_meta.hexagons.keys().nth(1).unwrap().clone();
            } else {
                hexagon_uuid1 = cluster.cluster_meta.hexagons.keys().nth(0).unwrap().clone();
                hexagon_uuid0 = cluster.cluster_meta.hexagons.keys().nth(1).unwrap().clone();
            }
        }

        // prepare new blocks
        let core_block_mutex = Arc::new(Mutex::new(CoreBlock::new(&hexagon_uuid0, &cluster_uuid)));
        let input_block_mutex = Arc::new(Mutex::new(InputBlock::new(&input_name, &hexagon_uuid0, &cluster_uuid, &finish_counter)));
        let output_block_mutex = Arc::new(Mutex::new(OutputBlock::new( &hexagon_uuid1, &cluster_uuid, &output_name)));
        let output_buffer_mutex = Arc::new(Mutex::new(OutputBuffer::new(&output_name, &hexagon_uuid1, &cluster_uuid, &OutputType::PlainOutput, &finish_counter)));

        // add blocks to cluster
        root_handler.add_core_block(&core_block_mutex);
        root_handler.add_input_block(&input_block_mutex);
        root_handler.add_output_block(&output_block_mutex);
        root_handler.add_output_buffer(&output_buffer_mutex);

        let mut test_section = AxonSection::default();
        let core_block = core_block_mutex.lock().unwrap();
        test_section.source_block_uuid = core_block.uuid.clone();
        test_section.source_hexagon_uuid = core_block.hexagon_uuid.clone();
        test_section.cluster_uuid = core_block.cluster_uuid.clone();
        test_section.source_pos = 0;

        assert_eq!(root_handler.get_target(&mut test_section), true);

        assert_eq!(test_section.source_block_uuid, core_block.uuid);
        assert_eq!(test_section.source_hexagon_uuid, core_block.hexagon_uuid);
        assert_eq!(test_section.cluster_uuid, core_block.cluster_uuid);
        assert_eq!(test_section.source_pos, 0);
    }

    #[test]
    #[serial]
    fn test_create_restore_checkpoint() {
        let file_path_str = "/tmp/test_checkpoint".to_string();
        let file_path: PathBuf = PathBuf::from(&file_path_str);
        match fs::remove_file(&file_path) {
            Ok(_) => {},
            Err(_) => {}
        }

        let finish_counter = Arc::new(Mutex::new(FinishCounter::default()));
        let cluster_uuid = Uuid::new_v4();
        let cluster_uuid_new = Uuid::new_v4();
        let hexagon_uuid0;
        let hexagon_uuid1;
        let cluster_name = "test_cluster".to_string();
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
            test_output: 2,2,2;".to_string();

        let mut root_handler = CLUSTER_HANDLER.write().unwrap();
        root_handler.delete_all_cluster();
        let _ = root_handler.init_new_cluster(&cluster_uuid, &cluster_name, template);

        {
            let cluster = root_handler.clusters.get(&cluster_uuid).unwrap();
            if cluster.cluster_meta.hexagons.values().nth(0).unwrap().is_input {
                hexagon_uuid0 = cluster.cluster_meta.hexagons.keys().nth(0).unwrap().clone();
                hexagon_uuid1 = cluster.cluster_meta.hexagons.keys().nth(1).unwrap().clone();
            } else {
                hexagon_uuid1 = cluster.cluster_meta.hexagons.keys().nth(0).unwrap().clone();
                hexagon_uuid0 = cluster.cluster_meta.hexagons.keys().nth(1).unwrap().clone();
            }
        }

        // prepare new blocks
        let core_block_mutex = Arc::new(Mutex::new(CoreBlock::new(&hexagon_uuid0, &cluster_uuid)));
        let input_block_mutex = Arc::new(Mutex::new(InputBlock::new(&input_name, &hexagon_uuid0, &cluster_uuid, &finish_counter)));
        let output_block_mutex = Arc::new(Mutex::new(OutputBlock::new( &hexagon_uuid1, &cluster_uuid, &output_name)));
        let output_buffer_mutex = Arc::new(Mutex::new(OutputBuffer::new(&output_name, &hexagon_uuid1, &cluster_uuid, &OutputType::PlainOutput, &finish_counter)));

        // add blocks to cluster
        root_handler.add_core_block(&core_block_mutex);
        root_handler.add_input_block(&input_block_mutex);
        root_handler.add_output_block(&output_block_mutex);
        root_handler.add_output_buffer(&output_buffer_mutex);

        // save and restore
        let _ = root_handler.create_checkpoint(&cluster_uuid, &file_path);
        let _ = root_handler.restore_checkpoint(&cluster_uuid_new, &file_path);

        {
            let cluster = root_handler.clusters.get(&cluster_uuid_new).unwrap();
            let hexagons = cluster.hexagon_data.read().unwrap();
            assert_eq!(hexagons.len(), 2);
            // check hexagon 0
            {
                let hexagon0 = hexagons.get(&hexagon_uuid0).unwrap();
                assert_eq!(hexagon0.blocks.len(), 2);
                let inputs = cluster.inputs.read().unwrap();
                assert!(inputs.contains_key(&input_name));
            }

            // check hexagon 1
            {
                let hexagon1 = hexagons.get(&hexagon_uuid1).unwrap();
                assert_eq!(hexagon1.blocks.len(), 1);
                let outputs = cluster.outputs.read().unwrap();
                assert!(outputs.contains_key(&output_name));
            }
        }

        // check getter
        assert_eq!(root_handler.get_input_block(&cluster_uuid_new, &input_name).is_none(), false);
        assert_eq!(root_handler.get_input_block(&cluster_uuid_new, &output_name).is_none(), true);
        assert_eq!(root_handler.get_output_buffer(&cluster_uuid_new, &input_name).is_none(), true);
        assert_eq!(root_handler.get_output_buffer(&cluster_uuid_new, &output_name).is_none(), false);
        assert_eq!(root_handler.get_block(&cluster_uuid_new, &hexagon_uuid0, &core_block_mutex.lock().unwrap().uuid).is_none(), false);
        assert_eq!(root_handler.get_block(&cluster_uuid_new, &hexagon_uuid1, &Uuid::new_v4()).is_none(), true);

    }
}
