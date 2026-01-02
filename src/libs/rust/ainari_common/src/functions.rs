// Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::objects::*;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Component, Path};

pub fn sha256_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let result = hasher.finalize();
    hex::encode(result) // Convert hash bytes to a hexadecimal String
}

pub fn split_bearer_token(token: &str) -> Option<&str> {
    let parts: Vec<&str> = token.splitn(2, ' ').collect();
    if parts.len() == 2 && parts[0] == "Bearer" {
        Some(parts[1])
    } else {
        None
    }
}

pub fn create_sha256_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let result = hasher.finalize();
    format!("{:x}", result)
}

pub fn is_safe_subpath(path: &Path) -> bool {
    // Reject absolute paths immediately
    if path.is_absolute() {
        return false;
    }

    // Check each component of the path
    for comp in path.components() {
        match comp {
            Component::ParentDir => return false, // contains ".."
            Component::RootDir => return false,   // starts with "/"
            _ => {}
        }
    }

    true
}

pub fn clear_directory<P: AsRef<Path>>(dir: P) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}

#[inline]
pub fn pcg_hash(input: &mut u32) -> u32 {
    let state = input.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277_803_737);
    *input = (word >> 22) ^ word;
    *input
}

pub fn get_neighbor_pos(source_pos: &Position, side: usize) -> Position {
    let mut result = Position { x: 0, y: 0, z: 0 };

    match side {
        0 => {
            result.x = if source_pos.y % 2 == 0 {
                source_pos.x - 1
            } else {
                source_pos.x
            };
            result.y = source_pos.y - 1;
            result.z = source_pos.z - 1;
        }
        1 => {
            result.x = if source_pos.y % 2 == 0 {
                source_pos.x
            } else {
                source_pos.x + 1
            };
            result.y = source_pos.y - 1;
            result.z = source_pos.z - 1;
        }
        2 => {
            result.x = source_pos.x;
            result.y = source_pos.y;
            result.z = source_pos.z - 1;
        }
        3 => {
            result.x = if source_pos.y % 2 == 0 {
                source_pos.x
            } else {
                source_pos.x + 1
            };
            result.y = source_pos.y - 1;
            result.z = source_pos.z;
        }
        4 => {
            result.x = source_pos.x + 1;
            result.y = source_pos.y;
            result.z = source_pos.z;
        }
        5 => {
            result.x = if source_pos.y % 2 == 0 {
                source_pos.x
            } else {
                source_pos.x + 1
            };
            result.y = source_pos.y + 1;
            result.z = source_pos.z;
        }
        6 => {
            result.x = if source_pos.y % 2 == 0 {
                source_pos.x - 1
            } else {
                source_pos.x
            };
            result.y = source_pos.y - 1;
            result.z = source_pos.z;
        }
        7 => {
            result.x = source_pos.x - 1;
            result.y = source_pos.y;
            result.z = source_pos.z;
        }
        8 => {
            result.x = if source_pos.y % 2 == 0 {
                source_pos.x - 1
            } else {
                source_pos.x
            };
            result.y = source_pos.y + 1;
            result.z = source_pos.z;
        }
        9 => {
            result.x = source_pos.x;
            result.y = source_pos.y;
            result.z = source_pos.z + 1;
        }
        10 => {
            result.x = if source_pos.y % 2 == 0 {
                source_pos.x - 1
            } else {
                source_pos.x
            };
            result.y = source_pos.y + 1;
            result.z = source_pos.z + 1;
        }
        11 => {
            result.x = if source_pos.y % 2 == 0 {
                source_pos.x
            } else {
                source_pos.x + 1
            };
            result.y = source_pos.y + 1;
            result.z = source_pos.z + 1;
        }
        _ => panic!("Invalid side value: {side}"),
    }

    result
}

pub fn get_next_sides(side: u8) -> [u8; 5] {
    match side {
        0 => [1, 4, 11, 5, 2],
        1 => [2, 8, 10, 7, 0],
        2 => [0, 6, 9, 3, 1],
        3 => [5, 2, 8, 10, 7],
        4 => [8, 10, 7, 0, 6],
        5 => [7, 0, 6, 9, 3],
        6 => [4, 11, 5, 2, 8],
        7 => [3, 1, 4, 11, 5],
        8 => [6, 9, 3, 1, 4],
        9 => [11, 5, 2, 8, 10],
        10 => [9, 3, 1, 4, 11],
        11 => [10, 7, 0, 6, 9],
        _ => panic!("Invalid side value: {side}; This should never happen!"),
    }
}
