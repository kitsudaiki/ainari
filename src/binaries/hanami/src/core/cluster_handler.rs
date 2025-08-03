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

use hanami_cluster_parser::cluster_parser::parse_cluster_template;
use hanami_cluster_parser::cluster_meta_structs::*;
use hanami_common::error::HanamiError;
use hanami_common::constants::*;

use crate::core::blocks::axons::AxonSection;
use crate::core::blocks::input_block::*;
use crate::core::blocks::core_block::*;
use crate::core::blocks::output_block::*;
use crate::core::processing::output_buffer::OutputBuffer;

use super::processing::cluster_interface::ClusterInterface;
use super::blocks::block_trait::Block;

lazy_static::lazy_static! {
    pub static ref CLUSTER_HANDLER: RwLock<ClusterDataHandler> = RwLock::new(init_cluster_data_handler());
}

// ==================================================================================================

#[derive(Default)]
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

pub struct HexagonIO {
    pub input_block: Option<Arc<Mutex<InputBlock>>>,
    pub output_buffer: Option<Arc<Mutex<OutputBuffer>>>,
}

impl HexagonIO {
    pub fn new() -> Self {
        HexagonIO {
            input_block: None,
            output_buffer: None,
        }
    }
}

// ==================================================================================================

pub struct ClusterContent {
    pub cluster_meta: ClusterMeta,
    pub hexagon_data: RwLock<HashMap<Uuid, HexagonData>>,
    pub hexagon_io: RwLock<HashMap<String, HexagonIO>>,
    pub cluster_interface: Option<Arc<Mutex<ClusterInterface>>>,
}

impl ClusterContent {
    pub fn new(cluster_meta: ClusterMeta) -> Self {
        ClusterContent {
            cluster_meta: cluster_meta,
            hexagon_data: RwLock::new(HashMap::new()),
            hexagon_io: RwLock::new(HashMap::new()),
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
        let finish_counter_arc = Arc::new(Mutex::new(FinishCounter::default()));
        let mut finish_counter = finish_counter_arc.lock().unwrap();
        let interface = Arc::new(Mutex::new(ClusterInterface::new(&cluster_uuid, &finish_counter_arc)));

        // add cluster to the cluster-handler
        parsed_cluster.uuid = cluster_uuid.clone();
        if self.register_cluster(&parsed_cluster, Some(interface)) == false {
            let msg = format!("Failed to add cluster with UUID '{cluster_uuid}' to cluster-handler");
            return Err(HanamiError::Error(msg));
        }

        // initialize input-blocks
        for input_meta in parsed_cluster.inputs.iter() {
            let input_block_arc = Arc::new(Mutex::new(InputBlock::new(&input_meta.name, &input_meta.hexagon_uuid, &cluster_uuid, &finish_counter_arc)));
            if self.add_input_block(&input_block_arc) == false {
                let msg = format!("Failed to add input-block with name '{}' to cluster-handler", input_meta.name);
                return Err(HanamiError::Error(msg));
            }
        }
        finish_counter.input_compare = parsed_cluster.inputs.len();

        // initilize output-buffer
        for output_meta in parsed_cluster.outputs.iter() {
            let output_buffer_arc = Arc::new(Mutex::new(OutputBuffer::new(&output_meta.name, &output_meta.hexagon_uuid, &cluster_uuid, &output_meta.output_type, &finish_counter_arc)));
            if self.add_output_buffer(&output_buffer_arc) == false {
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
    pub fn add_core_block(&mut self, block_arc: &Arc<Mutex<CoreBlock>>) -> bool {
        return self.add_block(&(block_arc.clone() as Arc<Mutex<dyn Block>>));
    }

    /**
     * 
     */
    pub fn add_output_block(&mut self, block_arc: &Arc<Mutex<OutputBlock>>) -> bool {
        return self.add_block(&(block_arc.clone() as Arc<Mutex<dyn Block>>));
    }
    
    /**
     * 
     */
    fn add_input_block(&mut self, block_arc: &Arc<Mutex<InputBlock>>) -> bool {
        let input_block = block_arc.lock().unwrap();
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
        let mut hexagon_io_map = cluster_link.hexagon_io.write().unwrap();
        if hexagon_io_map.contains_key(&block_name) == false {
            hexagon_io_map.insert(block_name.clone(), HexagonIO::new());
        } else {
            return false;
        }

        let hexgon_io = if let Some(h) = hexagon_io_map.get_mut(&block_name) {
            h
        } else {
            return false;
        };

        // a hexagon can only have one input-block
        if hexgon_io.input_block.is_none() == false {
            return false;
        }

        // add new block
        let block_uuid = input_block.get_uuid();
        if hexgon_link.blocks.contains_key(&block_uuid) == false {
            hexgon_link.blocks.insert(block_uuid.clone(), Arc::clone(block_arc) as Arc<Mutex<dyn Block>>);
            hexgon_io.input_block = Some(block_arc.clone());
            return true;
        } 

        false
    }

    /**
     * 
     */  
    fn add_output_buffer(&mut self, block_arc: &Arc<Mutex<OutputBuffer >>) -> bool {
        let output_buffer = block_arc.lock().unwrap();
        let cluster_uuid = output_buffer.cluster_uuid.clone();
        let name = output_buffer.name.clone();

        // get cluster
        let cluster_link = if let Some(c) = self.clusters.get_mut(&cluster_uuid)  {
            c
        } else {
            return false;
        };

        // get hexagon-io
        let mut hexagon_io_map = cluster_link.hexagon_io.write().unwrap();
        if hexagon_io_map.contains_key(&name) == false {
            hexagon_io_map.insert(name.clone(), HexagonIO::new());
        } else {
            return false;
        }

        let hexgon_io = if let Some(h) = hexagon_io_map.get_mut(&name) {
            h
        } else {
            return false;
        };

        // add new block
        if hexgon_io.output_buffer.is_none() {
            hexgon_io.output_buffer = Some(Arc::clone(block_arc));
            return true;
        } 

        false
    }
      
    /**
     * 
     */
    fn add_block(&mut self, block_arc: &Arc<Mutex<dyn Block>>) -> bool {
        let block = block_arc.lock().unwrap();
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
            hexgon_link.blocks.insert(block_uuid.clone(), Arc::clone(block_arc));
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

        let binding = cluster_link.hexagon_io.read().unwrap();
        let hexagon_link = if let Some(h) = binding.get(name) {
            h
        } else {
            return None;
        };

        if let Some(input_block_arc) = &hexagon_link.input_block {
            return Some(input_block_arc.clone());
        }

        None
    }

    /**
     * 
     */
    pub fn get_number_of_io(&self, cluster_uuid: &Uuid) -> (usize, usize) {
        let cluster_link = if let Some(c) = self.clusters.get(cluster_uuid) {
            c
        } else {
            return (0, 0);
        };

        (cluster_link.cluster_meta.inputs.len(), cluster_link.cluster_meta.outputs.len())
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

        let binding = cluster_link.hexagon_io.read().unwrap();
        let hexagon_link = if let Some(h) = binding.get(name) {
            h
        } else {
            return None;
        };

        if let Some(output_buffer_arc) = &hexagon_link.output_buffer {
            return Some(output_buffer_arc.clone());
        }

        None
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
            for block_arc in target_hexagon_link.blocks.values() {
                let mut block = block_arc.lock().unwrap();
                if block.get_free_input(axon_section) != false {
                    axon_section.target_block = Some(block_arc.clone());
                    axon_section.source_block = Some(source_block);
                    return true;
                }
            }
        }

        // create new block
        if is_output_hexagon {
            let output_block_arc = Arc::new(Mutex::new(OutputBlock::new(&target_hexagon_uuid, &axon_section.cluster_uuid, &output_hexagon_name)));
            if self.add_output_block(&output_block_arc) == false {
                return false;
            }
            let mut output_block = output_block_arc.lock().unwrap();
            if output_block.get_free_input(axon_section) {
                axon_section.target_block = Some(output_block_arc.clone());
                axon_section.source_block = Some(source_block);
                return true;
            }
        } else {
            let core_block_arc = Arc::new(Mutex::new(CoreBlock::new(&target_hexagon_uuid, &axon_section.cluster_uuid)));
            if self.add_core_block(&core_block_arc) == false {
                return false;
            }
            let mut core_block = core_block_arc.lock().unwrap();
            if core_block.get_free_input(axon_section) {
                axon_section.target_block = Some(core_block_arc.clone());
                axon_section.source_block = Some(source_block);
                return true;
            }
        }

        false
    }
}


#[cfg(test)]
mod tests {
    use crate::core::blocks::core_block::CoreBlock;
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
            let hexagons_io = cluster.hexagon_io.read().unwrap();
            assert_eq!(hexagons.len(), 2);
            // check hexagon 0
            let hexagon0 = hexagons.get(&hexagon_uuid0).unwrap();
            assert_eq!(hexagon0.blocks.len(), 2);
            assert_eq!(hexagons_io.get(&input_name).unwrap().input_block.is_none(), false);
            assert_eq!(hexagons_io.get(&input_name).unwrap().output_buffer.is_none(), true);
            // check hexagon 1
            let hexagon1 = hexagons.get(&hexagon_uuid1).unwrap();
            assert_eq!(hexagon1.blocks.len(), 1);
            assert_eq!(hexagons_io.get(&output_name).unwrap().input_block.is_none(), true);
            assert_eq!(hexagons_io.get(&output_name).unwrap().output_buffer.is_none(), false);
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
        let core_block_arc = Arc::new(Mutex::new(CoreBlock::new(&hexagon_uuid0, &cluster_uuid)));
        let input_block_arc = Arc::new(Mutex::new(InputBlock::new(&input_name, &hexagon_uuid0, &cluster_uuid, &finish_counter)));
        let output_block_arc = Arc::new(Mutex::new(OutputBlock::new( &hexagon_uuid1, &cluster_uuid, &output_name)));
        let output_buffer_arc = Arc::new(Mutex::new(OutputBuffer::new(&output_name, &hexagon_uuid1, &cluster_uuid, &OutputType::PlainOutput, &finish_counter)));

        // add blocks to cluster
        root_handler.add_core_block(&core_block_arc);
        root_handler.add_input_block(&input_block_arc);
        root_handler.add_output_block(&output_block_arc);
        root_handler.add_output_buffer(&output_buffer_arc);

        let mut test_section = AxonSection::default();
        let core_block = core_block_arc.lock().unwrap();
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
}
