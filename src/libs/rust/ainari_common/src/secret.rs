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

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use subtle::ConstantTimeEq;
use validator::ValidateLength;
use zeroize::Zeroize;

/// Wrapper around a string, which must not be leaked into logs or error-messages.
///
/// `Debug` and `Display` both print `***` instead of the content, so a secret stays hidden even
/// when it is part of a bigger struct, which is logged as a whole. The content is only reachable
/// through `reveal`, which makes every intended access visible at the call-site.
#[derive(Deserialize, Serialize, JsonSchema, Clone)]
pub struct Secret(String);

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "***")
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "***")
    }
}

impl From<String> for Secret {
    fn from(value: String) -> Self {
        Secret(value)
    }
}

impl From<&str> for Secret {
    fn from(value: &str) -> Self {
        Secret(value.to_owned())
    }
}

impl Secret {
    /// Gives access to the wrapped content.
    ///
    /// This is the only way to read a secret, so every place, which really needs the plain value,
    /// is easy to find.
    ///
    /// # Returns
    ///
    /// The wrapped string.
    pub fn reveal(&self) -> &str {
        &self.0
    }
}

/// Compares in constant time, so the duration of a failed comparison against a secret like the
/// internal API-key doesn't reveal, how many leading bytes were correct. Only the length can still
/// be derived from the timing.
impl PartialEq for Secret {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_bytes().ct_eq(other.0.as_bytes()).into()
    }
}
impl Eq for Secret {}

/// Overwrites the content with zeros, before the memory is released, so the secret doesn't
/// remain readable in freed memory.
impl Drop for Secret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl ValidateLength<usize> for Secret {
    fn length(&self) -> Option<usize> {
        Some(self.0.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_eq() {
        assert_eq!(Secret::from("api-key"), Secret::from("api-key"));
        assert_ne!(Secret::from("api-key"), Secret::from("api-kex"));
        assert_ne!(Secret::from("api-key"), Secret::from("api-key-longer"));
        assert_ne!(Secret::from("api-key"), Secret::from(""));
        assert_eq!(Secret::from(""), Secret::from(""));
    }

    #[test]
    fn test_secret_hidden_in_output() {
        let secret = Secret::from("api-key");
        assert_eq!(format!("{secret}"), "***");
        assert_eq!(format!("{secret:?}"), "***");
        assert_eq!(secret.reveal(), "api-key");
    }
}
