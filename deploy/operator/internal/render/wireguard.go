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
	"slices"
	"strings"

	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/util/intstr"
	"sigs.k8s.io/controller-runtime/pkg/client"

	ainariv1alpha1 "github.com/kitsudaiki/ainari/deploy/operator/api/v1alpha1"
)

// The connections of ryokan and sakura to onsen run through a wireguard-tunnel. Every pod of
// onsen, ryokan and sakura has its own key and address and every onsen is a peer of every ryokan
// and every sakura, so ryokan and sakura reach every onsen and every onsen reaches every ryokan.
//
// The component creates the interface at its start from '/etc/wireguard/wg0.conf', which only
// contains the interface itself. The peers are added by the sidecar 'wireguard-sync', which
// keeps them in sync with the secret of the component, so the running pods get new peers, when
// the number of replicas changes.

const (
	// Every component has its own block of addresses within 10.10.0.0/16. The interface gets the
	// whole network, so the routes to all addresses exist from the start, also to the ones of
	// peers, which are added later.
	wireGuardAddressBlock = 16384
	wireGuardPrefix       = 16

	// directory of the sidecar, where the secret of the component is mounted
	wireGuardPeersDir = "/etc/ainari/wireguard"
)

// wireGuardSecretName is the name of the secret with the wireguard-configs of the pods of a
// component. It contains the config '<pod>.conf' of the interface and the peers '<pod>.peers'
// of every pod.
func wireGuardSecretName(component string) string {
	return "wg-" + component + "-secret"
}

// wireGuardAddress returns the address of a pod within the tunnel.
func wireGuardAddress(member ainariv1alpha1.WireGuardMember) string {
	block := slices.Index(ainariv1alpha1.WireGuardComponents, member.Component)
	// the first address of the network is skipped
	host := block*wireGuardAddressBlock + int(member.Ordinal) + 1
	return fmt.Sprintf("10.10.%d.%d", host/256, host%256)
}

// wireGuardEndpoint returns the endpoint of a pod, which is its name within the headless
// service of its component.
func (r *renderer) wireGuardEndpoint(member ainariv1alpha1.WireGuardMember) string {
	return fmt.Sprintf("%s.%s.%s.svc.cluster.local:%d", member.PodName(), member.Component, r.Namespace, wireGuardPort)
}

// wireGuardConnected checks, if the pods of two components are peers of each other. Onsen is
// connected with all other components, which are not connected with each other.
func wireGuardConnected(a, b string) bool {
	return (a == "onsen") != (b == "onsen")
}

// wireGuard returns the secrets with the wireguard-configs of onsen, ryokan and sakura and the
// script of the sidecar, which keeps the peers in sync.
func (r *renderer) wireGuard() ([]client.Object, error) {
	if !r.Spec.Global.WireGuard.Enabled {
		return nil, nil
	}
	members := r.Spec.WireGuardMembers()

	syncScript, err := r.execute("sync_wireguard.sh", r)
	if err != nil {
		return nil, err
	}
	objects := []client.Object{configMap("wireguard-scripts", map[string]string{"sync_wireguard.sh": syncScript})}

	for _, component := range ainariv1alpha1.WireGuardComponents {
		data := map[string]string{}
		for _, member := range members {
			if member.Component != component {
				continue
			}
			config, err := r.execute("wg-interface.conf", map[string]any{
				"PrivateKey": r.Secrets.WireGuard[member.PodName()].PrivateKey,
				"Address":    fmt.Sprintf("%s/%d", wireGuardAddress(member), wireGuardPrefix),
				"ListenPort": wireGuardPort,
			})
			if err != nil {
				return nil, err
			}
			data[member.PodName()+".conf"] = config
			data[member.PodName()+".peers"] = r.wireGuardPeers(member, members)
		}
		objects = append(objects, secret(wireGuardSecretName(component), data))
	}
	return objects, nil
}

// wireGuardPeers returns the peers of a pod in the format of the sync-script.
func (r *renderer) wireGuardPeers(member ainariv1alpha1.WireGuardMember, members []ainariv1alpha1.WireGuardMember) string {
	var peers strings.Builder
	for _, peer := range members {
		if !wireGuardConnected(member.Component, peer.Component) {
			continue
		}
		fmt.Fprintf(&peers, "%s %s/32 %s\n",
			r.Secrets.WireGuard[peer.PodName()].PublicKey,
			wireGuardAddress(peer),
			r.wireGuardEndpoint(peer))
	}
	return peers.String()
}

// wireGuardMount mounts the config of the interface of the pod to the path, where the component
// creates the interface from. The container needs the env-variable POD_NAME.
func wireGuardMount() corev1.VolumeMount {
	return corev1.VolumeMount{
		Name:        "wg-secret",
		MountPath:   "/etc/wireguard/wg0.conf",
		SubPathExpr: "$(POD_NAME).conf",
		ReadOnly:    true,
	}
}

// wireGuardVolumes returns the volumes of wireGuardMount and wireGuardSidecar.
func wireGuardVolumes(component string) []corev1.Volume {
	return []corev1.Volume{
		secretVolume("wg-secret", wireGuardSecretName(component)),
		configMapVolume("wireguard-scripts", "wireguard-scripts"),
	}
}

// wireGuardSidecar returns the sidecar, which keeps the peers of the interface of the pod in
// sync. It uses the image of the component, which already contains the wireguard-tools.
func (r *renderer) wireGuardSidecar(image string, pullPolicy corev1.PullPolicy) corev1.Container {
	root := int64(0)
	return corev1.Container{
		Name:            "wireguard-sync",
		Image:           image,
		ImagePullPolicy: r.pullPolicy(pullPolicy),
		Command:         []string{"/bin/bash", "/etc/ainari/scripts/sync_wireguard.sh"},
		Env:             []corev1.EnvVar{envPodName()},
		SecurityContext: &corev1.SecurityContext{
			RunAsUser:    &root,
			Capabilities: &corev1.Capabilities{Add: []corev1.Capability{"NET_ADMIN"}},
		},
		VolumeMounts: []corev1.VolumeMount{
			// mounted as directory, so updates of the secret reach the running pod
			{Name: "wg-secret", MountPath: wireGuardPeersDir, ReadOnly: true},
			{Name: "wireguard-scripts", MountPath: "/etc/ainari/scripts", ReadOnly: true},
		},
	}
}

// wireGuardServicePorts returns the port of the tunnel for the headless service of a component,
// or nothing, if the tunnel is disabled.
func (r *renderer) wireGuardServicePorts() []corev1.ServicePort {
	if !r.Spec.Global.WireGuard.Enabled {
		return nil
	}
	return []corev1.ServicePort{{
		Name:       "wireguard",
		Protocol:   corev1.ProtocolUDP,
		Port:       wireGuardPort,
		TargetPort: intstr.FromInt32(wireGuardPort),
	}}
}
