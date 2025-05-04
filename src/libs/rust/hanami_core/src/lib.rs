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

pub mod cluster_handler;
pub mod cluster;
pub mod task_queue;
pub mod tasks;

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
