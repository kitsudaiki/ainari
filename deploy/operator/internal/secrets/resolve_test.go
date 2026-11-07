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
	"strings"
	"testing"

	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/types"
	clientgoscheme "k8s.io/client-go/kubernetes/scheme"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/client/fake"

	ainariv1alpha1 "github.com/kitsudaiki/ainari/deploy/operator/api/v1alpha1"
)

func testAinari() *ainariv1alpha1.Ainari {
	a := &ainariv1alpha1.Ainari{ObjectMeta: metav1.ObjectMeta{Name: "ainari", Namespace: "test"}}
	a.Spec.MySQL.Deploy = true
	return a
}

// newStore returns a store on a fake api-server with the given objects.
func newStore(objects ...client.Object) (*Store, client.Client) {
	scheme := runtime.NewScheme()
	_ = clientgoscheme.AddToScheme(scheme)
	c := fake.NewClientBuilder().WithScheme(scheme).WithObjects(objects...).Build()
	return &Store{Reader: c, Writer: c, Labels: map[string]string{"app.kubernetes.io/instance": "ainari"}}, c
}

func getSecret(t *testing.T, c client.Client, name string) *corev1.Secret {
	t.Helper()
	secret := &corev1.Secret{}
	if err := c.Get(context.Background(), types.NamespacedName{Namespace: "test", Name: name}, secret); err != nil {
		t.Fatalf("failed to read the secret %s: %v", name, err)
	}
	return secret
}

// allValues returns the data of all secrets in the namespace by '<secret>/<key>'.
func allValues(t *testing.T, c client.Client) map[string]string {
	t.Helper()
	list := &corev1.SecretList{}
	if err := c.List(context.Background(), list, client.InNamespace("test")); err != nil {
		t.Fatal(err)
	}
	values := map[string]string{}
	for _, secret := range list.Items {
		for key, value := range secret.Data {
			values[secret.Name+"/"+key] = string(value)
		}
	}
	return values
}

func TestResolveCreatesSecretsOnce(t *testing.T) {
	store, c := newStore()
	ainari := testAinari()

	values, err := store.Resolve(context.Background(), ainari)
	if err != nil {
		t.Fatal(err)
	}
	if values.MLSGrantPublicKey == "" || values.OmamoriEncryptionKey == "" {
		t.Errorf("incomplete values: %+v", values)
	}

	for _, ref := range []Ref{
		InternalAPIKey, OnsenRegistrationKey, SakuraRegistrationKey, MLSGrantSigningKey,
		MikoTokenKey, MikoAdminPassphrase, OmamoriEncryptionKey, MySQLRootPassword,
		MySQLPassword(&ainari.Spec.MySQL, "izakaya"),
		MySQLPassword(&ainari.Spec.MySQL, "omikuji"),
		MySQLPassword(&ainari.Spec.MySQL, "isekai"),
	} {
		secret := getSecret(t, c, ref.Secret)
		if len(secret.Data[ref.Key]) == 0 {
			t.Errorf("the key %s of the secret %s is missing", ref.Key, ref.Secret)
		}
		if secret.Immutable == nil || !*secret.Immutable {
			t.Errorf("the secret %s is not immutable", ref.Secret)
		}
		if len(secret.OwnerReferences) != 0 {
			t.Errorf("the secret %s has an owner, so it would be deleted with the Ainari", ref.Secret)
		}
		if secret.Labels[LabelGenerated] != "true" || secret.Labels["app.kubernetes.io/instance"] != "ainari" {
			t.Errorf("the secret %s has the wrong labels: %v", ref.Secret, secret.Labels)
		}
	}

	// Another run, like after a restart of the operator, and a run with another spec never
	// change an existing value.
	before := allValues(t, c)
	if _, err := store.Resolve(context.Background(), ainari); err != nil {
		t.Fatal(err)
	}
	ainari.Spec.Onsen.Replicas = 3
	if _, err := store.Resolve(context.Background(), ainari); err != nil {
		t.Fatal(err)
	}
	after := allValues(t, c)
	for key, value := range before {
		if after[key] != value {
			t.Errorf("the value %s changed", key)
		}
	}
}

func TestResolveKeepsExistingSecrets(t *testing.T) {
	// created by someone else before, like the passphrase of the test-user of the kind-setup
	existing := &corev1.Secret{
		ObjectMeta: metav1.ObjectMeta{Name: MikoAdminPassphrase.Secret, Namespace: "test"},
		Data:       map[string][]byte{MikoAdminPassphrase.Key: []byte("asdfasdf")},
	}
	// an old secret, which misses a key, which is needed now
	partial := &corev1.Secret{
		ObjectMeta: metav1.ObjectMeta{Name: generatedMySQLSecret, Namespace: "test"},
		Data:       map[string][]byte{"miko_password": []byte("old-password")},
	}
	store, c := newStore(existing, partial)

	if _, err := store.Resolve(context.Background(), testAinari()); err != nil {
		t.Fatal(err)
	}

	admin := getSecret(t, c, MikoAdminPassphrase.Secret)
	if string(admin.Data[MikoAdminPassphrase.Key]) != "asdfasdf" {
		t.Errorf("the existing passphrase was overwritten")
	}
	if admin.Immutable != nil || len(admin.Labels) != 0 {
		t.Errorf("the existing secret was changed")
	}

	mysql := getSecret(t, c, generatedMySQLSecret)
	if string(mysql.Data["miko_password"]) != "old-password" {
		t.Errorf("the existing password was overwritten")
	}
	if len(mysql.Data["root_password"]) == 0 || len(mysql.Data["izakaya_password"]) == 0 {
		t.Errorf("the missing passwords were not added")
	}
}

func TestResolveNeverReplacesInvalidValues(t *testing.T) {
	invalid := &corev1.Secret{
		ObjectMeta: metav1.ObjectMeta{Name: OmamoriEncryptionKey.Secret, Namespace: "test"},
		Data:       map[string][]byte{OmamoriEncryptionKey.Key: []byte("dG9vIHNob3J0")},
	}
	store, c := newStore(invalid)

	_, err := store.Resolve(context.Background(), testAinari())
	if err == nil || !strings.Contains(err.Error(), OmamoriEncryptionKey.Secret) {
		t.Errorf("an invalid key is accepted: %v", err)
	}
	if string(getSecret(t, c, OmamoriEncryptionKey.Secret).Data[OmamoriEncryptionKey.Key]) != "dG9vIHNob3J0" {
		t.Errorf("the invalid key was replaced")
	}
}

func TestResolveImmutableSecretWithMissingKey(t *testing.T) {
	immutable := true
	secret := &corev1.Secret{
		ObjectMeta: metav1.ObjectMeta{Name: generatedMySQLSecret, Namespace: "test"},
		Immutable:  &immutable,
		Data:       map[string][]byte{"root_password": []byte("root")},
	}
	store, _ := newStore(secret)

	if _, err := store.Resolve(context.Background(), testAinari()); err == nil || !strings.Contains(err.Error(), "immutable") {
		t.Errorf("a missing key of an immutable secret is not reported: %v", err)
	}
}

func TestResolveExternalMySQL(t *testing.T) {
	ainari := testAinari()
	ainari.Spec.MySQL.Deploy = false
	ainari.Spec.MySQL.CredentialsSecret = "external"

	store, _ := newStore()
	if _, err := store.Resolve(context.Background(), ainari); err == nil || !strings.Contains(err.Error(), "external") {
		t.Errorf("a missing secret of the external server is accepted: %v", err)
	}

	external := &corev1.Secret{ObjectMeta: metav1.ObjectMeta{Name: "external", Namespace: "test"}, Data: map[string][]byte{}}
	for _, database := range ainari.Spec.MySQL.Databases.ByComponent() {
		external.Data[database.Component+"_password"] = []byte("password")
	}
	store, c := newStore(external)
	if _, err := store.Resolve(context.Background(), ainari); err != nil {
		t.Fatal(err)
	}
	if err := c.Get(context.Background(), types.NamespacedName{Namespace: "test", Name: generatedMySQLSecret}, &corev1.Secret{}); err == nil {
		t.Errorf("passwords are generated for an external server")
	}
}

func TestMLSPublicKey(t *testing.T) {
	// the key-pair of the docker-compose setup
	public, err := ed25519PublicKey("Veuv2tBnfEjOJwFRdf6rDF9CymxxQwvHvAb3vey8Xoo=")
	if err != nil || public != "cOldUtphCTOrQbFpXGDZPIcPfDomKajUzOLVyM8whxo=" {
		t.Errorf("wrong public key %s: %v", public, err)
	}
}
