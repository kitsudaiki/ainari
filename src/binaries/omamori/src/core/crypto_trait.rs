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

use uuid::Uuid;

use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

/// Backend, which holds the payloads of the secrets.
///
/// The trait keeps the endpoints independent of the way the payloads are actually protected, so a
/// different backend can be used without touching the callers. The payload itself never stays in
/// the database of the omamori; only the uuid of the secret is used to address it here.
pub trait CryptoModule {
    /// Encrypts the payload and stores it under the uuid of the secret.
    ///
    /// An already existing payload for the same uuid is replaced.
    fn store(&self, secret_uuid: &Uuid, plaintext: &Secret) -> Result<(), AinariError>;

    /// Reads the payload of a secret again and decrypts it.
    fn retrieve(&self, secret_uuid: &Uuid) -> Result<Secret, AinariError>;

    /// Removes the payload of a secret from the backend.
    fn delete(&self, secret_uuid: &Uuid) -> Result<(), AinariError>;

    /// Returns the name of the backend, which is stored together with the secret, so the correct
    /// backend can be selected again, when the payload is read.
    #[allow(dead_code)]
    fn get_name(&self) -> String;
}
