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
	"reflect"
	"strings"
	"testing"

	appsv1 "k8s.io/api/apps/v1"
	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/resource"
	"k8s.io/apimachinery/pkg/apis/meta/v1/unstructured"
	"sigs.k8s.io/controller-runtime/pkg/client"

	ainariv1alpha1 "github.com/kitsudaiki/ainari/deploy/operator/api/v1alpha1"
	"github.com/kitsudaiki/ainari/deploy/operator/internal/render"
	"github.com/kitsudaiki/ainari/deploy/operator/internal/secrets"
)

// testSpec returns a spec with the defaults of the CRD, like the api-server returns it.
func testSpec() *ainariv1alpha1.AinariSpec {
	return &ainariv1alpha1.AinariSpec{
		Global: ainariv1alpha1.GlobalSpec{
			Gateway:          ainariv1alpha1.GatewaySpec{Enabled: true, Name: "ainari-gateway", Namespace: "ainari-gateway", Listener: "tls-passthrough"},
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
				FloatingIPCIDR: "10.0.0.0/24", MLSGrantValidity: 86400, MLSGrantRefreshInterval: 300,
			},
		},
		Sakura:  ainariv1alpha1.SakuraSpec{Replicas: 1, Image: "kitsudaiki/sakura:develop", Domain: "local-sakura", KVMGID: 993, HostDataPath: "/etc/ainari"},
		Ryokan:  ainariv1alpha1.RyokanSpec{Replicas: 1, Image: "kitsudaiki/ryokan:develop", Domain: "local-ryokan"},
		Omamori: ainariv1alpha1.OmamoriSpec{Replicas: 1, Image: "kitsudaiki/omamori:develop", Domain: "local-omamori"},
		Omikuji: ainariv1alpha1.OmikujiSpec{Replicas: 1, Image: "kitsudaiki/omikuji:develop", Domain: "local-omikuji"},
		Isekai: ainariv1alpha1.IsekaiSpec{
			Replicas: 1, Image: "kitsudaiki/isekai:develop", Domain: "local-isekai",
			Processing: ainariv1alpha1.IsekaiProcessingSpec{NumberOfWorkers: 4, MaxParallelSnapshots: 2},
			KDF:        ainariv1alpha1.IsekaiKDFSpec{MemoryKiB: 65536, Iterations: 3, Parallelism: 4},
			BackupStorage: ainariv1alpha1.BackupStorageSpec{
				Replicas: 1, StorageClass: "local-path", StorageSize: resource.MustParse("10Gi"),
			},
		},
		Izakaya: ainariv1alpha1.IzakayaSpec{Replicas: 1, Image: "kitsudaiki/izakaya:develop", Domain: "local-izakaya", KeyRotationInterval: 3600, MemberTimeout: 120},
		Onsen: ainariv1alpha1.OnsenSpec{
			Replicas: 1, Image: "kitsudaiki/onsen:develop", StorageClass: "local-path", StorageSize: resource.MustParse("1Gi"),
		},
		Torii: ainariv1alpha1.ToriiSpec{
			Replicas: 1, Image: "kitsudaiki/torii:develop", Domain: "local-torii",
			StorageClass: "local-path", StorageSize: resource.MustParse("100Mi"),
			ServiceType: corev1.ServiceTypeLoadBalancer,
			Proxy:       ainariv1alpha1.ToriiProxySpec{Domain: "ainari.localhost", Port: 10042},
			Public:      ainariv1alpha1.ToriiPublicSpec{OverlayIface: "veth-gw", UplinkIface: "veth-gw", UplinkNextHop: "10.0.0.1"},
			Underlay:    ainariv1alpha1.ToriiUnderlaySpec{Iface: "eth0"},
		},
		Dashboard: ainariv1alpha1.DashboardSpec{Enabled: true, Replicas: 1, Image: "kitsudaiki/ainari_dashboard:develop", Domain: "local-ainari"},
	}
}

// testSecrets returns the secret values, which the renderer writes into the configs.
func testSecrets() *secrets.Values {
	return &secrets.Values{
		MLSGrantPublicKey:    "cOldUtphCTOrQbFpXGDZPIcPfDomKajUzOLVyM8whxo=",
		OmamoriEncryptionKey: "q9vN4CjOQm5wKzyzjZtS7t4oQp8oQK1JvU5xgq8vFzE=",
	}
}

// renderObjects renders a spec and indexes the objects by their kind and name.
func renderObjects(t *testing.T, spec *ainariv1alpha1.AinariSpec) map[string]client.Object {
	t.Helper()
	objects, err := render.Render(render.Input{Namespace: "test", Spec: spec, Secrets: testSecrets()})
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
		"Deployment/omikuji", "TLSRoute/omikuji-route",
		"Deployment/isekai", "TLSRoute/isekai-route", "StatefulSet/onsen-backup",
		"Deployment/torii-public", "Deployment/dashboard",
		"StatefulSet/mysql", "StatefulSet/ryokan", "StatefulSet/onsen", "StatefulSet/sakura",
		"Issuer/ainari-ca-issuer", "Certificate/miko-cert", "TLSRoute/miko-route",
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
	spec.Global.Gateway.Enabled = false
	spec.MySQL.Deploy = false
	spec.MySQL.Host = "mysql.example"
	spec.MySQL.CredentialsSecret = "external-mysql"
	objects := renderObjects(t, spec)

	for key := range objects {
		for _, unwanted := range []string{"dashboard", "TLSRoute/", "StatefulSet/mysql"} {
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

func TestRenderTLSRoute(t *testing.T) {
	objects := renderObjects(t, testSpec())

	route := objects["TLSRoute/miko-route"].(*unstructured.Unstructured).Object
	parentRefs, _, _ := unstructured.NestedSlice(route, "spec", "parentRefs")
	wantParent := map[string]any{"name": "ainari-gateway", "namespace": "ainari-gateway", "sectionName": "tls-passthrough"}
	if !reflect.DeepEqual(parentRefs, []any{wantParent}) {
		t.Errorf("the route has the wrong parent: %v", parentRefs)
	}
	hostnames, _, _ := unstructured.NestedStringSlice(route, "spec", "hostnames")
	if !reflect.DeepEqual(hostnames, []string{"local-miko"}) {
		t.Errorf("the route has the wrong hostnames: %v", hostnames)
	}

	// the tls is passed through, so the backend has to be the service of a nginx-sidecar
	rules, _, _ := unstructured.NestedSlice(route, "spec", "rules")
	backend := rules[0].(map[string]any)["backendRefs"].([]any)[0].(map[string]any)
	service, ok := objects["Service/"+backend["name"].(string)].(*corev1.Service)
	if !ok {
		t.Fatalf("the backend %v of the route is not rendered", backend)
	}
	if port := service.Spec.Ports[0]; int64(port.Port) != backend["port"] || port.TargetPort.IntVal != 9443 {
		t.Errorf("the backend %v doesn't point to the external nginx-sidecar: %+v", backend, port)
	}
}

func TestRenderProxies(t *testing.T) {
	spec := testSpec()
	spec.Torii.ServiceType = corev1.ServiceTypeNodePort
	spec.Global.ExternalServices = ainariv1alpha1.ExternalServicesSpec{
		Enabled: true, Type: ainariv1alpha1.ExternalServiceNodePort, NodePortOffset: 20000,
	}
	objects := renderObjects(t, spec)

	// all proxies share one port
	service := objects["Service/torii-gateway-service"].(*corev1.Service)
	if len(service.Spec.Ports) != 1 {
		t.Fatalf("the proxies have more than one port: %+v", service.Spec.Ports)
	}
	if port := service.Spec.Ports[0]; port.Port != 10042 || port.TargetPort.IntVal != 10042 || port.NodePort != 30042 {
		t.Errorf("the proxies are published on the wrong port: %+v", port)
	}
	config := objects["ConfigMap/torii-config"].(*corev1.ConfigMap).Data["torii-public.toml"]
	for _, want := range []string{`domain = "ainari.localhost"`, "port = 10042"} {
		if !strings.Contains(config, want) {
			t.Errorf("the config of the gateway at the edge has no '%s':\n%s", want, config)
		}
	}
	proxySection := config[strings.Index(config, "[proxy]"):strings.Index(config, "[network]")]
	if strings.Contains(proxySection, "public_port =") {
		t.Errorf("the proxies of the gateway at the edge have a public port of their own:\n%s", config)
	}

	// the clients get the certificate of the external api of the sakura-hosts over the proxies,
	// so it must only contain the names of the proxies and not the ones of the sakura-hosts
	cert := objects["Certificate/sakura-proxy-cert"].(*unstructured.Unstructured).Object
	dnsNames, _, _ := unstructured.NestedStringSlice(cert, "spec", "dnsNames")
	if !reflect.DeepEqual(dnsNames, []string{"*.ainari.localhost"}) {
		t.Errorf("the certificate of the proxies has the wrong names: %v", dnsNames)
	}
	if _, found, _ := unstructured.NestedSlice(cert, "spec", "ipAddresses"); found {
		t.Errorf("the certificate of the proxies has ip-addresses")
	}
	pod := objects["StatefulSet/sakura"].(*appsv1.StatefulSet).Spec.Template.Spec
	secretOf := map[string]string{}
	for _, volume := range pod.Volumes {
		if volume.Secret != nil {
			secretOf[volume.Name] = volume.Secret.SecretName
		}
	}
	for _, container := range pod.Containers {
		if container.Name != "external-tls-termination" {
			continue
		}
		for _, mount := range container.VolumeMounts {
			if mount.MountPath == "/etc/nginx/certs" && secretOf[mount.Name] != "sakura-proxy-tls-secret" {
				t.Errorf("the external api of sakura uses the certificate %s", secretOf[mount.Name])
			}
		}
	}

	// a load-balancer can publish the proxies under another port
	spec.Torii.Proxy.PublicPort = 443
	objects = renderObjects(t, spec)
	service = objects["Service/torii-gateway-service"].(*corev1.Service)
	if port := service.Spec.Ports[0]; port.Port != 443 || port.TargetPort.IntVal != 10042 {
		t.Errorf("the proxies are not published on their public port: %+v", port)
	}
	config = objects["ConfigMap/torii-config"].(*corev1.ConfigMap).Data["torii-public.toml"]
	if !strings.Contains(config, "public_port = 443") {
		t.Errorf("the gateway at the edge doesn't know its public port:\n%s", config)
	}
}

func TestRenderUnderlay(t *testing.T) {
	// toriiContainers returns the containers of the gateway at the edge and in front of the
	// sakura-hosts
	toriiContainers := func(objects map[string]client.Object) []corev1.Container {
		var containers []corev1.Container
		for _, pod := range []corev1.PodSpec{
			objects["Deployment/torii-public"].(*appsv1.Deployment).Spec.Template.Spec,
			objects["StatefulSet/sakura"].(*appsv1.StatefulSet).Spec.Template.Spec,
		} {
			for _, container := range pod.Containers {
				if container.Name == "torii" {
					containers = append(containers, container)
				}
			}
		}
		return containers
	}
	envOf := func(container corev1.Container, name string) (string, bool) {
		for _, env := range container.Env {
			if env.Name == name {
				return env.Value, true
			}
		}
		return "", false
	}

	// by default the underlay is the pod-network and the gateway at the edge is reached over its
	// pod
	spec := testSpec()
	objects := renderObjects(t, spec)
	configs := objects["ConfigMap/torii-config"].(*corev1.ConfigMap).Data
	for _, name := range []string{"torii-public.toml", "torii-vmm.toml"} {
		if !strings.Contains(configs[name], `underlay_iface = "eth0"`) {
			t.Errorf("the underlay of %s is not the pod-network:\n%s", name, configs[name])
		}
	}
	if !strings.Contains(configs["torii-vmm.toml"], `default_gateway_ip = "@DEFAULT_GATEWAY_IP@"`) {
		t.Errorf("the default gateway is not resolved from the pod of the edge:\n%s", configs["torii-vmm.toml"])
	}
	for _, container := range toriiContainers(objects) {
		if _, found := envOf(container, "WAIT_FOR_UNDERLAY"); found {
			t.Errorf("a gateway waits for the pod-network")
		}
	}

	// an underlay of its own is moved into the pods, so the gateways wait for it, and the
	// gateway at the edge is reached over its address there
	spec.Torii.Underlay = ainariv1alpha1.ToriiUnderlaySpec{Iface: "underlay0", WaitForIface: true, EdgeAddress: "192.168.57.11"}
	objects = renderObjects(t, spec)
	configs = objects["ConfigMap/torii-config"].(*corev1.ConfigMap).Data
	for _, name := range []string{"torii-public.toml", "torii-vmm.toml"} {
		if !strings.Contains(configs[name], `underlay_iface = "underlay0"`) {
			t.Errorf("the underlay of %s is not the configured interface:\n%s", name, configs[name])
		}
	}
	if !strings.Contains(configs["torii-vmm.toml"], `default_gateway_ip = "192.168.57.11"`) {
		t.Errorf("the default gateway is not the edge on the underlay:\n%s", configs["torii-vmm.toml"])
	}
	containers := toriiContainers(objects)
	if len(containers) != 2 {
		t.Fatalf("expected the gateway at the edge and one in front of the sakura-hosts, got %d", len(containers))
	}
	for _, container := range containers {
		if value, _ := envOf(container, "WAIT_FOR_UNDERLAY"); value != "underlay0" {
			t.Errorf("a gateway doesn't wait for its underlay: '%s'", value)
		}
		if value, _ := envOf(container, "UNDERLAY_IFACE"); value != "underlay0" {
			t.Errorf("the start-script of a gateway doesn't know its underlay: '%s'", value)
		}
		if _, found := envOf(container, "DEFAULT_GATEWAY_HOST"); found {
			t.Errorf("a gateway resolves the edge over the pod-network")
		}
		if container.StartupProbe == nil {
			t.Errorf("a gateway, which waits for its underlay, has no startup-probe")
		}
	}
}

func TestRenderOnsenAPI(t *testing.T) {
	objects := renderObjects(t, testSpec())

	for _, name := range []string{"onsen", "onsen-backup"} {
		// every pod is reached over its own tls-termination in front of the internal api
		config := objects["ConfigMap/"+name+"-config"].(*corev1.ConfigMap).Data["onsen.toml"]
		for _, wanted := range []string{"internal_port = 10426", "public_port = 11426", `internal_ip = "127.0.0.1"`} {
			if !strings.Contains(config, wanted) {
				t.Errorf("'%s' is missing in the config of %s:\n%s", wanted, name, config)
			}
		}
		nginx := objects["ConfigMap/"+name+"-nginx-config"].(*corev1.ConfigMap).Data["internal.conf"]
		for _, wanted := range []string{"listen 8443 ssl;", "proxy_pass http://127.0.0.1:10426/;", "client_max_body_size 0;", "proxy_buffering off;"} {
			if !strings.Contains(nginx, wanted) {
				t.Errorf("'%s' is missing in the nginx-config of %s:\n%s", wanted, name, nginx)
			}
		}

		template := objects["StatefulSet/"+name].(*appsv1.StatefulSet).Spec.Template
		containers := template.Spec.Containers
		if len(containers) != 2 || containers[1].Name != "tls-termination" {
			t.Errorf("%s has no tls-termination", name)
		}
		if containers[0].ReadinessProbe == nil || containers[0].LivenessProbe == nil {
			t.Errorf("%s has no probes", name)
		}

		ports := objects["Service/"+name].(*corev1.Service).Spec.Ports
		if len(ports) != 1 || ports[0].Port != 8443 {
			t.Errorf("the headless service of %s has the ports %v", name, ports)
		}

		// the certificate is valid for the names of all pods, but has no domain of a TLSRoute
		cert := objects["Certificate/"+name+"-cert"].(*unstructured.Unstructured)
		dnsNames, _, _ := unstructured.NestedStringSlice(cert.Object, "spec", "dnsNames")
		if len(dnsNames) != 1 || dnsNames[0] != "*."+name+".test.svc.cluster.local" {
			t.Errorf("the certificate of %s has the names %v", name, dnsNames)
		}
	}

	// the other components keep the limit of the body
	nginx := objects["ConfigMap/miko-nginx-config"].(*corev1.ConfigMap).Data["internal.conf"]
	if !strings.Contains(nginx, "client_max_body_size 1G;") || strings.Contains(nginx, "proxy_buffering off;") {
		t.Errorf("the nginx-config of miko has the settings for large files:\n%s", nginx)
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

// rustLogOf returns the value of RUST_LOG of the containers of a deployment.
func rustLogOf(t *testing.T, objects map[string]client.Object, key string) string {
	t.Helper()
	for _, container := range objects[key].(*appsv1.Deployment).Spec.Template.Spec.Containers {
		for _, env := range container.Env {
			if env.Name == "RUST_LOG" {
				return env.Value
			}
		}
	}
	t.Fatalf("RUST_LOG is missing in %s", key)
	return ""
}

func TestRenderDebug(t *testing.T) {
	// the debug-logs are disabled by default
	objects := renderObjects(t, testSpec())
	config := objects["ConfigMap/miko-config"].(*corev1.ConfigMap).Data["miko.toml"]
	if !strings.Contains(config, "debug = false") {
		t.Errorf("debug is not disabled in the config of miko:\n%s", config)
	}
	if rustLog := rustLogOf(t, objects, "Deployment/miko"); rustLog != "info" {
		t.Errorf("miko has RUST_LOG '%s' instead of 'info'", rustLog)
	}

	spec := testSpec()
	spec.Global.Debug = true
	objects = renderObjects(t, spec)
	config = objects["ConfigMap/miko-config"].(*corev1.ConfigMap).Data["miko.toml"]
	if !strings.Contains(config, "debug = true") {
		t.Errorf("debug is not enabled in the config of miko:\n%s", config)
	}
	if rustLog := rustLogOf(t, objects, "Deployment/miko"); rustLog != "debug" {
		t.Errorf("miko has RUST_LOG '%s' instead of 'debug'", rustLog)
	}
}

func TestRenderOmikuji(t *testing.T) {
	objects := renderObjects(t, testSpec())

	// miko hands out the address of omikuji, so the other components can send their logs to it
	mikoConfig := objects["ConfigMap/miko-config"].(*corev1.ConfigMap).Data["miko.toml"]
	if !strings.Contains(mikoConfig, "[endpoints.omikuji]") ||
		!strings.Contains(mikoConfig, `internal_address = "https://omikuji-tls-service.test.svc.cluster.local:8443"`) {
		t.Errorf("the endpoint of omikuji is missing in the config of miko:\n%s", mikoConfig)
	}

	// omikuji stores the logs in its own database of the mysql-server
	config := objects["ConfigMap/omikuji-config"].(*corev1.ConfigMap).Data["omikuji.toml"]
	for _, wanted := range []string{`database_type = "mysql"`, "internal_port = 10424", "[mysql]"} {
		if !strings.Contains(config, wanted) {
			t.Errorf("'%s' is missing in the config of omikuji:\n%s", wanted, config)
		}
	}
}

func TestRenderIsekai(t *testing.T) {
	objects := renderObjects(t, testSpec())

	// miko hands out the address of isekai to the clients and the other components
	mikoConfig := objects["ConfigMap/miko-config"].(*corev1.ConfigMap).Data["miko.toml"]
	if !strings.Contains(mikoConfig, "[endpoints.isekai]") ||
		!strings.Contains(mikoConfig, `internal_address = "https://isekai-tls-service.test.svc.cluster.local:8443"`) {
		t.Errorf("the endpoint of isekai is missing in the config of miko:\n%s", mikoConfig)
	}

	// isekai stores its jobs in its own database and uses the internal endpoints of miko
	config := objects["ConfigMap/isekai-config"].(*corev1.ConfigMap).Data["isekai.toml"]
	for _, wanted := range []string{
		`database_type = "mysql"`, "internal_port = 10425", "[mysql]",
		`internal_address = "https://miko-tls-service.test.svc.cluster.local:8443"`,
		"number_of_workers = 4", "max_parallel_snapshots = 2",
		"memory_kib = 65536", "iterations = 3", "parallelism = 4",
	} {
		if !strings.Contains(config, wanted) {
			t.Errorf("'%s' is missing in the config of isekai:\n%s", wanted, config)
		}
	}
	if _, ok := objects["Service/isekai-external"]; !ok {
		t.Errorf("isekai has no external api")
	}

	// the backups are stored by their own onsen with its own volume and address
	backups := objects["StatefulSet/onsen-backup"].(*appsv1.StatefulSet)
	if *backups.Spec.Replicas != 1 || backups.Spec.ServiceName != "onsen-backup" {
		t.Errorf("the onsen of the backups has %d replicas and the service %s",
			*backups.Spec.Replicas, backups.Spec.ServiceName)
	}
	size := backups.Spec.VolumeClaimTemplates[0].Spec.Resources.Requests[corev1.ResourceStorage]
	if size.String() != "10Gi" {
		t.Errorf("the volume of the backups has the size %s instead of 10Gi", size.String())
	}
	if volume := backups.Spec.Template.Spec.Volumes[0]; volume.ConfigMap.Name != "onsen-backup-config" {
		t.Errorf("the onsen of the backups uses the config %s", volume.ConfigMap.Name)
	}
	label := backups.Spec.Template.Spec.Affinity.NodeAffinity.RequiredDuringSchedulingIgnoredDuringExecution.NodeSelectorTerms[0].MatchExpressions[0].Key
	if label != "onsen-backup-node" {
		t.Errorf("the onsen of the backups runs on the nodes with the label %s", label)
	}
	backupConfig := objects["ConfigMap/onsen-backup-config"].(*corev1.ConfigMap).Data["onsen.toml"]
	if !strings.Contains(backupConfig, `address = "https://@HOSTNAME@.onsen-backup.test.svc.cluster.local:8443"`) {
		t.Errorf("the onsen of the backups has the wrong address:\n%s", backupConfig)
	}
	onsenConfig := objects["ConfigMap/onsen-config"].(*corev1.ConfigMap).Data["onsen.toml"]
	if !strings.Contains(onsenConfig, `address = "https://@HOSTNAME@.onsen.test.svc.cluster.local:8443"`) {
		t.Errorf("onsen has the wrong address:\n%s", onsenConfig)
	}
}
