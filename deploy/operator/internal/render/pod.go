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
	"fmt"
	"strconv"

	appsv1 "k8s.io/api/apps/v1"
	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/resource"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/util/intstr"

	"github.com/kitsudaiki/ainari/deploy/operator/internal/secrets"
)

// appLabels are the labels, which select the pods of a component.
func appLabels(app string) map[string]string {
	return map[string]string{"app": app}
}

// pullPolicy returns the pull-policy of a component. The global value overwrites the one of the
// component, so a local setup can switch all of them at once.
func (r *renderer) pullPolicy(policy corev1.PullPolicy) corev1.PullPolicy {
	if r.Spec.Global.ImagePullPolicy != "" {
		return r.Spec.Global.ImagePullPolicy
	}
	return policy
}

// storageClass returns the storage-class of a component, which is again overwritten by the
// global value.
func (r *renderer) storageClass(storageClass string) *string {
	if r.Spec.Global.StorageClass != "" {
		return &r.Spec.Global.StorageClass
	}
	return &storageClass
}

// affinity returns the affinity of a component. On a real cluster every component runs on the
// nodes, which carry its label, and never twice on the same node. A local cluster has only one
// node, so both rules are switched off there.
func (r *renderer) affinity(app, nodeLabel string) *corev1.Affinity {
	if !r.Spec.Global.StrictScheduling {
		return nil
	}
	return &corev1.Affinity{
		NodeAffinity: &corev1.NodeAffinity{
			RequiredDuringSchedulingIgnoredDuringExecution: &corev1.NodeSelector{
				NodeSelectorTerms: []corev1.NodeSelectorTerm{{
					MatchExpressions: []corev1.NodeSelectorRequirement{{
						Key:      nodeLabel,
						Operator: corev1.NodeSelectorOpIn,
						Values:   []string{"true"},
					}},
				}},
			},
		},
		PodAntiAffinity: &corev1.PodAntiAffinity{
			RequiredDuringSchedulingIgnoredDuringExecution: []corev1.PodAffinityTerm{{
				LabelSelector: &metav1.LabelSelector{
					MatchExpressions: []metav1.LabelSelectorRequirement{{
						Key:      "app",
						Operator: metav1.LabelSelectorOpIn,
						Values:   []string{app},
					}},
				},
				TopologyKey: corev1.LabelHostname,
			}},
		},
	}
}

// singleNodeRollout is the update-strategy of the deployments. Every node runs at most one
// replica of a component (see affinity), so an update can't start an additional pod first. One
// old pod is removed, before its replacement is started.
func singleNodeRollout() appsv1.DeploymentStrategy {
	zero := intstr.FromInt32(0)
	one := intstr.FromInt32(1)
	return appsv1.DeploymentStrategy{
		Type: appsv1.RollingUpdateDeploymentStrategyType,
		RollingUpdate: &appsv1.RollingUpdateDeployment{
			MaxSurge:       &zero,
			MaxUnavailable: &one,
		},
	}
}

// waitForMysql returns an init-container, which waits until the mysql-server accepts
// connections. The components exit at their start, if they can't reach their database, so
// without it, they would be restarted with an increasing delay, while the server is still
// initializing. mysqladmin succeeds, as soon as the server answers, even if it rejects the login.
func (r *renderer) waitForMysql() corev1.Container {
	return corev1.Container{
		Name:            "wait-for-mysql",
		Image:           r.Spec.MySQL.Image,
		ImagePullPolicy: r.pullPolicy(r.Spec.MySQL.ImagePullPolicy),
		Command: []string{"sh", "-c", fmt.Sprintf(`until mysqladmin ping --host=%s --port=%d --connect-timeout=2 > /dev/null 2>&1; do
  echo "waiting for the mysql-server ..."
  sleep 2
done
`, r.mysqlHost(), r.Spec.MySQL.Port)},
	}
}

// waitForMiko returns an init-container, which waits until miko answers. Onsen and sakura
// request the endpoints of the other components from miko at their start and exit, if it is not
// reachable yet, so without it they would be restarted with an increasing delay, while miko is
// still starting. The certificate is not verified, because the check sends no data and only
// waits for an answer.
func (r *renderer) waitForMiko() corev1.Container {
	return corev1.Container{
		Name:  "wait-for-miko",
		Image: r.Spec.Global.NginxImage,
		Command: []string{"sh", "-c", fmt.Sprintf(`until curl --silent --fail --insecure --max-time 3 %s/v1alpha/is_ready > /dev/null; do
  echo "waiting for miko ..."
  sleep 2
done
`, r.mikoAddress())},
	}
}

// envValue returns an env-variable with a fixed value.
func envValue(name, value string) corev1.EnvVar {
	return corev1.EnvVar{Name: name, Value: value}
}

// envSecret returns an env-variable with the value of a key of a secret.
func envSecret(name string, ref secrets.Ref) corev1.EnvVar {
	return corev1.EnvVar{
		Name: name,
		ValueFrom: &corev1.EnvVarSource{
			SecretKeyRef: &corev1.SecretKeySelector{
				LocalObjectReference: corev1.LocalObjectReference{Name: ref.Secret},
				Key:                  ref.Key,
			},
		},
	}
}

// envPodName returns an env-variable with the name of the pod, for example to select the
// directory of the pod on the node with 'subPathExpr: $(POD_NAME)'.
func envPodName() corev1.EnvVar {
	return corev1.EnvVar{
		Name: "POD_NAME",
		ValueFrom: &corev1.EnvVarSource{
			FieldRef: &corev1.ObjectFieldSelector{FieldPath: "metadata.name"},
		},
	}
}

// envInternalAPIKey returns the env-variable with the key, with which the components
// authenticate each other.
func envInternalAPIKey() corev1.EnvVar {
	return envSecret("INTERNAL_API_KEY", secrets.InternalAPIKey)
}

// configMapVolume returns a volume with the content of a configmap.
func configMapVolume(name, configMap string) corev1.Volume {
	return corev1.Volume{
		Name: name,
		VolumeSource: corev1.VolumeSource{
			ConfigMap: &corev1.ConfigMapVolumeSource{
				LocalObjectReference: corev1.LocalObjectReference{Name: configMap},
			},
		},
	}
}

// secretVolume returns a volume with the content of a secret.
func secretVolume(name, secret string) corev1.Volume {
	return corev1.Volume{
		Name:         name,
		VolumeSource: corev1.VolumeSource{Secret: &corev1.SecretVolumeSource{SecretName: secret}},
	}
}

// hostPathVolume returns a volume with a directory of the node.
func hostPathVolume(name, path string, pathType *corev1.HostPathType) corev1.Volume {
	return corev1.Volume{
		Name:         name,
		VolumeSource: corev1.VolumeSource{HostPath: &corev1.HostPathVolumeSource{Path: path, Type: pathType}},
	}
}

// persistentVolumeClaim returns a claim of a volume, which is only used by one node at once.
func persistentVolumeClaim(name string, storageClass *string, size resource.Quantity) *corev1.PersistentVolumeClaim {
	return &corev1.PersistentVolumeClaim{
		ObjectMeta: metav1.ObjectMeta{Name: name},
		Spec: corev1.PersistentVolumeClaimSpec{
			AccessModes:      []corev1.PersistentVolumeAccessMode{corev1.ReadWriteOnce},
			StorageClassName: storageClass,
			Resources: corev1.VolumeResourceRequirements{
				Requests: corev1.ResourceList{corev1.ResourceStorage: size},
			},
		},
	}
}

// fileMount mounts a single file of a volume.
func fileMount(volume, path, key string) corev1.VolumeMount {
	return corev1.VolumeMount{Name: volume, MountPath: path, SubPath: key, ReadOnly: true}
}

// probeTiming is the timing of a readiness- or liveness-probe.
type probeTiming struct {
	initialDelay int32
	period       int32
	failures     int32
}

var (
	readiness = probeTiming{initialDelay: 5, period: 10, failures: 3}
	liveness  = probeTiming{initialDelay: 15, period: 20, failures: 3}
)

// probe returns a probe with a handler and a timing.
func probe(handler corev1.ProbeHandler, timing probeTiming) *corev1.Probe {
	return &corev1.Probe{
		ProbeHandler:        handler,
		InitialDelaySeconds: timing.initialDelay,
		PeriodSeconds:       timing.period,
		TimeoutSeconds:      2,
		SuccessThreshold:    1,
		FailureThreshold:    timing.failures,
	}
}

// curlReady asks the readiness-endpoint of a component on localhost with curl.
func curlReady(port int32) corev1.ProbeHandler {
	return corev1.ProbeHandler{
		Exec: &corev1.ExecAction{
			Command: []string{"curl", "--fail", "localhost:" + strconv.Itoa(int(port)) + "/v1alpha/is_ready"},
		},
	}
}

// httpsGet asks a path over a tls-termination, whose certificate is not verified by the kubelet.
func httpsGet(path string, port int32) corev1.ProbeHandler {
	return corev1.ProbeHandler{
		HTTPGet: &corev1.HTTPGetAction{
			Path:   path,
			Port:   intstr.FromInt32(port),
			Scheme: corev1.URISchemeHTTPS,
		},
	}
}
