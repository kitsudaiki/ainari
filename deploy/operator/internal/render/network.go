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

	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/apis/meta/v1/unstructured"
	"k8s.io/apimachinery/pkg/util/intstr"
	"sigs.k8s.io/controller-runtime/pkg/client"
)

// servicePort returns a tcp-port of a service.
func servicePort(name string, port, targetPort int32) corev1.ServicePort {
	return corev1.ServicePort{
		Name:       name,
		Protocol:   corev1.ProtocolTCP,
		Port:       port,
		TargetPort: intstr.FromInt32(targetPort),
	}
}

// tlsService returns the service of the internal tls-termination of a component, which the other
// components reach.
func tlsService(name, app string, targetPort int32) *corev1.Service {
	return &corev1.Service{
		ObjectMeta: metav1.ObjectMeta{
			Name:   tlsServiceName(name),
			Labels: map[string]string{"run": app},
		},
		Spec: corev1.ServiceSpec{
			Type:     corev1.ServiceTypeClusterIP,
			Selector: appLabels(app),
			Ports:    []corev1.ServicePort{servicePort("input", internalTLSPort, targetPort)},
		},
	}
}

// headlessService returns a headless service, which resolves to the addresses of the pods, also
// of the ones, which are not ready yet.
func headlessService(name, app string, ports ...corev1.ServicePort) *corev1.Service {
	return &corev1.Service{
		ObjectMeta: metav1.ObjectMeta{Name: name, Labels: appLabels(app)},
		Spec: corev1.ServiceSpec{
			ClusterIP:                corev1.ClusterIPNone,
			PublishNotReadyAddresses: true,
			Selector:                 appLabels(app),
			Ports:                    ports,
		},
	}
}

// externalService returns the service of the external api of a component, which publishes the
// given port, forwarded to the targetPort of the pod. It always exists, because the TLSRoute
// forwards to it as well, but it only publishes the port outside of the cluster, if
// global.externalServices is enabled, otherwise it has the type ClusterIP. With the type
// NodePort, the node-port is the port itself plus a fixed offset, so the kind-setup can map it
// back to the original port on the host. With the type LoadBalancer, the port itself is
// published, for example by the servicelb of k3s on every node.
func (r *renderer) externalService(app string, port, targetPort int32) *corev1.Service {
	external := r.Spec.Global.ExternalServices
	serviceType := corev1.ServiceTypeClusterIP
	if external.Enabled {
		serviceType = corev1.ServiceType(external.Type)
	}

	servicePort := servicePort(fmt.Sprintf("p-%d", port), port, targetPort)
	if serviceType == corev1.ServiceTypeNodePort {
		servicePort.NodePort = port + external.NodePortOffset
	}

	return &corev1.Service{
		ObjectMeta: metav1.ObjectMeta{Name: externalServiceName(app), Labels: appLabels(app)},
		Spec: corev1.ServiceSpec{
			Type:     serviceType,
			Selector: appLabels(app),
			Ports:    []corev1.ServicePort{servicePort},
		},
	}
}

// tlsRoute returns the TLSRoute of a component, which attaches the domain to the gateway and passes
// the tls-connections through to the given service, so the tls is terminated by the nginx-sidecar
// of the component and not by the gateway. It returns nothing, if the gateway is disabled. The
// gateway-api is not imported as a library, so the route is created as unstructured object.
func (r *renderer) tlsRoute(name, domain, service string, port int32) []client.Object {
	gateway := r.Spec.Global.Gateway
	if !gateway.Enabled {
		return nil
	}

	parentRef := map[string]any{"name": gateway.Name}
	if gateway.Namespace != "" {
		parentRef["namespace"] = gateway.Namespace
	}
	if gateway.Listener != "" {
		parentRef["sectionName"] = gateway.Listener
	}

	route := &unstructured.Unstructured{}
	route.SetAPIVersion("gateway.networking.k8s.io/v1")
	route.SetKind("TLSRoute")
	route.SetName(name + "-route")
	route.Object["spec"] = map[string]any{
		"parentRefs": []any{parentRef},
		"hostnames":  []any{domain},
		"rules": []any{map[string]any{
			"backendRefs": []any{map[string]any{"name": service, "port": int64(port)}},
		}},
	}
	return []client.Object{route}
}
