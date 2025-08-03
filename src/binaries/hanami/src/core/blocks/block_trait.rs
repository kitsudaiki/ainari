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

use uuid::Uuid;
use std::sync::{Arc, Mutex};

use super::axons::*;
use super::block_io::*;

pub trait Block: Send + Sync {
    fn train(&mut self, place_offset: usize, own: Arc<Mutex<dyn Block>>);
    fn process(&mut self,);
    fn backpropagate(&mut self);

    fn get_free_input(&mut self, axon_section: &mut AxonSection) -> bool;
    fn get_uuid(&self) -> Uuid;
    fn get_hexagon_uud(&self) -> Uuid;
    fn get_cluster_uud(&self) -> Uuid;

    fn get_block_io(&mut self) -> &mut BlockIoBuffer;
}
