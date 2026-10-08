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

package v1alpha1

import (
	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/resource"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

// AinariSpec describes a complete ainari-stack. The operator deploys all components of it into
// the namespace of the resource, so there can only be one of them per namespace. The spec
// contains no secrets: the operator generates all keys and passwords once and stores them in
// secrets of the namespace, which it never changes afterwards.
// +kubebuilder:validation:XValidation:rule="self.mysql.deploy || (has(self.mysql.host) && size(self.mysql.host) > 0)",message="mysql.host is required, if mysql.deploy is disabled"
// +kubebuilder:validation:XValidation:rule="self.mysql.deploy || (has(self.mysql.credentialsSecret) && size(self.mysql.credentialsSecret) > 0)",message="mysql.credentialsSecret is required, if mysql.deploy is disabled"
type AinariSpec struct {
	// settings, which apply to all components
	// +kubebuilder:default={}
	// +optional
	Global GlobalSpec `json:"global"`

	// Database of miko, hanami, ryokan, omamori and izakaya. Each of them has its own database and
	// user on the same mysql-server, so all their replicas share their state.
	// +kubebuilder:default={}
	// +optional
	MySQL MySQLSpec `json:"mysql"`

	// +required
	Miko MikoSpec `json:"miko"`

	// +kubebuilder:default={}
	// +optional
	Hanami HanamiSpec `json:"hanami"`

	// +kubebuilder:default={}
	// +optional
	Sakura SakuraSpec `json:"sakura"`

	// +kubebuilder:default={}
	// +optional
	Ryokan RyokanSpec `json:"ryokan"`

	// +kubebuilder:default={}
	// +optional
	Omamori OmamoriSpec `json:"omamori"`

	// Shares the MLS key-packages of the gateways, delivers their MLS-messages and coordinates the
	// changes and key-rotations of the groups. It is only deployed, if
	// hanami.network.mlsEncryption is enabled.
	// +kubebuilder:default={}
	// +optional
	Izakaya IzakayaSpec `json:"izakaya"`

	// +kubebuilder:default={}
	// +optional
	Onsen OnsenSpec `json:"onsen"`

	// +kubebuilder:default={}
	// +optional
	Torii ToriiSpec `json:"torii"`

	// +kubebuilder:default={}
	// +optional
	Dashboard DashboardSpec `json:"dashboard"`
}

// GlobalSpec contains the settings, which apply to all components.
type GlobalSpec struct {
	// Every component is reachable over an ingress of ingress-nginx. The components always talk
	// https to each other over a nginx-sidecar with a certificate of cert-manager, so
	// cert-manager is always required.
	// +kubebuilder:default={}
	// +optional
	Ingress IngressSpec `json:"ingress"`

	// All certificates are signed by one CA, so a client only has to trust this CA.
	// +kubebuilder:default={}
	// +optional
	Certificates CertificatesSpec `json:"certificates"`

	// The connections of ryokan and sakura to onsen run through wireguard. Every pod of onsen,
	// ryokan and sakura gets its own key and address within 10.10.0.0/16 and every onsen is
	// connected with every ryokan and every sakura. The operator creates the keys and configs and
	// updates the peers of the running pods, when the number of replicas changes.
	// +kubebuilder:default={}
	// +optional
	WireGuard WireGuardSpec `json:"wireguard"`

	// Every component runs only on the nodes with its label (for example 'hanami-node=true') and
	// never twice on the same node. A cluster with only one node needs this to be disabled.
	// +kubebuilder:default=true
	// +optional
	StrictScheduling bool `json:"strictScheduling"`

	// overwrites the storage-class of all components, if set
	// +optional
	StorageClass string `json:"storageClass,omitempty"`

	// overwrites the pull-policy of all components, if set
	// +optional
	ImagePullPolicy corev1.PullPolicy `json:"imagePullPolicy,omitempty"`

	// image of the nginx-sidecars, which terminate tls in front of the components
	// +kubebuilder:default="nginx:latest"
	// +optional
	NginxImage string `json:"nginxImage"`

	// Publishes the api of every component and the dashboard outside of the cluster, beside the
	// ingresses.
	// +kubebuilder:default={}
	// +optional
	ExternalServices ExternalServicesSpec `json:"externalServices"`
}

// IngressSpec configures the ingresses of the components.
type IngressSpec struct {
	// +kubebuilder:default=true
	// +optional
	Enabled bool `json:"enabled"`

	// ingress-class of ingress-nginx, which has to support ssl-passthrough
	// +kubebuilder:default="nginx"
	// +optional
	ClassName string `json:"className"`
}

// CertificatesSpec configures the CA and the certificates of the components.
type CertificatesSpec struct {
	// Name of a secret of the type kubernetes.io/tls with the certificate and key of an own CA.
	// If empty, cert-manager creates a CA in the secret 'ainari-ca'.
	// +optional
	CASecret string `json:"caSecret,omitempty"`

	// additional names, which every certificate is valid for
	// +optional
	ExtraDNSNames []string `json:"extraDnsNames,omitempty"`

	// additional addresses, which every certificate is valid for
	// +optional
	ExtraIPAddresses []string `json:"extraIpAddresses,omitempty"`
}

// WireGuardSpec configures the tunnel between ryokan, onsen and sakura.
type WireGuardSpec struct {
	// +kubebuilder:default=true
	// +optional
	Enabled bool `json:"enabled"`
}

// ExternalServiceType is the type of the services, which publish the apis outside of the
// cluster.
// +kubebuilder:validation:Enum=NodePort;LoadBalancer
type ExternalServiceType string

const (
	ExternalServiceNodePort     ExternalServiceType = "NodePort"
	ExternalServiceLoadBalancer ExternalServiceType = "LoadBalancer"
)

// ExternalServicesSpec configures the services, which publish the apis outside of the cluster.
type ExternalServicesSpec struct {
	// +kubebuilder:default=false
	// +optional
	Enabled bool `json:"enabled"`

	// NodePort publishes the api on the node-port, which is the port of the component plus
	// nodePortOffset, for example 11417 + 20000 = 31417 for miko. LoadBalancer publishes it on
	// the port of the component itself, for example on the nodes with the servicelb of k3s.
	// +kubebuilder:default=NodePort
	// +optional
	Type ExternalServiceType `json:"type"`

	// +kubebuilder:default=20000
	// +optional
	// +kubebuilder:validation:Minimum=0
	NodePortOffset int32 `json:"nodePortOffset"`
}

// MySQLSpec configures the mysql-server of the components.
type MySQLSpec struct {
	// Deploys a mysql-server into the cluster, which runs on the node with the label
	// 'mysql-node=true' (see global.strictScheduling). If disabled, an already existing server
	// is used, which is reached at 'host' and already has the databases and users below.
	// +kubebuilder:default=true
	// +optional
	Deploy bool `json:"deploy"`

	// address of the external server, only used if 'deploy' is disabled
	// +optional
	Host string `json:"host,omitempty"`

	// +kubebuilder:default=3306
	// +optional
	// +kubebuilder:validation:Minimum=1
	// +kubebuilder:validation:Maximum=65535
	Port int32 `json:"port"`

	// +kubebuilder:default="mysql:8.4"
	// +optional
	Image string `json:"image"`

	// +kubebuilder:default=IfNotPresent
	// +optional
	ImagePullPolicy corev1.PullPolicy `json:"imagePullPolicy"`

	// storage-class of the volume, overwritten by global.storageClass
	// +kubebuilder:default="local-path"
	// +optional
	StorageClass string `json:"storageClass"`

	// size of the volume
	// +kubebuilder:default="5Gi"
	// +optional
	StorageSize resource.Quantity `json:"storageSize"`

	// Name of an existing secret with the passwords of the external server, only used if
	// 'deploy' is disabled. It needs the keys 'miko_password', 'hanami_password',
	// 'ryokan_password', 'omamori_password' and 'izakaya_password'. The passwords of a deployed
	// server are generated by the operator.
	// +optional
	CredentialsSecret string `json:"credentialsSecret,omitempty"`

	// database and user of every component
	// +kubebuilder:default={}
	// +optional
	Databases MySQLDatabasesSpec `json:"databases"`
}

// MySQLDatabasesSpec contains the database of every component.
type MySQLDatabasesSpec struct {
	// +kubebuilder:default={}
	// +optional
	Miko MySQLDatabaseSpec `json:"miko"`

	// +kubebuilder:default={}
	// +optional
	Hanami MySQLDatabaseSpec `json:"hanami"`

	// +kubebuilder:default={}
	// +optional
	Ryokan MySQLDatabaseSpec `json:"ryokan"`

	// +kubebuilder:default={}
	// +optional
	Omamori MySQLDatabaseSpec `json:"omamori"`

	// +kubebuilder:default={}
	// +optional
	Izakaya MySQLDatabaseSpec `json:"izakaya"`
}

// MySQLDatabaseSpec is the database and user of a component. The names are used within the
// init-script of the server, so they are restricted to letters, digits and underscores.
type MySQLDatabaseSpec struct {
	// name of the database, the name of the component, if empty
	// +kubebuilder:validation:Pattern=`^[a-zA-Z0-9_]+$`
	// +optional
	Database string `json:"database,omitempty"`

	// name of the user, the name of the component, if empty
	// +kubebuilder:validation:Pattern=`^[a-zA-Z0-9_]+$`
	// +optional
	User string `json:"user,omitempty"`
}

// MikoSpec configures miko, which handles the users, their tokens and the endpoints of the other
// components.
type MikoSpec struct {
	// +kubebuilder:default=1
	// +optional
	// +kubebuilder:validation:Minimum=0
	Replicas int32 `json:"replicas"`

	// +kubebuilder:default="kitsudaiki/miko:develop"
	// +optional
	Image string `json:"image"`

	// +kubebuilder:default=Always
	// +optional
	ImagePullPolicy corev1.PullPolicy `json:"imagePullPolicy"`

	// domain of the ingress
	// +kubebuilder:default="local-miko"
	// +optional
	Domain string `json:"domain"`

	// address, which miko hands out to the clients. Derived from the domain, if empty.
	// +optional
	PublicAddress string `json:"publicAddress,omitempty"`

	// admin-user, which miko creates at its first start
	// +required
	Admin MikoAdminSpec `json:"admin"`

	// +kubebuilder:default={}
	// +optional
	Token MikoTokenSpec `json:"token"`
}

// MikoAdminSpec is the admin-user, which miko creates at its first start.
type MikoAdminSpec struct {
	// +kubebuilder:validation:MinLength=1
	ID string `json:"id"`

	// +kubebuilder:validation:MinLength=1
	Name string `json:"name"`
}

// MikoTokenSpec configures the tokens, which miko hands out.
type MikoTokenSpec struct {
	// seconds, until a token expires
	// +kubebuilder:default=3600
	// +optional
	// +kubebuilder:validation:Minimum=1
	ExpireTime int64 `json:"expireTime"`
}

// HanamiSpec configures hanami, which handles the virtual machines and their networks.
type HanamiSpec struct {
	// +kubebuilder:default=1
	// +optional
	// +kubebuilder:validation:Minimum=0
	Replicas int32 `json:"replicas"`

	// +kubebuilder:default="kitsudaiki/hanami:develop"
	// +optional
	Image string `json:"image"`

	// +kubebuilder:default=Always
	// +optional
	ImagePullPolicy corev1.PullPolicy `json:"imagePullPolicy"`

	// domain of the ingress
	// +kubebuilder:default="local-hanami"
	// +optional
	Domain string `json:"domain"`

	// address, which miko hands out to the clients. Derived from the domain, if empty.
	// +optional
	PublicAddress string `json:"publicAddress,omitempty"`

	// +kubebuilder:default={}
	// +optional
	Network HanamiNetworkSpec `json:"network"`
}

// HanamiNetworkSpec configures the networks of the virtual machines.
type HanamiNetworkSpec struct {
	// range of the floating ip-addresses, which are served by the torii at the edge
	// +kubebuilder:default="10.0.0.0/24"
	// +optional
	FloatingIPCIDR string `json:"floatingIpCidr"`

	// Encrypts the traffic between the virtual machines of a network on different hosts with
	// IPsec, whose keys the gateways exchange over MLS-groups, which izakaya coordinates.
	// +kubebuilder:default=true
	// +optional
	MLSEncryption bool `json:"mlsEncryption"`

	// seconds, for which a membership-grant allows a gateway to join the group of a network
	// +kubebuilder:default=86400
	// +optional
	// +kubebuilder:validation:Minimum=1
	MLSGrantValidity int64 `json:"mlsGrantValidity"`

	// seconds between two refreshes of all membership-grants
	// +kubebuilder:default=300
	// +optional
	// +kubebuilder:validation:Minimum=1
	MLSGrantRefreshInterval int64 `json:"mlsGrantRefreshInterval"`
}

// SakuraSpec configures the sakura-hosts, which run the virtual machines.
type SakuraSpec struct {
	// +kubebuilder:default=1
	// +optional
	// +kubebuilder:validation:Minimum=0
	Replicas int32 `json:"replicas"`

	// +kubebuilder:default="kitsudaiki/sakura:develop"
	// +optional
	Image string `json:"image"`

	// +kubebuilder:default=Always
	// +optional
	ImagePullPolicy corev1.PullPolicy `json:"imagePullPolicy"`

	// domain, which the certificate of sakura is valid for, if the ingresses are enabled
	// +kubebuilder:default="local-sakura"
	// +optional
	Domain string `json:"domain"`

	// maximum number of threads of the processing, 0 for no limit
	// +kubebuilder:default=0
	// +optional
	// +kubebuilder:validation:Minimum=0
	MaxNumberOfThreads int32 `json:"maxNumberOfThreads"`

	// Id of the group, which owns /dev/kvm on the nodes. Sakura runs as a normal user and needs
	// it to open the virtual machines.
	// +kubebuilder:default=993
	// +optional
	KVMGID int64 `json:"kvmGid"`

	// Directory on the node, which holds the data of the sakura-host and of the gateway in front
	// of it: their databases and the disks of the virtual machines. Every pod gets its own
	// subdirectory '<pod-name>/sakura' and '<pod-name>/torii', so the data stays on the node and
	// survives a restart of the pod.
	// +kubebuilder:default="/etc/ainari"
	// +optional
	HostDataPath string `json:"hostDataPath"`
}

// RyokanSpec configures ryokan, which handles the images of the virtual machines.
type RyokanSpec struct {
	// +kubebuilder:default=1
	// +optional
	// +kubebuilder:validation:Minimum=0
	Replicas int32 `json:"replicas"`

	// +kubebuilder:default="kitsudaiki/ryokan:develop"
	// +optional
	Image string `json:"image"`

	// +kubebuilder:default=Always
	// +optional
	ImagePullPolicy corev1.PullPolicy `json:"imagePullPolicy"`

	// domain of the ingress
	// +kubebuilder:default="local-ryokan"
	// +optional
	Domain string `json:"domain"`

	// address, which miko hands out to the clients. Derived from the domain, if empty.
	// +optional
	PublicAddress string `json:"publicAddress,omitempty"`
}

// OmamoriSpec configures omamori, which stores the secrets of the users.
type OmamoriSpec struct {
	// +kubebuilder:default=1
	// +optional
	// +kubebuilder:validation:Minimum=0
	Replicas int32 `json:"replicas"`

	// +kubebuilder:default="kitsudaiki/omamori:develop"
	// +optional
	Image string `json:"image"`

	// +kubebuilder:default=Always
	// +optional
	ImagePullPolicy corev1.PullPolicy `json:"imagePullPolicy"`

	// domain of the ingress
	// +kubebuilder:default="local-omamori"
	// +optional
	Domain string `json:"domain"`

	// address, which miko hands out to the clients. Derived from the domain, if empty.
	// +optional
	PublicAddress string `json:"publicAddress,omitempty"`
}

// IzakayaSpec configures izakaya, which coordinates the MLS-groups of the gateways. It keeps its
// state in the mysql-server, so it can run with several replicas. Only the other components talk
// to it, so it has no external api.
type IzakayaSpec struct {
	// +kubebuilder:default=1
	// +optional
	// +kubebuilder:validation:Minimum=0
	Replicas int32 `json:"replicas"`

	// +kubebuilder:default="kitsudaiki/izakaya:develop"
	// +optional
	Image string `json:"image"`

	// +kubebuilder:default=Always
	// +optional
	ImagePullPolicy corev1.PullPolicy `json:"imagePullPolicy"`

	// domain, which the certificate of izakaya is valid for, if the ingresses are enabled
	// +kubebuilder:default="local-izakaya"
	// +optional
	Domain string `json:"domain"`

	// seconds between two key-rotations of a group, whose membership didn't change
	// +kubebuilder:default=3600
	// +optional
	// +kubebuilder:validation:Minimum=1
	KeyRotationInterval int64 `json:"keyRotationInterval"`

	// seconds without contact, after which a gateway is removed from its groups, for example the
	// gateway of a sakura-host, which came back in a new pod with a new address
	// +kubebuilder:default=120
	// +optional
	// +kubebuilder:validation:Minimum=1
	MemberTimeout int64 `json:"memberTimeout"`
}

// OnsenSpec configures onsen, which stores the images of ryokan.
type OnsenSpec struct {
	// +kubebuilder:default=1
	// +optional
	// +kubebuilder:validation:Minimum=0
	Replicas int32 `json:"replicas"`

	// +kubebuilder:default="kitsudaiki/onsen:develop"
	// +optional
	Image string `json:"image"`

	// +kubebuilder:default=Always
	// +optional
	ImagePullPolicy corev1.PullPolicy `json:"imagePullPolicy"`

	// storage-class of the volume, overwritten by global.storageClass
	// +kubebuilder:default="local-path"
	// +optional
	StorageClass string `json:"storageClass"`

	// size of the volume
	// +kubebuilder:default="1Gi"
	// +optional
	StorageSize resource.Quantity `json:"storageSize"`
}

// ToriiSpec configures the gateway at the edge of the network and the gateways in front of the
// sakura-hosts.
type ToriiSpec struct {
	// replicas of the gateway at the edge
	// +kubebuilder:default=1
	// +optional
	// +kubebuilder:validation:Minimum=0
	Replicas int32 `json:"replicas"`

	// +kubebuilder:default="kitsudaiki/torii:develop"
	// +optional
	Image string `json:"image"`

	// +kubebuilder:default=Always
	// +optional
	ImagePullPolicy corev1.PullPolicy `json:"imagePullPolicy"`

	// storage-class of the volume, overwritten by global.storageClass
	// +kubebuilder:default="local-path"
	// +optional
	StorageClass string `json:"storageClass"`

	// size of the volume
	// +kubebuilder:default="100Mi"
	// +optional
	StorageSize resource.Quantity `json:"storageSize"`

	// domain of the ingress
	// +kubebuilder:default="local-torii"
	// +optional
	Domain string `json:"domain"`

	// address, which miko hands out to the clients. Derived from the domain, if empty.
	// +optional
	PublicAddress string `json:"publicAddress,omitempty"`

	// type of the service, which publishes the proxy-ports of the gateway at the edge
	// +kubebuilder:default=LoadBalancer
	// +optional
	// +kubebuilder:validation:Enum=ClusterIP;NodePort;LoadBalancer
	ServiceType corev1.ServiceType `json:"serviceType"`

	// proxy-ports of the gateway at the edge
	// +kubebuilder:default={start: 10040, end: 10050}
	// +optional
	PortRange PortRangeSpec `json:"portRange"`

	// +kubebuilder:default={}
	// +optional
	Public ToriiPublicSpec `json:"public"`
}

// PortRangeSpec is a range of ports, including both ends.
// +kubebuilder:validation:XValidation:rule="self.start <= self.end",message="the start of the port-range has to be lower or equal to its end"
type PortRangeSpec struct {
	// +kubebuilder:validation:Minimum=1
	// +kubebuilder:validation:Maximum=65535
	Start int32 `json:"start"`

	// +kubebuilder:validation:Minimum=1
	// +kubebuilder:validation:Maximum=65535
	End int32 `json:"end"`
}

// ToriiPublicSpec configures the gateway at the edge of the network, which serves the floating
// ip-addresses on its uplink.
type ToriiPublicSpec struct {
	// +kubebuilder:default="veth-gw"
	// +optional
	OverlayIface string `json:"overlayIface"`

	// +kubebuilder:default="veth-gw"
	// +optional
	UplinkIface string `json:"uplinkIface"`

	// address of the next hop behind the uplink
	// +kubebuilder:default="10.0.0.1"
	// +optional
	UplinkNextHop string `json:"uplinkNextHop"`

	// Name of an interface, which is moved into the pod from the outside (for example by
	// testing/kind/setup_kind_stack.sh). The gateway waits for it, before it starts. Empty, if
	// the uplink is already there.
	// +optional
	WaitForIface string `json:"waitForIface,omitempty"`
}

// DashboardSpec configures the web-dashboard.
type DashboardSpec struct {
	// +kubebuilder:default=true
	// +optional
	Enabled bool `json:"enabled"`

	// +kubebuilder:default=1
	// +optional
	// +kubebuilder:validation:Minimum=0
	Replicas int32 `json:"replicas"`

	// +kubebuilder:default="kitsudaiki/ainari_dashboard:develop"
	// +optional
	Image string `json:"image"`

	// +kubebuilder:default=Always
	// +optional
	ImagePullPolicy corev1.PullPolicy `json:"imagePullPolicy"`

	// domain of the ingress
	// +kubebuilder:default="local-ainari"
	// +optional
	Domain string `json:"domain"`
}

// ComponentStatus is the state of the workload of a component.
type ComponentStatus struct {
	Name string `json:"name"`

	// replicas, which should run
	Replicas int32 `json:"replicas"`

	// replicas, which are ready
	ReadyReplicas int32 `json:"readyReplicas"`
}

// AinariStatus is the observed state of an ainari-stack.
type AinariStatus struct {
	// generation of the spec, which was deployed last
	// +optional
	ObservedGeneration int64 `json:"observedGeneration,omitempty"`

	// +optional
	// +listType=map
	// +listMapKey=type
	Conditions []metav1.Condition `json:"conditions,omitempty"`

	// +optional
	// +listType=map
	// +listMapKey=name
	Components []ComponentStatus `json:"components,omitempty"`

	// base64-encoded public key, which belongs to the key in the secret
	// 'mls-grant-signing-key', with which hanami signs the membership-grants of the MLS-groups
	// +optional
	MLSGrantPublicKey string `json:"mlsGrantPublicKey,omitempty"`
}

// Condition-types of an Ainari-resource.
const (
	// ConditionReady is true, if all objects are deployed and all workloads are ready.
	ConditionReady = "Ready"
)

// Ainari is a complete ainari-stack.
// +kubebuilder:object:root=true
// +kubebuilder:subresource:status
// +kubebuilder:printcolumn:name="Ready",type=string,JSONPath=`.status.conditions[?(@.type=="Ready")].status`
// +kubebuilder:printcolumn:name="Reason",type=string,JSONPath=`.status.conditions[?(@.type=="Ready")].reason`
// +kubebuilder:printcolumn:name="Age",type=date,JSONPath=`.metadata.creationTimestamp`
type Ainari struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`

	Spec   AinariSpec   `json:"spec,omitempty"`
	Status AinariStatus `json:"status,omitempty"`
}

// AinariList is a list of Ainari-resources.
// +kubebuilder:object:root=true
type AinariList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`
	Items           []Ainari `json:"items"`
}

func init() {
	SchemeBuilder.Register(&Ainari{}, &AinariList{})
}
