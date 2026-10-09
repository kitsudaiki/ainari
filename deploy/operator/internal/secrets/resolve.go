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
	"context"
	"fmt"

	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/types"

	ainariv1alpha1 "github.com/kitsudaiki/ainari/deploy/operator/api/v1alpha1"
)

// Resolve creates all secrets, which the stack of an Ainari-resource needs, if they don't exist
// yet, and returns the values, which are part of the configs.
func (s *Store) Resolve(ctx context.Context, ainari *ainariv1alpha1.Ainari) (*Values, error) {
	spec := &ainari.Spec
	namespace := ainari.Namespace

	for _, generated := range requiredSecrets(spec) {
		if _, err := s.ensure(ctx, namespace, generated); err != nil {
			return nil, err
		}
	}
	if !spec.MySQL.Deploy {
		if err := s.checkExternalMySQL(ctx, namespace, &spec.MySQL); err != nil {
			return nil, err
		}
	}

	values := &Values{WireGuard: map[string]KeyPair{}}

	omamori, err := s.ensure(ctx, namespace, singleKey(OmamoriEncryptionKey, generateKey, validateKey))
	if err != nil {
		return nil, err
	}
	values.OmamoriEncryptionKey = omamori[OmamoriEncryptionKey.Key]

	if spec.Hanami.Network.MLSEncryption {
		mls, err := s.ensure(ctx, namespace, singleKey(MLSGrantSigningKey, generateKey, validateKey))
		if err != nil {
			return nil, err
		}
		if values.MLSGrantPublicKey, err = ed25519PublicKey(mls[MLSGrantSigningKey.Key]); err != nil {
			return nil, err
		}
	}

	if members := spec.WireGuardMembers(); len(members) > 0 {
		if values.WireGuard, err = s.resolveWireGuard(ctx, namespace, members); err != nil {
			return nil, err
		}
	}
	return values, nil
}

// requiredSecrets returns the generated secrets, whose values are only read by the pods.
func requiredSecrets(spec *ainariv1alpha1.AinariSpec) []generatedSecret {
	required := []generatedSecret{
		singleKey(InternalAPIKey, generatePassword, nil),
		singleKey(OnsenRegistrationKey, generatePassword, nil),
		singleKey(SakuraRegistrationKey, generatePassword, nil),
		singleKey(MikoTokenKey, generatePassword, nil),
		singleKey(MikoAdminPassphrase, generatePassword, nil),
	}

	if spec.MySQL.Deploy {
		mysql := generatedSecret{
			name: generatedMySQLSecret,
			keys: []generatedKey{{name: MySQLRootPassword.Key, generate: generatePassword}},
		}
		for _, database := range spec.MySQL.Databases.ByComponent() {
			mysql.keys = append(mysql.keys, generatedKey{
				name:     MySQLPassword(&spec.MySQL, database.Component).Key,
				generate: generatePassword,
			})
		}
		required = append(required, mysql)
	}
	return required
}

// resolveWireGuard returns the key-pairs of the pods of the wireguard-tunnel. A new pod gets a
// new key, the keys of the existing ones and of removed ones are kept, so a pod, which comes
// back, gets its old key again.
func (s *Store) resolveWireGuard(ctx context.Context, namespace string, members []ainariv1alpha1.WireGuardMember) (map[string]KeyPair, error) {
	spec := generatedSecret{name: wireGuardKeysSecret, growing: true}
	for _, member := range members {
		spec.keys = append(spec.keys, generatedKey{
			name:     member.PodName(),
			generate: generateWireGuardKey,
			validate: validateKey,
		})
	}
	privateKeys, err := s.ensure(ctx, namespace, spec)
	if err != nil {
		return nil, err
	}

	keys := map[string]KeyPair{}
	for name, privateKey := range privateKeys {
		publicKey, err := wireGuardPublicKey(privateKey)
		if err != nil {
			return nil, fmt.Errorf("invalid wireguard-key of %s: %w", name, err)
		}
		keys[name] = KeyPair{PrivateKey: privateKey, PublicKey: publicKey}
	}
	return keys, nil
}

// checkExternalMySQL checks, if the given secret of an external mysql-server has the passwords
// of all components. It is never changed by the operator.
func (s *Store) checkExternalMySQL(ctx context.Context, namespace string, mysql *ainariv1alpha1.MySQLSpec) error {
	secret := &corev1.Secret{}
	if err := s.Reader.Get(ctx, types.NamespacedName{Namespace: namespace, Name: mysql.CredentialsSecret}, secret); err != nil {
		return fmt.Errorf("failed to read the secret '%s' of mysql.credentialsSecret: %w", mysql.CredentialsSecret, err)
	}
	for _, database := range mysql.Databases.ByComponent() {
		key := MySQLPassword(mysql, database.Component).Key
		if len(secret.Data[key]) == 0 {
			return fmt.Errorf("the secret '%s' of mysql.credentialsSecret has no key '%s'", mysql.CredentialsSecret, key)
		}
	}
	return nil
}

// singleKey describes a generated secret with only one key.
func singleKey(ref Ref, generate func() string, validate func(string) error) generatedSecret {
	return generatedSecret{
		name: ref.Secret,
		keys: []generatedKey{{name: ref.Key, generate: generate, validate: validate}},
	}
}

// validateKey checks, if a value is a base64-encoded key of 32 bytes.
func validateKey(value string) error {
	_, err := decodeKey(value)
	return err
}
