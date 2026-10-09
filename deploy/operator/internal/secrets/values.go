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

// Package secrets generates the keys and passwords of an ainari-stack. Every one of them is
// created once in a secret of the namespace, which is never changed afterwards (see Store). The
// pods read most of them directly from these secrets, only the values, which have to be part of
// a config, are handed to the renderer.
package secrets

// Values contains the secret values, which the renderer writes into the configs.
type Values struct {
	// public key, which belongs to MLSGrantSigningKey, base64-encoded. Empty, if
	// hanami.network.mlsEncryption is disabled.
	MLSGrantPublicKey string

	// key of omamori, which is part of its config
	OmamoriEncryptionKey string

	// keys of the pods of the wireguard-tunnel by the name of the pod, empty if
	// global.wireguard is disabled
	WireGuard map[string]KeyPair
}

// KeyPair is a base64-encoded key-pair.
type KeyPair struct {
	PrivateKey string
	PublicKey  string
}
