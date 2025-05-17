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

use log::error;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use uuid::Uuid;

use hanami_cluster_parser::cluster_parser::parse_cluster_template;
use hanami_cluster_parser::cluster_meta_structs::*;
use hanami_common::error::HanamiError;

use super::cluster::Cluster;

// HINT (kitsudaiki): ffi is necessary ot get the c++ stuff, defined in the lib.rs
use crate::ffi;
use autocxx::prelude::*;


lazy_static::lazy_static! {
    pub static ref CLUSTER_HANDLER: Arc<Mutex<ClusterHandler>> = Arc::new(Mutex::new(init_cluster_handler()));
}

pub struct ClusterHandler {
    pub clusters: HashMap<Uuid, Cluster>,
    pub hanami_core: UniquePtr<ffi::HanamiCore>
}

impl ClusterHandler {
    fn search_hexagon(&self, hexagons: &Vec<Position>, pos: &Position) -> i32 {
        let n: usize = hexagons.len();

        for i in 0..n {
            if hexagons[i] == *pos {
                return i as i32;
            }
        }
        -1
    }

    pub fn create_cluster(&mut self, uuid: Uuid, name: String, cluster_template: String) -> Result<(), HanamiError> {
        // parse cluster-template
        let parsed_cluster: ClusterMeta = match parse_cluster_template(cluster_template.as_str()) {
            Ok(parsed) => parsed,
            Err(e) => {
                let msg = format!("Can not create cluster: {:?}", e);
                return Err(HanamiError::InputError(msg));
            }
        };

        // convert template from rust into c++
        let mut cluster_meta = ffi::ClusterMeta::new().within_unique_ptr();
    
        // convert settings to c++
        cluster_meta.pin_mut().setSettings(
            parsed_cluster.settings.neuron_cooldown,
            parsed_cluster.settings.refractory_time,
            parsed_cluster.settings.max_connection_distance);

        // convert axons to c++
        for axon in parsed_cluster.axons {
            let from_id:i32 = self.search_hexagon(&parsed_cluster.hexagons, &axon.from);
            if from_id == -1 {
                let msg = format!("Invalid axon with source: {}", axon.from);
                return Err(HanamiError::InputError(msg));
            }
            let target_id:i32 = self.search_hexagon(&parsed_cluster.hexagons, &axon.to);
            if target_id == -1 {
                let msg = format!("Invalid axon with target: {}", axon.to);
                return Err(HanamiError::InputError(msg));
            }
            cluster_meta.pin_mut().addAxon(from_id as u32, target_id as u32);
        }

        // convert inputs to c++
        for input in parsed_cluster.inputs {
            let id:i32 = self.search_hexagon(&parsed_cluster.hexagons, &input.pos);
            if id == -1 {
                let msg = format!("Invalid input position: {}", input.pos);
                return Err(HanamiError::InputError(msg));
            }
            cxx::let_cxx_string!(name_str = input.name.as_str());

            cluster_meta.pin_mut().addInput(&name_str, id as u32);
        }

        // convert outputs to c++
        for output in parsed_cluster.outputs {
            let id:i32 = self.search_hexagon(&parsed_cluster.hexagons, &output.pos);
            if id == -1 {
                let msg = format!("Invalid output position: {}", output.pos);
                return Err(HanamiError::InputError(msg));
            }
            cxx::let_cxx_string!(name_str = output.name.as_str());

            cluster_meta.pin_mut().addOutput(&name_str, id as u32, output.output_type as u8);
        }

        // convert hexagons to c++
        for hexagon in parsed_cluster.hexagons {
            cluster_meta.pin_mut().addHexagon(hexagon.0, hexagon.1, hexagon.2);
        }
        
        // convert remainting necessary variables from rust into c++
        cxx::let_cxx_string!(uuid_str = uuid.to_string().as_str());
        cxx::let_cxx_string!(name_str = name.as_str());
        let mut error_msg = ffi::make_string("");

        // initialize hanami-code c++-code
        let cluster_link: UniquePtr<ffi::ClusterLink> = self.hanami_core.pin_mut().createCluster(&uuid_str, &name_str, &cluster_meta, error_msg.pin_mut());

        // add cluster to the cluster-handler
        if self.add(uuid, Cluster::new(uuid, cluster_link)) == false {
            let msg = format!("Failed to add cluster with UUID '{uuid}' to cluster-handler");
            return Err(HanamiError::Error(msg));
        }

        Ok(())
    }

    pub fn init_hanami_root(&mut self, max_memory_usage: f32) -> bool {
        let mut error_msg = ffi::make_string("");

        let success = self.hanami_core.pin_mut().init(max_memory_usage, error_msg.pin_mut());

        if !success {
            error!("Initializing hanami-core failed with error: {}", error_msg.to_string());
        }

        success
    }

    fn add(&mut self, uuid: Uuid, cluster: Cluster) -> bool {
        if self.clusters.contains_key(&uuid) {
            false
        } else {
            self.clusters.insert(uuid, cluster);
            true
        }
    }

    pub fn get(&mut self, uuid: &Uuid) -> Option<&mut Cluster> {
        self.clusters.get_mut(uuid)
    }

    pub fn delete(&mut self, uuid: &Uuid) -> bool {
        let cluster = self.clusters.remove(uuid);
        if let Some(mut c) = cluster {
            c.stop();
            return true;
        }

        false
    }
}

pub fn init_cluster_handler() -> ClusterHandler {
    let cluster_handler = ClusterHandler {
        clusters: HashMap::new(),
        hanami_core: ffi::createRootObj(),
    };
    cluster_handler
}

// HINT (kitsudaiki): Workaround to fix the error 
// `*const u8 cannot be sent between threads safely.`, which 
// comes with the `UniquePtr<ffi::HanamiCore>`
unsafe impl Send for ClusterHandler {}

#[cfg(test)]
mod tests {
    use crate::cluster_handler;

    use super::*;

    #[test]
    fn create_cluster() {
        let mut handle = cluster_handler::CLUSTER_HANDLER.lock().unwrap();
        handle.init_hanami_root(0.1f32);

        let key = Uuid::new_v4();
        let name = "test_cluster".to_string();
        let template = "version: 42 
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

        let ret = handle.create_cluster(key, name, template);
    }
}
