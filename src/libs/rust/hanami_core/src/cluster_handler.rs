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

#![allow(unsafe_code)]

use serde::Deserialize;
use log::{info, debug, error};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use uuid::Uuid;

use autocxx::prelude::*;
use cxx::{let_cxx_string, CxxString};
use std::pin::Pin;

use super::cluster::Cluster;

use hanami_common::enums::ReturnStatus;
use hanami_cluster_parser::cluster_parser::parse_cluster_template;
use hanami_cluster_parser::cluster_meta_structs::*;

autocxx::include_cpp! {
    #include "hanami_root.h"
    #include "hanami_structs.h"
    #include "cluster_link.h"
    safety!(unsafe_ffi)
    generate!("HanamiCore")
    generate!("ReturnStatus")
    generate!("createRootObj")
    generate!("ClusterMeta")
    generate!("ClusterLink")
}

lazy_static::lazy_static! {
    pub static ref CLUSTER_HANDLER: Arc<Mutex<ClusterHandler>> = Arc::new(Mutex::new(init_cluster_handler()));
}

pub struct ClusterHandler {
    pub clusters: HashMap<Uuid, Cluster>,
    pub hanami_core: UniquePtr<ffi::HanamiCore>
}

impl ClusterHandler {
    fn search_hexagon(&self, hexagons: &Vec<Position>, pos: Position) -> i32 {
        let n: usize = hexagons.len();

        for i in 0..n {
            if hexagons[i] == pos {
                return i as i32;
            }
        }
        -1
    }

    pub fn create_cluster(&mut self, uuid: Uuid, name: String, cluster_template: String) -> bool {
        // parse cluster-template
        let parsed_cluster: ClusterMeta = match parse_cluster_template(cluster_template.as_str()) {
            Ok(parsed) => parsed,
            Err(e) => {
                error!("Failed to parse cluster-template: {}", e);
                return false;
            }
        };

        // convert template from rust into c++
        let mut cluster_meta = ffi::ClusterMeta::new().within_unique_ptr();
    
        cluster_meta.pin_mut().setSettings(
            parsed_cluster.settings.neuron_cooldown,
            parsed_cluster.settings.refractory_time,
            parsed_cluster.settings.max_connection_distance);

        for axon in parsed_cluster.axons {
            let from_id:i32 = self.search_hexagon(&parsed_cluster.hexagons, axon.from);
            if from_id == -1 {
                return false;
            }
            let target_id:i32 = self.search_hexagon(&parsed_cluster.hexagons, axon.to);
            if target_id == -1 {
                return false;
            }
            cluster_meta.pin_mut().addAxon(from_id as u32, target_id as u32);
        }

        for input in parsed_cluster.inputs {
            let id:i32 = self.search_hexagon(&parsed_cluster.hexagons, input.pos);
            if id == -1 {
                return false;
            }
            cxx::let_cxx_string!(name_str = input.name.as_str());

            cluster_meta.pin_mut().addInput(&name_str, id as u32);
        }

        for output in parsed_cluster.outputs {
            let id:i32 = self.search_hexagon(&parsed_cluster.hexagons, output.pos);
            if id == -1 {
                return false;
            }
            cxx::let_cxx_string!(name_str = output.name.as_str());

            cluster_meta.pin_mut().addOutput(&name_str, id as u32, output.output_type as u8);
        }

        for hexagon in parsed_cluster.hexagons {
            cluster_meta.pin_mut().addHexagon(hexagon.0, hexagon.1, hexagon.2);
        }
        
        // convert remainting necessary variables from rust into c++
        cxx::let_cxx_string!(uuid_str = uuid.to_string().as_str());
        cxx::let_cxx_string!(name_str = name.as_str());
        let mut error_msg = ffi::make_string("");

        // initialize hanami-code c++-code
        let mut hanami_root_obj = ffi::createRootObj();
        let cpp_status = hanami_root_obj.pin_mut().createCluster(&uuid_str, &name_str, &cluster_meta, error_msg.pin_mut());
        cpp_status.printMetrics();
        //let rust_status = ReturnStatus::from_cpp(cpp_status.0);

        //println!("################################# {}", rust_status);
        true
    }

    pub fn init_hanami_root(&mut self) -> bool {
        let max_memory_usage: f32 = 0.1;
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

    pub fn get(&self, uuid: &Uuid) -> Option<&Cluster> {
        self.clusters.get(uuid)
    }

    pub fn delete(&mut self, uuid: &Uuid) -> Option<Cluster> {
        self.clusters.remove(uuid)
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
    fn test_add_and_get() {
        let mut handle = cluster_handler::CLUSTER_HANDLER.lock().unwrap();

        let key1 = Uuid::new_v4();
        let key2 = Uuid::new_v4();

        let cluster1 = Cluster::default();
        let cluster2 = Cluster::default();

        handle.add(key1.clone(), cluster1.clone());
        handle.add(key2.clone(), cluster2.clone());

        assert_eq!(handle.get(&key1), Some(cluster1).as_ref());
        assert_eq!(handle.get(&key2), Some(cluster2).as_ref());
    }

    #[test]
    fn test_get_missing_key() {
        let handle = cluster_handler::CLUSTER_HANDLER.lock().unwrap();

        assert_eq!(handle.get(&Uuid::new_v4()), None);
    }

    #[test]
    fn test_prevent_overwrite() {
        let mut handle = cluster_handler::CLUSTER_HANDLER.lock().unwrap();

        let key = Uuid::new_v4();

        let cluster1 = Cluster::default();
        let cluster2 = Cluster::default();

        assert!(handle.add(key.clone(), cluster1.clone()));
        assert!(!handle.add(key.clone(), cluster2.clone())); // Should return false!
        assert_eq!(handle.get(&key), Some(cluster1).as_ref());
    }

    #[test]
    fn test_delete_key() {
        let mut handle = cluster_handler::CLUSTER_HANDLER.lock().unwrap();

        let key = Uuid::new_v4();
        let cluster = Cluster::default();

        handle.add(key.clone(), cluster.clone());
        let removed = handle.delete(&key);


        assert_eq!(removed, Some(cluster));
        assert_eq!(handle.get(&key), None);
    }

    #[test]
    fn create_cluster() {
        let mut handle = cluster_handler::CLUSTER_HANDLER.lock().unwrap();
        handle.init_hanami_root();

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
