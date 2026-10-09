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

package render_test

import (
	"fmt"
	"reflect"
	"strings"
	"testing"

	appsv1 "k8s.io/api/apps/v1"
	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/resource"
	"sigs.k8s.io/controller-runtime/pkg/client"

	ainariv1alpha1 "github.com/kitsudaiki/ainari/deploy/operator/api/v1alpha1"
	"github.com/kitsudaiki/ainari/deploy/operator/internal/render"
	"github.com/kitsudaiki/ainari/deploy/operator/internal/secrets"
)

// testSpec returns a spec with the defaults of the CRD, like the api-server returns it.
func testSpec() *ainariv1alpha1.AinariSpec {
	return &ainariv1alpha1.AinariSpec{
		Global: ainariv1alpha1.GlobalSpec{
			Ingress:          ainariv1alpha1.IngressSpec{Enabled: true, ClassName: "nginx"},
			WireGuard:        ainariv1alpha1.WireGuardSpec{Enabled: true},
			StrictScheduling: true,
			NginxImage:       "nginx:latest",
			ExternalServices: ainariv1alpha1.ExternalServicesSpec{Type: ainariv1alpha1.ExternalServiceNodePort, NodePortOffset: 20000},
		},
		MySQL: ainariv1alpha1.MySQLSpec{
			Deploy: true, Port: 3306, Image: "mysql:8.4", ImagePullPolicy: corev1.PullIfNotPresent,
			StorageClass: "local-path", StorageSize: resource.MustParse("5Gi"),
		},
		Miko: ainariv1alpha1.MikoSpec{
			Replicas: 1, Image: "kitsudaiki/miko:develop", Domain: "local-miko",
			Admin: ainariv1alpha1.MikoAdminSpec{ID: "admin", Name: "admin"},
			Token: ainariv1alpha1.MikoTokenSpec{ExpireTime: 3600},
		},
		Hanami: ainariv1alpha1.HanamiSpec{
			Replicas: 1, Image: "kitsudaiki/hanami:develop", Domain: "local-hanami",
			Network: ainariv1alpha1.HanamiNetworkSpec{
				FloatingIPCIDR: "10.0.0.0/24", MLSEncryption: true, MLSGrantValidity: 86400, MLSGrantRefreshInterval: 300,
			},
		},
		Sakura:  ainariv1alpha1.SakuraSpec{Replicas: 1, Image: "kitsudaiki/sakura:develop", Domain: "local-sakura", KVMGID: 993, HostDataPath: "/etc/ainari"},
		Ryokan:  ainariv1alpha1.RyokanSpec{Replicas: 1, Image: "kitsudaiki/ryokan:develop", Domain: "local-ryokan"},
		Omamori: ainariv1alpha1.OmamoriSpec{Replicas: 1, Image: "kitsudaiki/omamori:develop", Domain: "local-omamori"},
		Izakaya: ainariv1alpha1.IzakayaSpec{Replicas: 1, Image: "kitsudaiki/izakaya:develop", Domain: "local-izakaya", KeyRotationInterval: 3600, MemberTimeout: 120},
		Onsen: ainariv1alpha1.OnsenSpec{
			Replicas: 1, Image: "kitsudaiki/onsen:develop", StorageClass: "local-path", StorageSize: resource.MustParse("1Gi"),
		},
		Torii: ainariv1alpha1.ToriiSpec{
			Replicas: 1, Image: "kitsudaiki/torii:develop", Domain: "local-torii",
			StorageClass: "local-path", StorageSize: resource.MustParse("100Mi"),
			ServiceType: corev1.ServiceTypeLoadBalancer,
			PortRange:   ainariv1alpha1.PortRangeSpec{Start: 10040, End: 10050},
			Public:      ainariv1alpha1.ToriiPublicSpec{OverlayIface: "veth-gw", UplinkIface: "veth-gw", UplinkNextHop: "10.0.0.1"},
		},
		Dashboard: ainariv1alpha1.DashboardSpec{Enabled: true, Replicas: 1, Image: "kitsudaiki/ainari_dashboard:develop", Domain: "local-ainari"},
	}
}

// testSecrets returns secret values, which fit to the spec, with a distinct key-pair of
// wireguard for every pod of the tunnel.
func testSecrets(spec *ainariv1alpha1.AinariSpec) *secrets.Values {
	values := &secrets.Values{
		MLSGrantPublicKey:    "cOldUtphCTOrQbFpXGDZPIcPfDomKajUzOLVyM8whxo=",
		OmamoriEncryptionKey: "q9vN4CjOQm5wKzyzjZtS7t4oQp8oQK1JvU5xgq8vFzE=",
		WireGuard:            map[string]secrets.KeyPair{},
	}
	for _, member := range spec.WireGuardMembers() {
		name := member.PodName()
		values.WireGuard[name] = secrets.KeyPair{PrivateKey: "private-" + name, PublicKey: "public-" + name}
	}
	return values
}

// renderObjects renders a spec and indexes the objects by their kind and name.
func renderObjects(t *testing.T, spec *ainariv1alpha1.AinariSpec) map[string]client.Object {
	t.Helper()
	objects, err := render.Render(render.Input{Namespace: "test", Spec: spec, Secrets: testSecrets(spec)})
	if err != nil {
		t.Fatalf("failed to render: %v", err)
	}

	index := map[string]client.Object{}
	for _, obj := range objects {
		key := reflect.TypeOf(obj).Elem().Name() + "/" + obj.GetName()
		if obj.GetObjectKind().GroupVersionKind().Kind != "" {
			key = obj.GetObjectKind().GroupVersionKind().Kind + "/" + obj.GetName()
		}
		if _, exists := index[key]; exists {
			t.Fatalf("the object %s is rendered twice", key)
		}
		if obj.GetNamespace() != "test" {
			t.Errorf("the object %s has the namespace '%s'", key, obj.GetNamespace())
		}
		index[key] = obj
	}
	return index
}

func TestRenderDefaultStack(t *testing.T) {
	objects := renderObjects(t, testSpec())

	for _, key := range []string{
		"Deployment/miko", "Deployment/hanami", "Deployment/omamori", "Deployment/izakaya",
		"Deployment/torii-public", "Deployment/dashboard",
		"StatefulSet/mysql", "StatefulSet/ryokan", "StatefulSet/onsen", "StatefulSet/sakura",
		"Issuer/ainari-ca-issuer", "Certificate/miko-cert", "Ingress/miko-ingress",
		"Secret/wg-onsen-secret", "ConfigMap/wireguard-scripts",
	} {
		if _, ok := objects[key]; !ok {
			t.Errorf("the object %s is missing", key)
		}
	}

	// the generated keys and passwords are created by the secrets-package and must never be
	// rendered, otherwise they would be overwritten with every reconciliation
	for _, ref := range []secrets.Ref{
		secrets.InternalAPIKey, secrets.OnsenRegistrationKey, secrets.SakuraRegistrationKey,
		secrets.MLSGrantSigningKey, secrets.MikoTokenKey, secrets.MikoAdminPassphrase,
		secrets.OmamoriEncryptionKey, secrets.MySQLRootPassword,
	} {
		if _, ok := objects["Secret/"+ref.Secret]; ok {
			t.Errorf("the generated secret %s is rendered", ref.Secret)
		}
	}
}

func TestRenderOptionalComponents(t *testing.T) {
	spec := testSpec()
	spec.Dashboard.Enabled = false
	spec.Hanami.Network.MLSEncryption = false
	spec.Global.Ingress.Enabled = false
	spec.Global.WireGuard.Enabled = false
	spec.MySQL.Deploy = false
	spec.MySQL.Host = "mysql.example"
	spec.MySQL.CredentialsSecret = "external-mysql"
	objects := renderObjects(t, spec)

	for key := range objects {
		for _, unwanted := range []string{"dashboard", "izakaya", "Ingress/", "wg-", "wireguard", "mls-grant", "StatefulSet/mysql"} {
			if strings.Contains(key, unwanted) {
				t.Errorf("the object %s is rendered, although it is disabled", key)
			}
		}
	}
	config := objects["ConfigMap/miko-config"].(*corev1.ConfigMap).Data["miko.toml"]
	if !strings.Contains(config, `host = "mysql.example"`) {
		t.Errorf("miko doesn't use the external mysql-server:\n%s", config)
	}
	password := objects["Deployment/miko"].(*appsv1.Deployment).Spec.Template.Spec.Containers[0].Env[0]
	if password.Name != "AINARI_MYSQL_PASSWORD" || password.ValueFrom.SecretKeyRef.Name != "external-mysql" {
		t.Errorf("miko doesn't read its password from the secret of the external server: %+v", password)
	}
}

func TestRenderWireGuardMesh(t *testing.T) {
	spec := testSpec()
	spec.Onsen.Replicas = 3
	spec.Ryokan.Replicas = 2
	spec.Sakura.Replicas = 4
	objects := renderObjects(t, spec)

	// every pod gets its own address and lists the pods of the other side as peers
	addresses := map[string]string{}
	peersOf := map[string][]string{}
	for _, component := range []string{"onsen", "ryokan", "sakura"} {
		data := objects["Secret/wg-"+component+"-secret"].(*corev1.Secret).Data
		replicas := map[string]int{"onsen": 3, "ryokan": 2, "sakura": 4}[component]
		if len(data) != 2*replicas {
			t.Fatalf("the secret of %s has %d keys instead of %d", component, len(data), 2*replicas)
		}
		for i := 0; i < replicas; i++ {
			pod := fmt.Sprintf("%s-%d", component, i)
			config := string(data[pod+".conf"])
			if !strings.Contains(config, "PrivateKey = private-"+pod) {
				t.Errorf("the config of %s has the wrong key:\n%s", pod, config)
			}
			for _, line := range strings.Split(config, "\n") {
				if address, ok := strings.CutPrefix(line, "Address = "); ok {
					addresses[pod] = strings.TrimSuffix(address, "/16")
				}
			}
			for _, line := range strings.Split(strings.TrimSpace(string(data[pod+".peers"])), "\n") {
				fields := strings.Fields(line)
				if len(fields) != 3 {
					t.Fatalf("invalid peer of %s: '%s'", pod, line)
				}
				peer := strings.TrimPrefix(fields[0], "public-")
				peersOf[pod] = append(peersOf[pod], peer+" "+fields[1]+" "+fields[2])
			}
		}
	}

	seen := map[string]string{}
	for pod, address := range addresses {
		if other, ok := seen[address]; ok {
			t.Errorf("%s and %s have the same address %s", pod, other, address)
		}
		seen[address] = pod
	}

	// onsen is connected with all others, ryokan and sakura only with onsen
	for pod, peers := range peersOf {
		wantedPeers := 3
		if strings.HasPrefix(pod, "onsen") {
			wantedPeers = 2 + 4
		}
		if len(peers) != wantedPeers {
			t.Errorf("%s has %d peers instead of %d: %v", pod, len(peers), wantedPeers, peers)
		}
		for _, peer := range peers {
			fields := strings.Fields(peer)
			name := fields[0]
			if strings.HasPrefix(pod, "onsen") == strings.HasPrefix(name, "onsen") {
				t.Errorf("%s has the peer %s of the same side", pod, name)
			}
			if fields[1] != addresses[name]+"/32" {
				t.Errorf("the peer %s of %s has the address %s instead of %s", name, pod, fields[1], addresses[name])
			}
			component := strings.Split(name, "-")[0]
			if want := fmt.Sprintf("%s.%s.test.svc.cluster.local:51820", name, component); fields[2] != want {
				t.Errorf("the peer %s of %s has the endpoint %s instead of %s", name, pod, fields[2], want)
			}
		}
	}

	// every pod of the tunnel runs the sidecar, which keeps its peers in sync
	for _, name := range []string{"onsen", "ryokan", "sakura"} {
		template := objects["StatefulSet/"+name].(*appsv1.StatefulSet).Spec.Template
		if !hasContainer(template.Spec.Containers, "wireguard-sync") {
			t.Errorf("%s has no sidecar wireguard-sync", name)
		}
	}
}

func TestRenderOnsenVolumePerPod(t *testing.T) {
	spec := testSpec()
	spec.Onsen.Replicas = 2
	onsen := renderObjects(t, spec)["StatefulSet/onsen"].(*appsv1.StatefulSet)

	if len(onsen.Spec.VolumeClaimTemplates) != 1 {
		t.Fatalf("onsen has no claim-template for its data")
	}
	if *onsen.Spec.Replicas != 2 {
		t.Errorf("onsen has %d replicas instead of 2", *onsen.Spec.Replicas)
	}
}

func TestRenderEscapesConfigValues(t *testing.T) {
	spec := testSpec()
	spec.Miko.PublicAddress = `https://evil"host\`
	config := renderObjects(t, spec)["ConfigMap/dashboard-config"].(*corev1.ConfigMap).Data["config.json"]
	if !strings.Contains(config, `"apiUrl": "https://evil\"host\\"`) {
		t.Errorf("the address is not escaped:\n%s", config)
	}
	config = renderObjects(t, spec)["ConfigMap/hanami-config"].(*corev1.ConfigMap).Data["hanami.toml"]
	if !strings.Contains(config, `address = "https://miko-tls-service.test.svc.cluster.local:8443"`) {
		t.Errorf("hanami has the wrong address of miko:\n%s", config)
	}
}

func hasContainer(containers []corev1.Container, name string) bool {
	for _, c := range containers {
		if c.Name == name {
			return true
		}
	}
	return false
}
