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

pub mod backup;
pub mod bundle;
pub mod keys;
pub mod manifest;
pub mod processing;
pub mod restore;
pub mod services;

/// Smallest allowed parameters of the key-derivation, so the tests stay fast.
#[cfg(test)]
pub(crate) const TEST_KDF_PARAMS: ainari_common::kdf::KdfParams = ainari_common::kdf::KdfParams {
    memory_kib: ainari_common::kdf::KDF_MIN_MEMORY_KIB,
    iterations: ainari_common::kdf::KDF_MIN_ITERATIONS,
    parallelism: ainari_common::kdf::KDF_MIN_PARALLELISM,
};
