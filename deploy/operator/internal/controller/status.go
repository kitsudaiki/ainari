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
	"fmt"
	"sort"
	"strings"

	appsv1 "k8s.io/api/apps/v1"
	"k8s.io/apimachinery/pkg/api/meta"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"sigs.k8s.io/controller-runtime/pkg/client"

	ainariv1alpha1 "github.com/kitsudaiki/ainari/deploy/operator/api/v1alpha1"
	"github.com/kitsudaiki/ainari/deploy/operator/internal/apply"
)

// state is the outcome of a reconciliation, which is written into the status.
type state struct {
	ready      metav1.ConditionStatus
	reason     string
	message    string
	components []ainariv1alpha1.ComponentStatus
}

// failed returns the state of a reconciliation, which couldn't deploy the stack. The components
// are kept from the previous reconciliation.
func failed(reason, message string) state {
	return state{ready: metav1.ConditionFalse, reason: reason, message: message}
}

// readiness returns the state of the workloads of the stack.
func (r *AinariReconciler) readiness(ctx context.Context, ainari *ainariv1alpha1.Ainari) (state, error) {
	components, err := r.componentStatus(ctx, ainari)
	if err != nil {
		return state{}, err
	}

	var waiting []string
	for _, c := range components {
		if c.ReadyReplicas < c.Replicas {
			waiting = append(waiting, c.Name)
		}
	}
	if len(waiting) > 0 {
		return state{
			ready:      metav1.ConditionFalse,
			reason:     "Progressing",
			message:    "waiting for " + strings.Join(waiting, ", "),
			components: components,
		}, nil
	}
	return state{
		ready:      metav1.ConditionTrue,
		reason:     "Ready",
		message:    "all components are ready",
		components: components,
	}, nil
}

// componentStatus returns the state of all deployments and statefulsets of the stack.
func (r *AinariReconciler) componentStatus(ctx context.Context, ainari *ainariv1alpha1.Ainari) ([]ainariv1alpha1.ComponentStatus, error) {
	selector := []client.ListOption{client.InNamespace(ainari.Namespace), client.MatchingLabels(apply.OwnerLabels(ainari))}
	var components []ainariv1alpha1.ComponentStatus

	deployments := &appsv1.DeploymentList{}
	if err := r.List(ctx, deployments, selector...); err != nil {
		return nil, err
	}
	for _, d := range deployments.Items {
		components = append(components, workloadStatus(d.Name, d.Spec.Replicas, d.Status.ReadyReplicas))
	}

	statefulSets := &appsv1.StatefulSetList{}
	if err := r.List(ctx, statefulSets, selector...); err != nil {
		return nil, err
	}
	for _, s := range statefulSets.Items {
		components = append(components, workloadStatus(s.Name, s.Spec.Replicas, s.Status.ReadyReplicas))
	}

	sort.Slice(components, func(i, j int) bool { return components[i].Name < components[j].Name })
	return components, nil
}

// workloadStatus returns the state of a deployment or statefulset.
func workloadStatus(name string, replicas *int32, ready int32) ainariv1alpha1.ComponentStatus {
	desired := int32(1)
	if replicas != nil {
		desired = *replicas
	}
	return ainariv1alpha1.ComponentStatus{Name: name, Replicas: desired, ReadyReplicas: ready}
}

// updateStatus writes the state of a reconciliation into the status of the Ainari-resource.
func (r *AinariReconciler) updateStatus(ctx context.Context, ainari *ainariv1alpha1.Ainari, s state) error {
	ainari.Status.ObservedGeneration = ainari.Generation
	if s.components != nil {
		ainari.Status.Components = s.components
	}
	meta.SetStatusCondition(&ainari.Status.Conditions, metav1.Condition{
		Type:               ainariv1alpha1.ConditionReady,
		Status:             s.ready,
		Reason:             s.reason,
		Message:            s.message,
		ObservedGeneration: ainari.Generation,
	})

	if err := r.Status().Update(ctx, ainari); err != nil {
		return fmt.Errorf("failed to update the status: %w", err)
	}
	return nil
}
