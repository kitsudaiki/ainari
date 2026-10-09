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

package controller

import (
	"context"
	"os"
	"strings"
	"testing"

	appsv1 "k8s.io/api/apps/v1"
	corev1 "k8s.io/api/core/v1"
	apierrors "k8s.io/apimachinery/pkg/api/errors"
	"k8s.io/apimachinery/pkg/api/meta"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/apis/meta/v1/unstructured"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/types"
	clientgoscheme "k8s.io/client-go/kubernetes/scheme"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/envtest"

	ainariv1alpha1 "github.com/kitsudaiki/ainari/deploy/operator/api/v1alpha1"
	"github.com/kitsudaiki/ainari/deploy/operator/internal/apply"
	"github.com/kitsudaiki/ainari/deploy/operator/internal/secrets"
)

// testEnv is an api-server with the CRDs of the operator and cert-manager.
type testEnv struct {
	client     client.Client
	reconciler *AinariReconciler
}

// startTestEnv starts an api-server, which needs the binaries of envtest in KUBEBUILDER_ASSETS
// (see 'make test').
func startTestEnv(t *testing.T) *testEnv {
	t.Helper()
	if os.Getenv("KUBEBUILDER_ASSETS") == "" {
		t.Skip("KUBEBUILDER_ASSETS is not set, run the tests with 'make test'")
	}

	env := &envtest.Environment{
		CRDDirectoryPaths:     []string{"../../config/crd/bases", "testdata"},
		ErrorIfCRDPathMissing: true,
	}
	cfg, err := env.Start()
	if err != nil {
		t.Fatalf("failed to start envtest: %v", err)
	}
	t.Cleanup(func() { _ = env.Stop() })

	scheme := runtime.NewScheme()
	if err := clientgoscheme.AddToScheme(scheme); err != nil {
		t.Fatal(err)
	}
	if err := ainariv1alpha1.AddToScheme(scheme); err != nil {
		t.Fatal(err)
	}
	c, err := client.New(cfg, client.Options{Scheme: scheme})
	if err != nil {
		t.Fatal(err)
	}

	return &testEnv{
		client: c,
		reconciler: &AinariReconciler{
			Client:    c,
			APIReader: c,
			Applier:   &apply.Applier{Client: c, Reader: c, Scheme: scheme, FieldOwner: apply.ManagedBy},
		},
	}
}

// createAinari creates an Ainari-resource from its spec in yaml-form, so the api-server sets the
// defaults like for a resource of kubectl.
func (e *testEnv) createAinari(t *testing.T, namespace, name string, spec map[string]any) *ainariv1alpha1.Ainari {
	t.Helper()
	ns := &corev1.Namespace{ObjectMeta: metav1.ObjectMeta{Name: namespace}}
	if err := e.client.Create(context.Background(), ns); err != nil && !apierrors.IsAlreadyExists(err) {
		t.Fatal(err)
	}

	obj := &unstructured.Unstructured{Object: map[string]any{"spec": spec}}
	obj.SetGroupVersionKind(ainariv1alpha1.GroupVersion.WithKind("Ainari"))
	obj.SetNamespace(namespace)
	obj.SetName(name)
	if err := e.client.Create(context.Background(), obj); err != nil {
		t.Fatalf("failed to create the Ainari: %v", err)
	}
	return e.getAinari(t, namespace, name)
}

func (e *testEnv) getAinari(t *testing.T, namespace, name string) *ainariv1alpha1.Ainari {
	t.Helper()
	ainari := &ainariv1alpha1.Ainari{}
	if err := e.client.Get(context.Background(), types.NamespacedName{Namespace: namespace, Name: name}, ainari); err != nil {
		t.Fatal(err)
	}
	return ainari
}

func (e *testEnv) reconcile(t *testing.T, ainari *ainariv1alpha1.Ainari) {
	t.Helper()
	request := ctrl.Request{NamespacedName: client.ObjectKeyFromObject(ainari)}
	if _, err := e.reconciler.Reconcile(context.Background(), request); err != nil {
		t.Fatalf("failed to reconcile: %v", err)
	}
}

func (e *testEnv) exists(t *testing.T, obj client.Object, namespace, name string) bool {
	t.Helper()
	err := e.client.Get(context.Background(), types.NamespacedName{Namespace: namespace, Name: name}, obj)
	if apierrors.IsNotFound(err) {
		return false
	}
	if err != nil {
		t.Fatal(err)
	}
	return true
}

func minimalSpec() map[string]any {
	return map[string]any{"miko": map[string]any{"admin": map[string]any{"id": "admin", "name": "admin"}}}
}

func TestReconcile(t *testing.T) {
	env := startTestEnv(t)
	ainari := env.createAinari(t, "stack", "ainari", minimalSpec())

	env.reconcile(t, ainari)
	// a second run with the same spec has nothing to change
	env.reconcile(t, ainari)

	miko := &appsv1.Deployment{}
	if !env.exists(t, miko, "stack", "miko") {
		t.Fatal("the deployment of miko is missing")
	}
	if !metav1.IsControlledBy(miko, ainari) {
		t.Errorf("miko is not owned by the Ainari")
	}
	for _, name := range []string{"ryokan", "onsen", "sakura", "mysql"} {
		if !env.exists(t, &appsv1.StatefulSet{}, "stack", name) {
			t.Errorf("the statefulset of %s is missing", name)
		}
	}
	certificate := &unstructured.Unstructured{}
	certificate.SetAPIVersion("cert-manager.io/v1")
	certificate.SetKind("Certificate")
	if !env.exists(t, certificate, "stack", "miko-cert") {
		t.Errorf("the certificate of miko is missing")
	}

	generated := &corev1.Secret{}
	if !env.exists(t, generated, "stack", secrets.MikoAdminPassphrase.Secret) || len(generated.Data[secrets.MikoAdminPassphrase.Key]) == 0 {
		t.Errorf("the generated passphrase of the admin is missing")
	}
	if len(generated.OwnerReferences) != 0 {
		t.Errorf("the generated passphrase is owned, so it would be deleted with the Ainari")
	}

	ainari = env.getAinari(t, "stack", "ainari")
	ready := meta.FindStatusCondition(ainari.Status.Conditions, ainariv1alpha1.ConditionReady)
	// envtest has no controllers, which start the pods
	if ready == nil || ready.Status != metav1.ConditionFalse || ready.Reason != "Progressing" {
		t.Errorf("unexpected ready-condition: %+v", ready)
	}
	if ainari.Status.MLSGrantPublicKey == "" || len(ainari.Status.Components) == 0 {
		t.Errorf("the status is incomplete: %+v", ainari.Status)
	}
}

func TestReconcileNeverChangesSecrets(t *testing.T) {
	env := startTestEnv(t)
	ns := &corev1.Namespace{ObjectMeta: metav1.ObjectMeta{Name: "keep"}}
	if err := env.client.Create(context.Background(), ns); err != nil {
		t.Fatal(err)
	}
	// created before, like the test-user of the kind-setup
	admin := &corev1.Secret{
		ObjectMeta: metav1.ObjectMeta{Name: secrets.MikoAdminPassphrase.Secret, Namespace: "keep"},
		Data:       map[string][]byte{secrets.MikoAdminPassphrase.Key: []byte("asdfasdf")},
	}
	if err := env.client.Create(context.Background(), admin); err != nil {
		t.Fatal(err)
	}

	ainari := env.createAinari(t, "keep", "ainari", minimalSpec())
	env.reconcile(t, ainari)
	before := env.secretValues(t, "keep")

	// another reconciliation, a change of the spec and a new Ainari after the old one was
	// deleted all keep the values
	ainari = env.getAinari(t, "keep", "ainari")
	ainari.Spec.Onsen.Replicas = 2
	if err := env.client.Update(context.Background(), ainari); err != nil {
		t.Fatal(err)
	}
	env.reconcile(t, ainari)
	if err := env.client.Delete(context.Background(), ainari); err != nil {
		t.Fatal(err)
	}
	ainari = env.createAinari(t, "keep", "recreated", minimalSpec())
	env.reconcile(t, ainari)

	after := env.secretValues(t, "keep")
	if len(before) == 0 {
		t.Fatal("no generated secrets found")
	}
	for key, value := range before {
		if after[key] != value {
			t.Errorf("the value %s changed", key)
		}
	}

	if !env.exists(t, admin, "keep", secrets.MikoAdminPassphrase.Secret) || string(admin.Data[secrets.MikoAdminPassphrase.Key]) != "asdfasdf" {
		t.Errorf("the existing passphrase was overwritten")
	}
}

// secretValues returns the data of all generated secrets of a namespace by '<secret>/<key>'.
func (e *testEnv) secretValues(t *testing.T, namespace string) map[string]string {
	t.Helper()
	list := &corev1.SecretList{}
	err := e.client.List(context.Background(), list, client.InNamespace(namespace), client.MatchingLabels{secrets.LabelGenerated: "true"})
	if err != nil {
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

func TestReconcileChangesOfTheSpec(t *testing.T) {
	env := startTestEnv(t)
	ainari := env.createAinari(t, "changes", "ainari", minimalSpec())
	env.reconcile(t, ainari)

	ainari = env.getAinari(t, "changes", "ainari")
	ainari.Spec.Dashboard.Enabled = false
	ainari.Spec.Onsen.Replicas = 3
	if err := env.client.Update(context.Background(), ainari); err != nil {
		t.Fatal(err)
	}
	env.reconcile(t, ainari)

	// a disabled component is removed
	if env.exists(t, &appsv1.Deployment{}, "changes", "dashboard") {
		t.Errorf("the dashboard still exists, although it is disabled")
	}
	if env.exists(t, &corev1.ConfigMap{}, "changes", "dashboard-config") {
		t.Errorf("the config of the dashboard still exists, although it is disabled")
	}

	// the running pods of ryokan get the new onsen-pods as peers
	wgRyokan := &corev1.Secret{}
	if !env.exists(t, wgRyokan, "changes", "wg-ryokan-secret") {
		t.Fatal("the wireguard-secret of ryokan is missing")
	}
	peers := strings.Split(strings.TrimSpace(string(wgRyokan.Data["ryokan-0.peers"])), "\n")
	if len(peers) != 3 {
		t.Errorf("ryokan has %d peers instead of the 3 onsen-pods: %v", len(peers), peers)
	}
}

func TestOnlyOneStackPerNamespace(t *testing.T) {
	env := startTestEnv(t)
	first := env.createAinari(t, "shared", "first", minimalSpec())
	env.reconcile(t, first)

	second := env.createAinari(t, "shared", "second", minimalSpec())
	env.reconcile(t, second)

	second = env.getAinari(t, "shared", "second")
	ready := meta.FindStatusCondition(second.Status.Conditions, ainariv1alpha1.ConditionReady)
	if ready == nil || ready.Reason != "Conflict" {
		t.Errorf("the second Ainari is not marked as conflict: %+v", ready)
	}
	miko := &appsv1.Deployment{}
	if !env.exists(t, miko, "shared", "miko") || !metav1.IsControlledBy(miko, first) {
		t.Errorf("the second Ainari took over the stack of the first one")
	}
}

func TestValidation(t *testing.T) {
	env := startTestEnv(t)
	ns := &corev1.Namespace{ObjectMeta: metav1.ObjectMeta{Name: "validation"}}
	if err := env.client.Create(context.Background(), ns); err != nil {
		t.Fatal(err)
	}

	for name, change := range map[string]func(spec map[string]any){
		"external mysql without host": func(spec map[string]any) {
			spec["mysql"] = map[string]any{"deploy": false}
		},
		"inverted port-range": func(spec map[string]any) {
			spec["torii"] = map[string]any{"portRange": map[string]any{"start": 2000, "end": 1000}}
		},
		"external mysql without credentials": func(spec map[string]any) {
			spec["mysql"] = map[string]any{"deploy": false, "host": "mysql.example"}
		},
	} {
		t.Run(name, func(t *testing.T) {
			spec := minimalSpec()
			change(spec)
			obj := &unstructured.Unstructured{Object: map[string]any{"spec": spec}}
			obj.SetGroupVersionKind(ainariv1alpha1.GroupVersion.WithKind("Ainari"))
			obj.SetNamespace("validation")
			obj.SetName("invalid")
			if err := env.client.Create(context.Background(), obj); err == nil {
				t.Errorf("the invalid spec was accepted")
			}
		})
	}
}
