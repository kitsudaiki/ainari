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

// The ainari-operator deploys the complete ainari-stack of an Ainari-resource.
package main

import (
	"flag"
	"os"

	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/labels"
	"k8s.io/apimachinery/pkg/runtime"
	clientgoscheme "k8s.io/client-go/kubernetes/scheme"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/cache"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/healthz"
	"sigs.k8s.io/controller-runtime/pkg/log/zap"
	metricsserver "sigs.k8s.io/controller-runtime/pkg/metrics/server"

	ainariv1alpha1 "github.com/kitsudaiki/ainari/deploy/operator/api/v1alpha1"
	"github.com/kitsudaiki/ainari/deploy/operator/internal/apply"
	"github.com/kitsudaiki/ainari/deploy/operator/internal/controller"
)

func main() {
	var metricsAddr, probeAddr string
	var leaderElection bool
	flag.StringVar(&metricsAddr, "metrics-bind-address", ":8080", "address of the metrics-endpoint, '0' to disable it")
	flag.StringVar(&probeAddr, "health-probe-bind-address", ":8081", "address of the health-endpoints")
	flag.BoolVar(&leaderElection, "leader-elect", false, "enables the leader-election, so only one replica of the operator is active")
	zapOptions := zap.Options{}
	zapOptions.BindFlags(flag.CommandLine)
	flag.Parse()

	ctrl.SetLogger(zap.New(zap.UseFlagOptions(&zapOptions)))
	log := ctrl.Log.WithName("setup")

	scheme := runtime.NewScheme()
	if err := clientgoscheme.AddToScheme(scheme); err != nil {
		log.Error(err, "failed to register the kubernetes-types")
		os.Exit(1)
	}
	if err := ainariv1alpha1.AddToScheme(scheme); err != nil {
		log.Error(err, "failed to register the ainari-types")
		os.Exit(1)
	}

	// Only the secrets and configmaps of the operator are cached, not all of the cluster. The
	// secrets, which are referenced by an Ainari-resource, are read directly.
	managed := labels.SelectorFromSet(labels.Set{apply.LabelManagedBy: apply.ManagedBy})
	mgr, err := ctrl.NewManager(ctrl.GetConfigOrDie(), ctrl.Options{
		Scheme:                 scheme,
		Metrics:                metricsserver.Options{BindAddress: metricsAddr},
		HealthProbeBindAddress: probeAddr,
		LeaderElection:         leaderElection,
		LeaderElectionID:       "ainari-operator.ainari.cloud",
		Cache: cache.Options{
			ByObject: map[client.Object]cache.ByObject{
				&corev1.Secret{}:    {Label: managed},
				&corev1.ConfigMap{}: {Label: managed},
			},
		},
	})
	if err != nil {
		log.Error(err, "failed to create the manager")
		os.Exit(1)
	}

	reconciler := &controller.AinariReconciler{
		Client:    mgr.GetClient(),
		APIReader: mgr.GetAPIReader(),
		Applier: &apply.Applier{
			Client:     mgr.GetClient(),
			Reader:     mgr.GetAPIReader(),
			Scheme:     mgr.GetScheme(),
			FieldOwner: apply.ManagedBy,
		},
	}
	if err := reconciler.SetupWithManager(mgr); err != nil {
		log.Error(err, "failed to create the controller")
		os.Exit(1)
	}

	if err := mgr.AddHealthzCheck("healthz", healthz.Ping); err != nil {
		log.Error(err, "failed to add the health-check")
		os.Exit(1)
	}
	if err := mgr.AddReadyzCheck("readyz", healthz.Ping); err != nil {
		log.Error(err, "failed to add the ready-check")
		os.Exit(1)
	}

	log.Info("starting the operator")
	if err := mgr.Start(ctrl.SetupSignalHandler()); err != nil {
		log.Error(err, "failed to run the manager")
		os.Exit(1)
	}
}
