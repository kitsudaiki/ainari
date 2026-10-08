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
)

// Ports, where the nginx-sidecars terminate tls in front of the components.
const (
	// tls-termination of the internal api, which the other components reach
	internalTLSPort = 8443
	// tls-termination of the external api, which the clients reach
	externalTLSPort = 9443
	// tls-termination of every torii, also the one in the pod of a sakura-host, where 8443
	// already belongs to sakura
	toriiTLSPort = 8444
	// port of the wireguard-tunnel
	wireGuardPort = 51820
	// grpc-port of onsen
	onsenGRPCPort = 50051
)

// apiPorts are the ports, where a component itself listens on localhost.
type apiPorts struct {
	Internal int32
	Public   int32
}

// ports of the components by their name
var ports = map[string]apiPorts{
	"ryokan":  {Internal: 10416, Public: 11416},
	"miko":    {Internal: 10417, Public: 11417},
	"hanami":  {Internal: 10418, Public: 11418},
	"torii":   {Internal: 10419, Public: 11419},
	"sakura":  {Internal: 10420, Public: 11420},
	"omamori": {Internal: 10421, Public: 11421},
	"izakaya": {Internal: 10423, Public: 11423},
}

// dashboardPort is the published port of the dashboard.
const dashboardPort = 11422

// portsOf returns the ports of a component and panics for an unknown one, which is a bug of the
// renderer and not of the input.
func portsOf(name string) apiPorts {
	p, ok := ports[name]
	if !ok {
		panic(fmt.Sprintf("unknown component '%s'", name))
	}
	return p
}

// mikoAddress is the address, which the other components use to reach miko.
func (r *renderer) mikoAddress() string {
	return r.internalAddress("miko")
}

// internalAddress is the address of the tls-termination in front of a component, which miko
// hands out to the other components.
func (r *renderer) internalAddress(name string) string {
	return fmt.Sprintf("https://%s.%s.svc.cluster.local:%d", tlsServiceName(name), r.Namespace, internalTLSPort)
}

// publicAddress is the address of a component, which miko hands out to the clients. An
// explicitly configured address wins, otherwise it is the domain of the ingress.
func (r *renderer) publicAddress(name string) string {
	domain, address := r.publicAPI(name)
	if address != "" {
		return address
	}
	return fmt.Sprintf("https://%s:443", domain)
}

// publicAPI returns the domain and the configured public address of a component, which is
// reachable by the clients.
func (r *renderer) publicAPI(name string) (domain, address string) {
	switch name {
	case "miko":
		return r.Spec.Miko.Domain, r.Spec.Miko.PublicAddress
	case "hanami":
		return r.Spec.Hanami.Domain, r.Spec.Hanami.PublicAddress
	case "ryokan":
		return r.Spec.Ryokan.Domain, r.Spec.Ryokan.PublicAddress
	case "omamori":
		return r.Spec.Omamori.Domain, r.Spec.Omamori.PublicAddress
	case "torii":
		return r.Spec.Torii.Domain, r.Spec.Torii.PublicAddress
	default:
		panic(fmt.Sprintf("component '%s' has no public api", name))
	}
}

// tlsServiceName is the name of the service of the internal tls-termination of a component.
func tlsServiceName(name string) string {
	return name + "-tls-service"
}

// externalServiceName is the name of the service of the external api of a component.
func externalServiceName(name string) string {
	return name + "-external"
}

// tlsSecretName is the name of the secret with the certificate of a component.
func tlsSecretName(name string) string {
	return name + "-tls-secret"
}

// nginxConfigName is the name of the configmap with the configs of the nginx-sidecars of a
// component.
func nginxConfigName(name string) string {
	return name + "-nginx-config"
}
