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

use log::{info, debug, error};
use uuid::Uuid;
use std::thread::{self, JoinHandle};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::task_queue::TaskQueue;

// HINT (kitsudaiki): ffi is necessary ot get the c++ stuff, defined in the lib.rs
use crate::ffi;
use autocxx::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum ClusterMode {
    TaskMode,
    DirectMode,
}

// HINT (kitsudaiki): cluster has to be defined here, because otherwise the assiging
// of the cluster_link would fail with an incompatible type error
pub struct Cluster {
    pub uuid: Uuid,
    pub name: String,

    pub mode: ClusterMode,

    pub queue: TaskQueue,

    pub cluster_link: UniquePtr<ffi::ClusterLink>, 

    pub handle: Option<JoinHandle<()>>,
    pub running: Arc<AtomicBool>,
}

impl Cluster {
    pub fn new(uuid: Uuid, name: String, cluster_link: UniquePtr<ffi::ClusterLink>) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);

        let handle = thread::spawn(move || {
            while running_clone.load(Ordering::Relaxed) {
                println!("Looping forever");
                thread::sleep(std::time::Duration::from_secs(1));
            }
            debug!("Thread stopped");
        });

        Cluster {
            name: name,
            uuid: uuid,
            cluster_link: cluster_link,
            mode: ClusterMode::TaskMode,
            queue: TaskQueue::default(),    
            handle: Some(handle),
            running,
        }
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }

    pub fn create_checkpoint(&mut self, file_path: String) -> bool {
        cxx::let_cxx_string!(file_path_str = file_path.as_str());

        // create checkpoint in c++-code. 
        // HINT (kitsudaiki): have to use into() to convert c_int into i32
        let ret: i32 = self.cluster_link.pin_mut().createCheckpoint(&file_path_str).into();
        if ret != 0 {
            return false;
        }
        true
    }
}

impl Drop for Cluster {
    fn drop(&mut self) {
        self.stop(); // make sure to stop thread on drop~!
    }
}
