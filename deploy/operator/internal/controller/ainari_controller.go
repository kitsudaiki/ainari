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

// Package controller contains the reconciler of the Ainari-resource.
package controller

import (
	"context"
	"fmt"
	"time"

	appsv1 "k8s.io/api/apps/v1"
	corev1 "k8s.io/api/core/v1"
	networkingv1 "k8s.io/api/networking/v1"
	"k8s.io/apimachinery/pkg/runtime/schema"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/builder"
	"sigs.k8s.io/controller-runtime/pkg/client"
	logf "sigs.k8s.io/controller-runtime/pkg/log"
	"sigs.k8s.io/controller-runtime/pkg/predicate"

	ainariv1alpha1 "github.com/kitsudaiki/ainari/deploy/operator/api/v1alpha1"
	"github.com/kitsudaiki/ainari/deploy/operator/internal/apply"
	"github.com/kitsudaiki/ainari/deploy/operator/internal/render"
	"github.com/kitsudaiki/ainari/deploy/operator/internal/secrets"
)

// resyncInterval is the time, after which a deployed stack is reconciled again, for example to
// check again the secret of an external mysql-server, which is not watched.
const resyncInterval = 5 * time.Minute

// prunedKinds are all kinds of objects, which the renderer creates. Objects of these kinds,
// which belong to an Ainari-resource, but are not rendered anymore, are deleted.
var prunedKinds = []schema.GroupVersionKind{
	appsv1.SchemeGroupVersion.WithKind("Deployment"),
	appsv1.SchemeGroupVersion.WithKind("StatefulSet"),
	corev1.SchemeGroupVersion.WithKind("Service"),
	corev1.SchemeGroupVersion.WithKind("ConfigMap"),
	corev1.SchemeGroupVersion.WithKind("Secret"),
	corev1.SchemeGroupVersion.WithKind("PersistentVolumeClaim"),
	networkingv1.SchemeGroupVersion.WithKind("Ingress"),
	{Group: "cert-manager.io", Version: "v1", Kind: "Issuer"},
	{Group: "cert-manager.io", Version: "v1", Kind: "Certificate"},
}

// AinariReconciler deploys the stack of an Ainari-resource.
type AinariReconciler struct {
	client.Client
	// APIReader reads directly from the api-server. It is used for the generated secrets, so a
	// secret, which was just created, is never missed because of an outdated cache.
	APIReader client.Reader
	Applier   *apply.Applier
}

// +kubebuilder:rbac:groups=ainari.kitsunemimi.moe,resources=ainaris,verbs=get;list;watch;update;patch
// +kubebuilder:rbac:groups=ainari.kitsunemimi.moe,resources=ainaris/status,verbs=get;update;patch
// +kubebuilder:rbac:groups=apps,resources=deployments;statefulsets,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups="",resources=services;configmaps;secrets;persistentvolumeclaims,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=networking.k8s.io,resources=ingresses,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=cert-manager.io,resources=issuers;certificates,verbs=get;list;watch;create;update;patch;delete

// Reconcile deploys the stack of an Ainari-resource and updates its status.
func (r *AinariReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	ainari := &ainariv1alpha1.Ainari{}
	if err := r.Get(ctx, req.NamespacedName, ainari); err != nil {
		return ctrl.Result{}, client.IgnoreNotFound(err)
	}
	// all objects are removed by the garbage-collector over their owner-reference
	if !ainari.DeletionTimestamp.IsZero() {
		return ctrl.Result{}, nil
	}

	// The objects have fixed names, because the components find each other over them, so there
	// can only be one stack per namespace.
	owner, err := r.namespaceOwner(ctx, ainari)
	if err != nil {
		return ctrl.Result{}, err
	}
	if owner != ainari.Name {
		// checked again later, so it takes over, once the other one is deleted
		msg := fmt.Sprintf("the namespace already contains the stack of the Ainari '%s'", owner)
		return ctrl.Result{RequeueAfter: resyncInterval}, r.updateStatus(ctx, ainari, failed("Conflict", msg))
	}

	if err := r.deploy(ctx, ainari); err != nil {
		logf.FromContext(ctx).Error(err, "failed to deploy the stack")
		if statusErr := r.updateStatus(ctx, ainari, failed("DeploymentFailed", err.Error())); statusErr != nil {
			return ctrl.Result{}, statusErr
		}
		return ctrl.Result{}, err
	}

	readiness, err := r.readiness(ctx, ainari)
	if err != nil {
		return ctrl.Result{}, err
	}
	return ctrl.Result{RequeueAfter: resyncInterval}, r.updateStatus(ctx, ainari, readiness)
}

// deploy applies all objects of the stack and removes the ones, which are not needed anymore.
func (r *AinariReconciler) deploy(ctx context.Context, ainari *ainariv1alpha1.Ainari) error {
	// The keys and passwords are created first and never changed afterwards. Their secrets have
	// no owner and are not rendered, so they are never pruned or deleted with the resource.
	store := &secrets.Store{Reader: r.APIReader, Writer: r.Client, Labels: apply.OwnerLabels(ainari)}
	values, err := store.Resolve(ctx, ainari)
	if err != nil {
		return err
	}
	ainari.Status.MLSGrantPublicKey = values.MLSGrantPublicKey

	objects, err := render.Render(render.Input{
		Namespace: ainari.Namespace,
		Spec:      &ainari.Spec,
		Secrets:   values,
	})
	if err != nil {
		return err
	}
	for _, obj := range objects {
		if err := r.Applier.Apply(ctx, ainari, obj); err != nil {
			return err
		}
	}
	return r.Applier.Prune(ctx, ainari, objects, prunedKinds)
}

// namespaceOwner returns the name of the Ainari-resource, which deploys the stack of the
// namespace. It is the oldest one, so a newer one can't take over an existing stack.
func (r *AinariReconciler) namespaceOwner(ctx context.Context, ainari *ainariv1alpha1.Ainari) (string, error) {
	list := &ainariv1alpha1.AinariList{}
	if err := r.List(ctx, list, client.InNamespace(ainari.Namespace)); err != nil {
		return "", err
	}

	oldest := ainari
	for i := range list.Items {
		other := &list.Items[i]
		if !other.DeletionTimestamp.IsZero() {
			continue
		}
		created, oldestCreated := other.CreationTimestamp, oldest.CreationTimestamp
		if created.Before(&oldestCreated) || (created.Equal(&oldestCreated) && other.Name < oldest.Name) {
			oldest = other
		}
	}
	return oldest.Name, nil
}

// SetupWithManager registers the reconciler at the manager.
func (r *AinariReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		// the status is written by the reconciler itself, so only changes of the spec count
		For(&ainariv1alpha1.Ainari{}, builder.WithPredicates(predicate.GenerationChangedPredicate{})).
		Owns(&appsv1.Deployment{}).
		Owns(&appsv1.StatefulSet{}).
		Owns(&corev1.Service{}).
		Owns(&corev1.ConfigMap{}).
		Owns(&corev1.Secret{}).
		Owns(&corev1.PersistentVolumeClaim{}).
		Owns(&networkingv1.Ingress{}).
		Named("ainari").
		Complete(r)
}
