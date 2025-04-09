// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimim.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use serde::Deserialize;
use log::{info, debug, error};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use super::cluster::Cluster;

#[derive(Debug, Deserialize)]
pub struct ClusterHandler {
    pub cluster: HashMap<String, Cluster>,
}

impl ClusterHandler {
    fn add(&mut self, key: String, cluster: Cluster)  -> bool {
        if self.cluster.contains_key(&key) {
            false
        } else {
            self.cluster.insert(key, cluster);
            true
        }
    }

    fn get(&self, key: &String) -> Option<&Cluster> {
        self.cluster.get(key)
    }

    fn delete(&mut self, key: &String) -> Option<Cluster> {
        self.cluster.remove(key)
    }
}

lazy_static::lazy_static! {
    pub static ref CLUSTER_HANDLER: Arc<Mutex<ClusterHandler>> = Arc::new(Mutex::new(init_cluser_handler()));
}

pub fn init_cluser_handler() -> ClusterHandler {
    let cluser_handler = ClusterHandler {
        cluster: HashMap::new(),
    };
    cluser_handler
}

#[cfg(test)]
mod tests {
    use crate::core::cluster::cluster_handler;

    use super::*;

    #[test]
    fn test_add_and_get() {
        let mut handle = cluster_handler::CLUSTER_HANDLER.lock().unwrap();

        let key1 = "key1".to_string();
        let key2 = "key2".to_string();

        let cluster1 = Cluster {
            port: 42,
        };
        let cluster2 = Cluster {
            port: 43,
        };

        handle.add(key1.clone(), cluster1.clone());
        handle.add(key2.clone(), cluster2.clone());

        assert_eq!(handle.get(&key1), Some(cluster1).as_ref());
        assert_eq!(handle.get(&key2), Some(cluster2).as_ref());
    }

    #[test]
    fn test_get_missing_key() {
        let handle = cluster_handler::CLUSTER_HANDLER.lock().unwrap();

        assert_eq!(handle.get(&"missing".to_string()), None);
    }

    #[test]
    fn test_prevent_overwrite() {
        let mut handle = cluster_handler::CLUSTER_HANDLER.lock().unwrap();

        let key = "key".to_string();

        let cluster1 = Cluster {
            port: 42,
        };
        let cluster2 = Cluster {
            port: 43,
        };

        assert!(handle.add(key.clone(), cluster1.clone()));
        assert!(!handle.add(key.clone(), cluster2.clone())); // Should return false!
        assert_eq!(handle.get(&key), Some(cluster1).as_ref());
    }

    #[test]
    fn test_delete_key() {
        let mut handle = cluster_handler::CLUSTER_HANDLER.lock().unwrap();

        let key = "delete_me".to_string();
        let cluster = Cluster {
            port: 42,
        };

        handle.add(key.clone(), cluster.clone());
        let removed = handle.delete(&key);


        assert_eq!(removed, Some(cluster));
        assert_eq!(handle.get(&key), None);
    }
}
