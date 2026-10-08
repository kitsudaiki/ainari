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

// Package render creates all kubernetes-objects of an ainari-stack from the spec of an
// Ainari-resource. It has no access to the cluster, so the same input always results in the same
// objects. Every component has its own file.
package render

import (
	"sigs.k8s.io/controller-runtime/pkg/client"

	ainariv1alpha1 "github.com/kitsudaiki/ainari/deploy/operator/api/v1alpha1"
	"github.com/kitsudaiki/ainari/deploy/operator/internal/secrets"
)

// Input contains everything, which the objects are created from.
type Input struct {
	// namespace of the Ainari-resource, where all objects are created
	Namespace string
	Spec      *ainariv1alpha1.AinariSpec
	Secrets   *secrets.Values
}

// Render returns all objects of the stack. The objects have their name and namespace, but no
// owner-reference yet.
func Render(in Input) ([]client.Object, error) {
	r := &renderer{
		Namespace: in.Namespace,
		Spec:      in.Spec,
		Secrets:   in.Secrets,
	}

	builders := []func() ([]client.Object, error){
		r.issuer,
		r.wireGuard,
		r.mysql,
		r.miko,
		r.hanami,
		r.ryokan,
		r.omamori,
		r.izakaya,
		r.onsen,
		r.sakura,
		r.torii,
		r.dashboard,
	}

	var objects []client.Object
	for _, build := range builders {
		built, err := build()
		if err != nil {
			return nil, err
		}
		objects = append(objects, built...)
	}

	for _, obj := range objects {
		obj.SetNamespace(in.Namespace)
	}
	return objects, nil
}

// renderer holds the input of Render. Its exported fields are also the data of the templates.
type renderer struct {
	Namespace string
	Spec      *ainariv1alpha1.AinariSpec
	Secrets   *secrets.Values
}
