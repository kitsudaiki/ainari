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

use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

pub trait CryptoModule {
    fn encrypt(&self, plaintext: &Secret, key_b64: &Secret) -> Result<String, AinariError>;
    fn decrypt(&self, encrypted_secret_b64: &str, key_b64: &Secret) -> Result<Secret, AinariError>;
    #[allow(dead_code)]
    fn get_name(&self) -> String;
}
