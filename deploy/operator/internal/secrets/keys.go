// Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>
//
// Licensed under the Apache License, Version 2.0 (the "License")
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

package secrets

import (
	"crypto/ecdh"
	"crypto/ed25519"
	"crypto/rand"
	"encoding/base64"
	"fmt"
)

// keySize is the size of all generated keys and of the keys, which are given as base64-encoded
// 32 bytes.
const keySize = 32

// randomBytes returns keySize random bytes.
func randomBytes() []byte {
	key := make([]byte, keySize)
	// never fails, see the documentation of crypto/rand
	_, _ = rand.Read(key)
	return key
}

// generatePassword returns a random password, which only consists of letters, digits, '-' and
// '_', so it can be used in configs and shell-scripts without escaping.
func generatePassword() string {
	return base64.RawURLEncoding.EncodeToString(randomBytes())
}

// generateKey returns a random key of keySize bytes in standard base64-encoding.
func generateKey() string {
	return base64.StdEncoding.EncodeToString(randomBytes())
}

// generateWireGuardKey returns a random private key of wireguard, like 'wg genkey'.
func generateWireGuardKey() string {
	key := randomBytes()
	// clamping of curve25519
	key[0] &= 248
	key[31] = (key[31] & 127) | 64
	return base64.StdEncoding.EncodeToString(key)
}

// decodeKey decodes a base64-encoded key of keySize bytes.
func decodeKey(encoded string) ([]byte, error) {
	key, err := base64.StdEncoding.DecodeString(encoded)
	if err != nil {
		return nil, fmt.Errorf("not a valid base64-encoded string")
	}
	if len(key) != keySize {
		return nil, fmt.Errorf("expected %d bytes, got %d", keySize, len(key))
	}
	return key, nil
}

// wireGuardPublicKey derives the public key of a private key of wireguard, like 'wg pubkey'.
func wireGuardPublicKey(privateKey string) (string, error) {
	seed, err := decodeKey(privateKey)
	if err != nil {
		return "", err
	}
	key, err := ecdh.X25519().NewPrivateKey(seed)
	if err != nil {
		return "", err
	}
	return base64.StdEncoding.EncodeToString(key.PublicKey().Bytes()), nil
}

// ed25519PublicKey derives the public key of an Ed25519 private key, which is given as its seed.
func ed25519PublicKey(seed string) (string, error) {
	decoded, err := decodeKey(seed)
	if err != nil {
		return "", err
	}
	public := ed25519.NewKeyFromSeed(decoded).Public().(ed25519.PublicKey)
	return base64.StdEncoding.EncodeToString(public), nil
}
