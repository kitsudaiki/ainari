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
	ainariv1alpha1 "github.com/kitsudaiki/ainari/deploy/operator/api/v1alpha1"
)

// Ref is a key within a secret, which the pods read with a secretKeyRef or a volume.
type Ref struct {
	Secret string
	Key    string
}

// The secrets, which the operator generates. Most names are the same as the ones of the former
// helm-chart, so the secrets of such a deployment are taken over with its keys and passwords.
var (
	// key, with which the components authenticate each other
	InternalAPIKey = Ref{Secret: "internal-api-key", Key: "internal_api_key"}
	// key, with which onsen registers at ryokan
	OnsenRegistrationKey = Ref{Secret: "onsen-registration-key", Key: "onsen_retistration_key"}
	// key, with which sakura registers at hanami
	SakuraRegistrationKey = Ref{Secret: "sakura-registration-key", Key: "sakura_retistration_key"}
	// Ed25519 private key as base64-encoded 32 byte seed, with which hanami signs the
	// membership-grants of the MLS-groups
	MLSGrantSigningKey = Ref{Secret: "mls-grant-signing-key", Key: "mls_grant_signing_key"}
	// key, with which miko signs its tokens
	MikoTokenKey = Ref{Secret: "token-key", Key: "token-key"}
	// passphrase of the admin, which miko creates at its first start
	MikoAdminPassphrase = Ref{Secret: "miko-admin", Key: "passphrase"}
	// base64-encoded 32 byte key, with which omamori encrypts the stored secrets
	OmamoriEncryptionKey = Ref{Secret: "omamori-encryption-key", Key: "key"}
	// password of the root-user of the deployed mysql-server
	MySQLRootPassword = Ref{Secret: generatedMySQLSecret, Key: "root_password"}
)

const (
	// secret with the passwords of the deployed mysql-server
	generatedMySQLSecret = "mysql-credentials"
	// secret with the private keys of all pods of the wireguard-tunnel
	wireGuardKeysSecret = "wireguard-keys"
)

// MySQLPassword returns the password of the database-user of a component, which is either in the
// generated secret of the deployed server or in the given secret of the external one.
func MySQLPassword(mysql *ainariv1alpha1.MySQLSpec, component string) Ref {
	secret := generatedMySQLSecret
	if !mysql.Deploy {
		secret = mysql.CredentialsSecret
	}
	return Ref{Secret: secret, Key: component + "_password"}
}
