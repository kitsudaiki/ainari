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
	apierrors "k8s.io/apimachinery/pkg/api/errors"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/types"
	"sigs.k8s.io/controller-runtime/pkg/client"
)

// LabelGenerated marks the secrets, which the operator generated. They don't belong to the
// Ainari-resource, so they stay, when it is deleted, and can be found with this label.
const LabelGenerated = "ainari.kitsunemimi.moe/generated"

// generatedSecret describes a secret, which the operator generates once.
type generatedSecret struct {
	name string
	keys []generatedKey
	// A secret, which gets new keys over time, like the one with the keys of the pods of the
	// wireguard-tunnel, can't be immutable. Its existing keys are kept anyway.
	growing bool
}

// generatedKey is a key of a generated secret.
type generatedKey struct {
	name     string
	generate func() string
	// checks a value, which already exists, optional
	validate func(string) error
}

// Store creates the generated secrets and reads their values.
//
// A secret is only ever created, but never replaced: an existing secret is used as it is, also
// one, which was created by someone else, like the helm-chart or a test-setup, and only missing
// keys are added. So no restart of the operator and no change of the spec ever changes a key,
// with which data was already encrypted, or a password, which a database already has. The
// secrets are created immutable, so also nobody else can change them by accident. They have no
// owner, so they outlive the Ainari-resource and a new one takes them over again.
type Store struct {
	// Reader reads directly from the api-server, so a secret, which was just created, is never
	// missed because of an outdated cache and created a second time.
	Reader client.Reader
	Writer client.Writer
	// labels of the created secrets
	Labels map[string]string
}

// ensure returns the values of a generated secret and creates it or its missing keys first.
func (s *Store) ensure(ctx context.Context, namespace string, spec generatedSecret) (map[string]string, error) {
	secret := &corev1.Secret{}
	err := s.Reader.Get(ctx, types.NamespacedName{Namespace: namespace, Name: spec.name}, secret)
	switch {
	case apierrors.IsNotFound(err):
		secret, err = s.create(ctx, namespace, spec)
	case err != nil:
		err = fmt.Errorf("failed to read the secret '%s': %w", spec.name, err)
	default:
		err = s.addMissingKeys(ctx, secret, spec)
	}
	if err != nil {
		return nil, err
	}

	values := map[string]string{}
	for _, key := range spec.keys {
		value := string(secret.Data[key.name])
		if key.validate != nil {
			if err := key.validate(value); err != nil {
				// never replaced, because data might already be encrypted with it
				return nil, fmt.Errorf("invalid value of the key '%s' in the secret '%s': %w", key.name, spec.name, err)
			}
		}
		values[key.name] = value
	}
	return values, nil
}

// create creates a generated secret with new values.
func (s *Store) create(ctx context.Context, namespace string, spec generatedSecret) (*corev1.Secret, error) {
	labels := map[string]string{LabelGenerated: "true"}
	for key, value := range s.Labels {
		labels[key] = value
	}
	immutable := !spec.growing

	secret := &corev1.Secret{
		ObjectMeta: metav1.ObjectMeta{
			Name:      spec.name,
			Namespace: namespace,
			Labels:    labels,
			Annotations: map[string]string{
				"ainari.kitsunemimi.moe/note": "generated once by the ainari-operator and never changed, " +
					"deleting it loses the access to the data, which depends on it",
			},
		},
		Type:      corev1.SecretTypeOpaque,
		Immutable: &immutable,
		Data:      map[string][]byte{},
	}
	for _, key := range spec.keys {
		secret.Data[key.name] = []byte(key.generate())
	}

	// Fails, if the secret was created in the meantime, for example by a second replica of the
	// operator. The next reconciliation reads it then.
	if err := s.Writer.Create(ctx, secret); err != nil {
		return nil, fmt.Errorf("failed to create the secret '%s': %w", spec.name, err)
	}
	return secret, nil
}

// addMissingKeys adds the keys, which an existing secret doesn't have yet, without changing the
// existing ones. The update fails, if the secret was changed in the meantime.
func (s *Store) addMissingKeys(ctx context.Context, secret *corev1.Secret, spec generatedSecret) error {
	var missing []generatedKey
	for _, key := range spec.keys {
		if len(secret.Data[key.name]) == 0 {
			missing = append(missing, key)
		}
	}
	if len(missing) == 0 {
		return nil
	}
	if secret.Immutable != nil && *secret.Immutable {
		return fmt.Errorf("the immutable secret '%s' misses the key '%s', it has to be recreated by hand", spec.name, missing[0].name)
	}

	if secret.Data == nil {
		secret.Data = map[string][]byte{}
	}
	for _, key := range missing {
		secret.Data[key.name] = []byte(key.generate())
	}
	if err := s.Writer.Update(ctx, secret); err != nil {
		return fmt.Errorf("failed to add the missing keys to the secret '%s': %w", spec.name, err)
	}
	return nil
}
