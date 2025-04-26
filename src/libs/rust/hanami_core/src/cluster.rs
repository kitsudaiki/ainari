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
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use bytemuck::{cast_slice, cast_slice_mut};
use std::io::SeekFrom;
use std::io::{Read, Seek, BufWriter, BufReader};

use crate::task_queue::{TaskQueue, init_task_queue};
use crate::tasks::{Task, InternalTaskType};
use crate::tasks::TaskVariant;

// HINT (kitsudaiki): ffi is necessary ot get the c++ stuff, defined in the lib.rs
use crate::ffi;
use autocxx::prelude::*;
use cxx::CxxString;

#[derive(Debug, Clone, PartialEq)]
pub enum ClusterMode {
    TaskMode,
    DirectMode,
}

pub struct ClusterLinkHanle {
    pub cluster_link: UniquePtr<ffi::ClusterLink>, 
}

impl ClusterLinkHanle {
    pub fn handle_task(&mut self, task: Task) {
        match task.info {
            TaskVariant::Training(mut task_info) => {
                let mut pos: u64 = 0;

                for i in 0..task_info.number_of_cycles {
                    for (hexagon_name, mut file_handle) in &mut task_info.inputs {  
                        let size_input = (file_handle.header.columns[0].end - file_handle.header.columns[0].start) as usize;
                        let mut input_read = vec![0.0f32; size_input];
                        let byte_slice_input: &mut [u8] = cast_slice_mut(input_read.as_mut_slice());
                        file_handle.target_file.seek(SeekFrom::Start(file_handle.payload_offset + pos)).unwrap();
                        let _ = file_handle.target_file.read_exact(byte_slice_input);
                        let input_ptr: *mut f32 = input_read.as_mut_ptr();

                        pos += 4 * size_input as u64;

                        cxx::let_cxx_string!(cxx_name = hexagon_name); 
                        unsafe {
                            self.cluster_link.pin_mut().fillInput(&cxx_name, input_ptr, size_input as u64);
                        }
                    }

                    for (hexagon_name, mut file_handle) in &mut task_info.outputs {  
                        let size_output = (file_handle.header.columns[1].end - file_handle.header.columns[1].start) as usize;
                        let mut output_read = vec![0.0f32; size_output];
                        let byte_slice_output: &mut [u8] = cast_slice_mut(output_read.as_mut_slice());
                        file_handle.target_file.seek(SeekFrom::Start(file_handle.payload_offset + pos)).unwrap();
                        let _ = file_handle.target_file.read_exact(byte_slice_output);
                        let output_ptr: *mut f32 = output_read.as_mut_ptr();

                        pos += 4 * size_output as u64;

                        cxx::let_cxx_string!(cxx_name = hexagon_name); 
                        unsafe {
                            self.cluster_link.pin_mut().fillExpected(&cxx_name, output_ptr, size_output as u64);
                        }
                    }

                    self.cluster_link.pin_mut().doTrain();
                }
            }, 
            TaskVariant::Request(_) => {}, 
            TaskVariant::CheckpointSave(_) => {
                cxx::let_cxx_string!(file_path_str = "");
                let _: i32 = self.cluster_link.pin_mut().createCheckpoint(&file_path_str).into();
            }, 
            TaskVariant::CheckpointRestore(_) => {}, 
        }
    }
}

// HINT (kitsudaiki): cluster has to be defined here, because otherwise the assiging
// of the cluster_link would fail with an incompatible type error
pub struct Cluster {
    pub uuid: Uuid,
    pub name: String,

    pub mode: ClusterMode,

    pub queue: Arc<Mutex<TaskQueue>>,
    pub cluster_link: Arc<Mutex<ClusterLinkHanle>>,

    pub handle: Option<JoinHandle<()>>,
    pub running: Arc<AtomicBool>,
}

// HINT (kitsudaiki): Workaround to fix the error 
// `*const u8 cannot be sent between threads safely.`, which 
// comes with the `UniquePtr<ffi::HanamiCore>`
unsafe impl Send for ClusterLinkHanle {}

impl Cluster {
    pub fn new(uuid: Uuid, name: String, cluster_link: UniquePtr<ffi::ClusterLink>) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);

        let queue = Arc::new(Mutex::new(init_task_queue()));
        let queue_clone = Arc::clone(&queue);

        let link = Arc::new(Mutex::new(ClusterLinkHanle{cluster_link: cluster_link}));
        let link_clone = Arc::clone(&link);

        let handle = thread::spawn(move || {
            while running_clone.load(Ordering::Relaxed) {
                println!("Looping forever");
                let mut queue_handle = queue_clone.lock().unwrap();

                if let Some(mut task) = queue_handle.get() {
                    drop(queue_handle);

                    //println!("Popped from front: {:?}", task);

                    let mut link_handle = link_clone.lock().unwrap();
                    link_handle.handle_task(task);
                } else {
                    drop(queue_handle);
                    thread::sleep(std::time::Duration::from_secs(1));
                }
            }
            debug!("Thread stopped");
        });

        Cluster {
            name: name,
            uuid: uuid,
            cluster_link: link,
            mode: ClusterMode::TaskMode,
            queue: queue, 
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

    pub fn add_task(&mut self, task: Task) {
        let mut queue_handle = self.queue.lock().unwrap();
        queue_handle.add(task);
    }
}

impl Drop for Cluster {
    fn drop(&mut self) {
        self.stop(); // make sure to stop thread on drop~!
    }
}
