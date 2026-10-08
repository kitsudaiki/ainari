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

// Package apply writes the objects of an owner into the cluster with server-side-apply and
// removes the ones, which are no longer part of it.
package apply

import (
	"context"
	"fmt"

	apierrors "k8s.io/apimachinery/pkg/api/errors"
	"k8s.io/apimachinery/pkg/api/meta"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/apis/meta/v1/unstructured"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/runtime/schema"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/client/apiutil"
	"sigs.k8s.io/controller-runtime/pkg/controller/controllerutil"
)

// Labels of all objects, which are managed by the operator.
const (
	LabelManagedBy = "app.kubernetes.io/managed-by"
	LabelInstance  = "app.kubernetes.io/instance"
	LabelPartOf    = "app.kubernetes.io/part-of"

	// ManagedBy is the value of LabelManagedBy.
	ManagedBy = "ainari-operator"
)

// Applier applies the objects of an owner and prunes the ones, which are no longer desired.
type Applier struct {
	Client client.Client
	// Reader reads directly from the api-server, so pruning doesn't need an informer for every
	// kind of object.
	Reader client.Reader
	Scheme *runtime.Scheme
	// name of the field-manager of server-side-apply
	FieldOwner string
}

// Apply writes an object of the owner into the cluster. The object gets the labels of the
// operator and the owner as controller-reference, so it is deleted together with the owner.
func (a *Applier) Apply(ctx context.Context, owner client.Object, obj client.Object) error {
	gvk, err := apiutil.GVKForObject(obj, a.Scheme)
	if err != nil {
		return err
	}

	labels := obj.GetLabels()
	if labels == nil {
		labels = map[string]string{}
	}
	for key, value := range OwnerLabels(owner) {
		labels[key] = value
	}
	obj.SetLabels(labels)

	if err := controllerutil.SetControllerReference(owner, obj, a.Scheme); err != nil {
		return err
	}

	u, err := toApplyObject(obj, gvk)
	if err != nil {
		return err
	}
	err = a.Client.Apply(ctx, client.ApplyConfigurationFromUnstructured(u), client.FieldOwner(a.FieldOwner), client.ForceOwnership)
	if err != nil {
		return fmt.Errorf("failed to apply %s '%s': %w", gvk.Kind, obj.GetName(), err)
	}
	return nil
}

// Prune deletes all objects of the given kinds, which belong to the owner, but are not part of
// the desired objects anymore, for example the ones of a disabled component. A kind, which is
// not known by the cluster, is skipped.
func (a *Applier) Prune(ctx context.Context, owner client.Object, desired []client.Object, kinds []schema.GroupVersionKind) error {
	keep := map[string]bool{}
	for _, obj := range desired {
		gvk, err := apiutil.GVKForObject(obj, a.Scheme)
		if err != nil {
			return err
		}
		keep[objectKey(gvk, obj.GetName())] = true
	}

	for _, gvk := range kinds {
		list := &metav1.PartialObjectMetadataList{}
		list.SetGroupVersionKind(gvk.GroupVersion().WithKind(gvk.Kind + "List"))
		err := a.Reader.List(ctx, list, client.InNamespace(owner.GetNamespace()), client.MatchingLabels(OwnerLabels(owner)))
		if meta.IsNoMatchError(err) {
			continue
		}
		if err != nil {
			return fmt.Errorf("failed to list the objects of the kind %s: %w", gvk.Kind, err)
		}

		for i := range list.Items {
			obj := &list.Items[i]
			if keep[objectKey(gvk, obj.Name)] || !metav1.IsControlledBy(obj, owner) {
				continue
			}
			obj.SetGroupVersionKind(gvk)
			if err := a.Client.Delete(ctx, obj); err != nil && !apierrors.IsNotFound(err) {
				return fmt.Errorf("failed to delete %s '%s': %w", gvk.Kind, obj.Name, err)
			}
		}
	}
	return nil
}

// OwnerLabels returns the labels, which mark the objects of an owner.
func OwnerLabels(owner client.Object) map[string]string {
	return map[string]string{
		LabelManagedBy: ManagedBy,
		LabelInstance:  owner.GetName(),
		LabelPartOf:    "ainari",
	}
}

// objectKey identifies an object of a kind within the namespace of the owner.
func objectKey(gvk schema.GroupVersionKind, name string) string {
	return gvk.GroupKind().String() + "/" + name
}

// toApplyObject converts an object into the unstructured form, which is sent with
// server-side-apply. Everything the operator doesn't set is removed, so it doesn't claim the
// ownership of fields, which are set by others, like the status or the creation-timestamp.
func toApplyObject(obj client.Object, gvk schema.GroupVersionKind) (*unstructured.Unstructured, error) {
	content, err := runtime.DefaultUnstructuredConverter.ToUnstructured(obj)
	if err != nil {
		return nil, err
	}
	delete(content, "status")
	removeNulls(content)

	u := &unstructured.Unstructured{Object: content}
	u.SetGroupVersionKind(gvk)
	return u, nil
}

// removeNulls removes all null-values from a map and its children. Typed objects contain them
// for fields without 'omitempty', like the creation-timestamp of the template of a deployment.
// Empty objects are kept, because some of them have a meaning, like 'selfSigned: {}' of an
// issuer of cert-manager.
func removeNulls(content map[string]any) {
	for key, value := range content {
		switch v := value.(type) {
		case nil:
			delete(content, key)
		case map[string]any:
			removeNulls(v)
		case []any:
			for _, item := range v {
				if m, ok := item.(map[string]any); ok {
					removeNulls(m)
				}
			}
		}
	}
}
