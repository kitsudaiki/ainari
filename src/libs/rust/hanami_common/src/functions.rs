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

use sha2::{Sha256, Digest};

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

pub fn fast_mem_copy<T: Copy>(src: &Vec<T>, dst: &mut Vec<T>) {
    assert_eq!(src.len(), dst.len(), "Vectors must be the same length!");

    unsafe {
        std::ptr::copy_nonoverlapping(
            src.as_ptr(),
            dst.as_mut_ptr(),
            src.len(),
        );
    }
}

#[inline]
pub fn pcg_hash(input: &mut u32) -> u32 {
    let state = input.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277_803_737);
    *input = (word >> 22) ^ word;
    *input
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fast_mem_copy() {
        let source_vec = vec![1.0f32, 2.0f32, 3.0f32];
        let mut destination_vec = vec![0.0f32, 0.0f32, 0.0f32];

        fast_mem_copy(&source_vec, &mut destination_vec);

        assert_eq!(destination_vec[0], 1.0f32);
        assert_eq!(destination_vec[1], 2.0f32);
        assert_eq!(destination_vec[2], 3.0f32);
    }
}
