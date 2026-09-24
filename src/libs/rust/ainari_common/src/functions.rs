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
use base64::{Engine as _, engine::general_purpose};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Component, Path};

/// The ssh-key-algorithms, which are accepted as public-key.
const SSH_KEY_ALGORITHMS: [&str; 8] = [
    "ssh-ed25519",
    "ssh-rsa",
    "ssh-dss",
    "ecdsa-sha2-nistp256",
    "ecdsa-sha2-nistp384",
    "ecdsa-sha2-nistp521",
    "sk-ssh-ed25519@openssh.com",
    "sk-ecdsa-sha2-nistp256@openssh.com",
];

/// Computes the SHA-256 hash of the given input string and returns it as a hexadecimal string.
///
/// This function takes a string slice as input, processes it through the SHA-256 hashing algorithm,
/// and returns the resulting hash as a hexadecimal string.
pub fn sha256_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let result = hasher.finalize();
    hex::encode(result) // Convert hash bytes to a hexadecimal String
}

/// Decodes the key-blob of an ssh-public-key, if the given input is a valid ssh-public-key.
///
/// An ssh-public-key consists of the algorithm-name, the base64-encoded key-blob and an optional
/// comment, all within a single line. The key-blob itself repeats the algorithm-name as its first
/// entry, so both are compared against each other to reject broken and forged keys.
///
/// # Arguments
///
/// * `public_key` - The ssh-public-key in its one-line openssh-representation
///
/// # Returns
///
/// The decoded key-blob, or None, if the input is not a valid ssh-public-key.
fn decode_ssh_public_key(public_key: &str) -> Option<Vec<u8>> {
    // a public-key is a single line, so anything with more lines is rejected
    if public_key.lines().count() != 1 {
        return None;
    }

    // the comment behind the key-blob is optional and ignored here
    let mut parts = public_key.split_whitespace();
    let algorithm = parts.next()?;
    let encoded_blob = parts.next()?;

    if !SSH_KEY_ALGORITHMS.contains(&algorithm) {
        return None;
    }

    let blob = general_purpose::STANDARD.decode(encoded_blob).ok()?;

    // the blob begins with its algorithm-name as length-prefixed string,
    // which has to match the algorithm in front of the blob
    let length_bytes: [u8; 4] = blob.get(0..4)?.try_into().ok()?;
    let name_length = u32::from_be_bytes(length_bytes) as usize;
    let name_end = 4usize.checked_add(name_length)?;
    if blob.get(4..name_end)? != algorithm.as_bytes() {
        return None;
    }

    Some(blob)
}

/// Checks if the given input is a valid ssh-public-key.
///
/// # Arguments
///
/// * `public_key` - The ssh-public-key in its one-line openssh-representation
///
/// # Returns
///
/// True, if the input is a valid ssh-public-key, else false.
pub fn is_valid_ssh_public_key(public_key: &str) -> bool {
    decode_ssh_public_key(public_key).is_some()
}

/// Calculates the fingerprint of an ssh-public-key.
///
/// The fingerprint is build in the same way as `ssh-keygen -l` does it: the SHA-256 hash of the
/// key-blob, base64-encoded without padding and prefixed with "SHA256:".
///
/// # Arguments
///
/// * `public_key` - The ssh-public-key in its one-line openssh-representation
///
/// # Returns
///
/// The fingerprint of the key, or None, if the input is not a valid ssh-public-key.
pub fn create_ssh_key_fingerprint(public_key: &str) -> Option<String> {
    let blob = decode_ssh_public_key(public_key)?;

    let mut hasher = Sha256::new();
    hasher.update(&blob);
    let hash = hasher.finalize();

    Some(format!(
        "SHA256:{}",
        general_purpose::STANDARD_NO_PAD.encode(hash)
    ))
}

/// Extracts the token part from a Bearer token string.
///
/// This function splits the input string by spaces and checks if the first part is "Bearer".
/// If so, it returns the second part as Some(&str). Otherwise, it returns None.
pub fn split_bearer_token(token: &str) -> Option<&str> {
    let parts: Vec<&str> = token.splitn(2, ' ').collect();
    if parts.len() == 2 && parts[0] == "Bearer" {
        Some(parts[1])
    } else {
        None
    }
}

/// Checks if a given path is a safe subpath.
///
/// A safe subpath is one that doesn't contain parent directory references ("..") or
/// absolute path components. This is useful for preventing directory traversal attacks.
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

/// Clears all files and subdirectories from the specified directory.
///
/// This function removes all contents of the given directory but leaves the directory itself intact.
///
/// # Arguments
///
/// * `dir` - A path to the directory to be cleared, which can be any type that implements `AsRef<Path>`
///
/// # Errors
///
/// Returns an `std::io::Error` if any operation fails during the directory clearing process.
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

/// Computes a PCG (Permuted Congruential Generator) hash of the given u32 value.
///
/// This is a fast, non-cryptographic hash function suitable for general-purpose use.
/// The function updates the input value in place and returns the computed hash.
#[inline]
pub fn pcg_hash(input: &mut u32) -> u32 {
    let state = input.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277_803_737);
    *input = (word >> 22) ^ word;
    *input
}

/// Calculates the position of a neighboring cell in a hexagonal grid.
///
/// Given a source position and a side number (0-11), returns the position of the adjacent cell.
/// The side numbering follows a specific pattern used in hexagonal grid algorithms.
///
/// # Arguments
///
/// * `source_pos` - The position of the source cell
/// * `side` - The side number (0-11) indicating which neighbor to get
///
/// # Panics
///
/// Panics if the side value is out of the valid range (0-11).
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

/// Gets the next five side numbers in a hexagonal grid traversal pattern.
///
/// Given a side number (0-11), returns an array of five side numbers that follow
/// a specific pattern used in hexagonal grid algorithms.
///
/// # Arguments
///
/// * `side` - The starting side number (0-11)
///
/// # Panics
///
/// Panics if the side value is out of the valid range (0-11).
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

#[cfg(test)]
mod tests {
    use super::*;

    // a real, but throw-away ed25519-key together with the fingerprint reported by `ssh-keygen -l`
    const ED25519_KEY: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIKjMONeVqsXz3KbidQwmPCY9pwZYiS/HutS6+DLhv1Cd test@ainari";
    const ED25519_FINGERPRINT: &str = "SHA256:L7ZRuLJJOC5pGhcNx9VLHNz8wFzLGET3pK/Oynybmyk";

    #[test]
    fn test_is_valid_ssh_public_key() {
        // a key with and without comment
        assert!(is_valid_ssh_public_key(ED25519_KEY));
        let without_comment = ED25519_KEY.rsplit_once(' ').unwrap().0;
        assert!(is_valid_ssh_public_key(without_comment));

        // the algorithm in front of the blob has to match the one inside of the blob
        assert!(!is_valid_ssh_public_key(
            &ED25519_KEY.replace("ssh-ed25519 ", "ssh-rsa ")
        ));

        // an unknown algorithm
        assert!(!is_valid_ssh_public_key(
            &ED25519_KEY.replace("ssh-ed25519 ", "ssh-unknown ")
        ));

        // a private-key is not a public-key
        assert!(!is_valid_ssh_public_key(
            "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEA\n-----END OPENSSH PRIVATE KEY-----"
        ));

        // broken and empty input
        assert!(!is_valid_ssh_public_key("ssh-ed25519 not-base64!"));
        assert!(!is_valid_ssh_public_key("ssh-ed25519"));
        assert!(!is_valid_ssh_public_key(""));

        // more than one key at once
        assert!(!is_valid_ssh_public_key(&format!(
            "{ED25519_KEY}\n{ED25519_KEY}"
        )));
    }

    #[test]
    fn test_create_ssh_key_fingerprint() {
        assert_eq!(
            create_ssh_key_fingerprint(ED25519_KEY),
            Some(ED25519_FINGERPRINT.to_string())
        );

        // the comment is not part of the fingerprint
        let without_comment = ED25519_KEY.rsplit_once(' ').unwrap().0;
        assert_eq!(
            create_ssh_key_fingerprint(without_comment),
            Some(ED25519_FINGERPRINT.to_string())
        );

        assert_eq!(create_ssh_key_fingerprint("invalid"), None);
    }
}
