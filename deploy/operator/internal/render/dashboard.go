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

package render

import (
	appsv1 "k8s.io/api/apps/v1"
	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"sigs.k8s.io/controller-runtime/pkg/client"
)

// dashboard returns the web-dashboard, if it is enabled.
func (r *renderer) dashboard() ([]client.Object, error) {
	spec := &r.Spec.Dashboard
	if !spec.Enabled {
		return nil, nil
	}

	// the browser talks to miko directly, so it gets the public address of miko
	config, err := r.execute("dashboard-config.json", r)
	if err != nil {
		return nil, err
	}
	// The image serves the dashboard on port 80 of all addresses. It is only reachable over the
	// tls-termination, like every other component, so it only listens on localhost here.
	defaultConf, err := r.execute("dashboard-default.conf", r)
	if err != nil {
		return nil, err
	}
	nginxConfig, err := r.nginxConfig(nginxParams{
		Listen:         internalTLSPort,
		Upstream:       80,
		RedirectDomain: spec.Domain,
		HTTPRedirect:   true,
	})
	if err != nil {
		return nil, err
	}

	objects := []client.Object{
		configMap("dashboard-config", map[string]string{
			"config.json":  config,
			"default.conf": defaultConf,
		}),
		configMap(nginxConfigName("dashboard"), map[string]string{"nginx.conf": nginxConfig}),
		r.dashboardDeployment(),
		tlsService("dashboard", "dashboard", internalTLSPort),
		r.externalService("dashboard", dashboardPort, internalTLSPort),
		r.certificate("dashboard", spec.Domain, tlsServiceName("dashboard")),
	}
	return append(objects, r.ingress("dashboard", spec.Domain, tlsServiceName("dashboard"), internalTLSPort)...), nil
}

// dashboardDeployment returns the deployment of the dashboard.
func (r *renderer) dashboardDeployment() *appsv1.Deployment {
	spec := &r.Spec.Dashboard

	// the dashboard only listens on localhost, so it is asked over its tls-termination
	ready := httpsGet("/", internalTLSPort)
	dashboard := corev1.Container{
		Name:            "dashboard",
		Image:           spec.Image,
		ImagePullPolicy: r.pullPolicy(spec.ImagePullPolicy),
		ReadinessProbe:  probe(ready, readiness),
		LivenessProbe:   probe(ready, liveness),
		VolumeMounts: []corev1.VolumeMount{
			fileMount("dashboard-config", "/usr/share/nginx/html/config.json", "config.json"),
			fileMount("dashboard-config", "/etc/nginx/conf.d/default.conf", "default.conf"),
		},
	}

	volumes := append(tlsVolumes("dashboard"), configMapVolume("dashboard-config", "dashboard-config"))

	return &appsv1.Deployment{
		ObjectMeta: metav1.ObjectMeta{Name: "dashboard", Labels: appLabels("dashboard")},
		Spec: appsv1.DeploymentSpec{
			Replicas: &spec.Replicas,
			Strategy: singleNodeRollout(),
			Selector: &metav1.LabelSelector{MatchLabels: appLabels("dashboard")},
			Template: corev1.PodTemplateSpec{
				ObjectMeta: metav1.ObjectMeta{Labels: appLabels("dashboard")},
				Spec: corev1.PodSpec{
					Affinity: r.affinity("dashboard", "ainari-dashboard-node"),
					Containers: []corev1.Container{
						dashboard,
						r.tlsSidecar("tls-termination", nginxConfigName("dashboard"), "nginx.conf", tlsCertsVolume),
					},
					Volumes: volumes,
				},
			},
		},
	}
}
