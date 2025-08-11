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

pub const UNINIT_POINT_32: u32 = 0x0FFFFFFF;
pub const ROWS_IN_READ_BUFFER: u64 = 1000;
pub const RAND_MAX: u32 = 2147483647;

pub const UNINIT_STATE_64: u64 = 0xFFFFFFFFFFFFFFFF;
pub const UNINIT_STATE_32: u32 = 0xFFFFFFFF;
pub const UNINIT_STATE_16: u16 = 0xFFFF;
pub const UNINIT_STATE_8: u8 = 0xFF;

// network-predefines
pub const BLOCK_DIM: usize = 128;
pub const POSSIBLE_NEXT_AXON_STEP: usize = 80;
pub const NUMBER_OF_POSSIBLE_NEXT: usize = 86;
pub const POTENTIAL_BORDER: f32 = 0.00001f32;
