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

// Package v1alpha1 contains the API of the ainari-operator.
// +kubebuilder:object:generate=true
// +groupName=ainari.cloud
package v1alpha1

import (
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/runtime/schema"
)

var (
	// GroupVersion is the group and version of the API.
	GroupVersion = schema.GroupVersion{Group: "ainari.cloud", Version: "v1alpha1"}

	// SchemeBuilder registers the types of the API.
	SchemeBuilder = runtime.NewSchemeBuilder(addKnownTypes)

	// AddToScheme adds the types of the API to a scheme.
	AddToScheme = SchemeBuilder.AddToScheme
)

// addKnownTypes registers the types of the API in the given scheme.
func addKnownTypes(scheme *runtime.Scheme) error {
	scheme.AddKnownTypes(GroupVersion, &Ainari{}, &AinariList{})
	metav1.AddToGroupVersion(scheme, GroupVersion)
	return nil
}
