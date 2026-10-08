# Kubernetes-installation

!!! warning

    The installation process is still only for testing, because many important parts are not
    implemented yet.

The whole stack is installed on an existing kubernetes by the operator in `deploy/operator`. It
deploys the stack of a single custom-resource of the kind `Ainari`, generates all keys and
passwords itself and keeps the stack in the state of the resource. All components run with the
pre-built images of [docker-hub](https://hub.docker.com/u/kitsudaiki).

Every image has the tag `develop` for the latest state of the develop-branch and the version of
every release without the leading `v`, for example `0.21.0`. `VERSION` stands for one of these
tags in the following steps.

## Requirements

- **Kubernetes** with `kubectl` access

    The nodes for sakura need `/dev/kvm`. Sakura and torii run as privileged pods, because they
    attach eBPF-programs to their interfaces. The traffic between virtual machines on different
    sakura-nodes is encrypted with IPsec by default, so the kernel of these nodes needs the
    IPsec-support (`xfrm`, `esp4` and `aes-gcm`), which the common distribution-kernels have.

    The volumes use the storage-class `local-path` by default. The sakura-hosts and the gateways
    in front of them keep their databases and the disks of the virtual machines directly on their
    node instead, in a directory with a subdirectory per pod (default `/etc/ainari`). So it needs
    enough space for the disks of the virtual machines.

    !!! example

        For a minimal single-node installation `k3s` without traefik can be used:

        ```bash
        curl -sfL https://get.k3s.io | INSTALL_K3S_EXEC="--disable traefik" sh -

        export KUBECONFIG=/etc/rancher/k3s/k3s.yaml
        ```

- **git**, to get the manifests of the operator

## Installation

1. **Get the repository**

    ```bash
    git clone https://github.com/kitsudaiki/ainari.git
    cd ainari
    ```

1. **Ingress-nginx-controller** (if not already exist in your kubernetes)

    The ingresses use the ingress-class `nginx`. It can be installed for example with its
    helm-chart:

    ```bash
    helm upgrade --install ingress-nginx ingress-nginx \
        --repo https://kubernetes.github.io/ingress-nginx \
        --namespace ingress-nginx --create-namespace
    ```

    Without an ingress-controller the ingresses have to be disabled and the apis published over
    node-ports instead, see *Without ingress-controller* in the step *Ainari-resource*.

1. **Cert-Manager** (if not already exist in your kubernetes)

    Required in any case, because all components talk https to each other with certificates of
    cert-manager.

    ```bash
    kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.18.2/cert-manager.yaml
    kubectl -n cert-manager wait deployment --all --for=condition=Available --timeout=300s
    ```

1. **Node labels**

    Every component is only scheduled on nodes with its label and never twice on the same node:

    ```bash
    kubectl label nodes NODE_NAME miko-node=true
    kubectl label nodes NODE_NAME hanami-node=true
    kubectl label nodes NODE_NAME ryokan-node=true
    kubectl label nodes NODE_NAME omamori-node=true
    kubectl label nodes NODE_NAME izakaya-node=true
    kubectl label nodes NODE_NAME onsen-node=true
    kubectl label nodes NODE_NAME sakura-node=true
    kubectl label nodes NODE_NAME torii-node=true
    kubectl label nodes NODE_NAME ainari-dashboard-node=true
    kubectl label nodes NODE_NAME mysql-node=true
    ```

    On a cluster with only one node, the labels are not needed, if the strict scheduling is
    disabled instead with `global.strictScheduling: false` in the step *Ainari-resource*.

    A sakura-pod keeps its data in the directory of the node, on which it runs. So every pod of
    sakura should always come back on the same node, which is the case, if every node with the
    label `sakura-node` runs exactly one of them.

1. **Install the operator**

    The operator runs in the namespace `ainari-system` with the image
    `kitsudaiki/ainari_operator`. Its manifests use the tag `develop`, which is replaced by the
    version to install:

    ```bash
    kubectl kustomize deploy/operator/config/default \
        | sed 's|kitsudaiki/ainari_operator:develop|kitsudaiki/ainari_operator:VERSION|' \
        | kubectl apply --server-side -f -
    kubectl -n ainari-system rollout status deployment/ainari-operator
    ```

    This installs the custom-resource-definition `ainaris.ainari.kitsunemimi.moe`, the operator
    and its permissions. One operator serves all `Ainari`-resources of the cluster.

1. **Ainari-resource**

    Create a file `my_ainari.yaml` with at least the following content. All other fields and their
    defaults are shown by `kubectl explain ainari.spec --recursive` and described in
    `deploy/operator/api/v1alpha1/ainari_types.go`.

    ```yaml
    apiVersion: ainari.kitsunemimi.moe/v1alpha1
    kind: Ainari
    metadata:
      name: ainari
    spec:
      miko:
        image: kitsudaiki/miko:VERSION
        admin:
          id: "USER_ID"
          name: "USER_NAME"
      hanami:
        image: kitsudaiki/hanami:VERSION
        network:
          floatingIpCidr: "FLOATING_IP_CIDR"
      ryokan:
        image: kitsudaiki/ryokan:VERSION
      omamori:
        image: kitsudaiki/omamori:VERSION
      izakaya:
        image: kitsudaiki/izakaya:VERSION
      onsen:
        image: kitsudaiki/onsen:VERSION
      sakura:
        image: kitsudaiki/sakura:VERSION
        # output of 'stat -c %g /dev/kvm' on the sakura-nodes
        kvmGid: KVM_GID
      torii:
        image: kitsudaiki/torii:VERSION
        public:
          overlayIface: "UPLINK_IFACE"
          uplinkIface: "UPLINK_IFACE"
          uplinkNextHop: "UPLINK_NEXT_HOP"
      dashboard:
        image: kitsudaiki/ainari_dashboard:VERSION
    ```

    - `VERSION`

        - Tag of the images on docker-hub, the same as the one of the operator. Without the field
          `image` a component uses the tag `develop`.

    - `USER_ID`, `USER_NAME`

        - **required**
        - Login of the initial admin-user. `USER_ID` MUST match the regex
          `[a-zA-Z][a-zA-Z_0-9@]*`, `USER_NAME` the regex `[a-zA-Z][a-zA-Z_0-9 ]*`, both with
          between `4` and `256` characters length.
        - The passphrase is generated by the operator, see [Secrets](#secrets).

    - `UPLINK_IFACE`, `UPLINK_NEXT_HOP`, `FLOATING_IP_CIDR`

        - Uplink of the gateway `torii-public`, on which the floating IPs out of
          `FLOATING_IP_CIDR` are served, and the router behind it. The interface has to exist
          within the pod of `torii-public`. If it is moved into the pod after its start, set its
          name additionally as `torii.public.waitForIface`. `testing/kind/setup_kind_stack.sh`
          shows an example, which injects a veth-pair into the pod.

    - Encryption

        - The traffic between the virtual machines of a network is encrypted by default. Without
          encryption set `hanami.network.mlsEncryption: false`. Then izakaya is not deployed.
          Single networks can also be created without encryption with the flag
          `disable_encryption`.

    - Mysql

        - Miko, hanami, ryokan, omamori and izakaya always store their data in a mysql-server,
          each in its own database with its own user. By default the operator deploys the server
          on the node with the label `mysql-node` and creates the databases and users at its
          first start.
        - To use an existing server instead, set `mysql.deploy: false`, its address as
          `mysql.host` (and `mysql.port`) and the name of a secret in the namespace of the
          installation as `mysql.credentialsSecret`. This secret needs the passwords of the
          users, which have to exist already, under the keys `miko_password`, `hanami_password`,
          `ryokan_password`, `omamori_password` and `izakaya_password`. The names of the
          databases and users are the names of the components by default and can be changed with
          `mysql.databases.<component>`. The components create their tables themselves.

    - Replicas

        - Every component has the field `replicas`. Miko, hanami, ryokan, omamori and izakaya can
          run with multiple replicas, one on each node with their label, because they keep their
          state only within their mysql-database. Onsen can run with multiple replicas as well,
          every one with its own volume.

    - Wireguard

        - The connections of ryokan and sakura to onsen run through wireguard. The operator
          creates the keys and configs of all pods itself, also for multiple replicas of onsen,
          ryokan and sakura, and updates the running pods, when the number of replicas changes.

    - Without ingress-controller

        - Set `global.ingress.enabled: false` and `global.externalServices.enabled: true`. Then
          the apis are published on node-ports, which are the ports of the components plus
          `global.externalServices.nodePortOffset` (default `20000`), for example `31417` for
          miko.

    - Domains

        - Every component is reachable over the domain in the field `domain` of the component, by
          default `local-miko`, `local-hanami`, `local-ryokan`, `local-omamori`, `local-torii`
          and `local-ainari` for the dashboard.

1. **Install the stack**

    ```bash
    kubectl create namespace ainari
    kubectl -n ainari apply -f my_ainari.yaml
    kubectl -n ainari wait ainari/ainari --for=condition=Ready --timeout=15m
    ```

    The operator deploys all components into the namespace of the resource, so there can only be
    one `Ainari` per namespace. Its state is shown with:

    ```bash
    kubectl -n ainari get ainari
    kubectl -n ainari get ainari ainari -o jsonpath='{.status.components}'
    ```

## Secrets

The `Ainari`-resource contains no secrets. The operator generates all keys and passwords once
and stores them in secrets of the namespace, which it never changes afterwards, also not after a
restart of the operator or a change of the resource. They have no owner, so they stay, when the
`Ainari` is deleted, and a new one takes them over again.

| Secret                    | Content                                                         |
| ------------------------- | --------------------------------------------------------------- |
| `miko-admin`              | passphrase of the admin-user                                    |
| `token-key`               | key, with which miko signs the tokens of the users              |
| `internal-api-key`        | key, with which the components authenticate each other         |
| `onsen-registration-key`  | key, with which onsen registers at ryokan                       |
| `sakura-registration-key` | key, with which sakura registers at hanami                      |
| `mls-grant-signing-key`   | key, with which hanami signs, which gateway may take part in the key-exchange of which network |
| `omamori-encryption-key`  | key, with which omamori encrypts the stored secrets             |
| `mysql-credentials`       | passwords of the deployed mysql-server                          |
| `wireguard-keys`          | private keys of the pods of the wireguard-tunnel                |

The passphrase of the admin-user is read with:

```bash
kubectl -n ainari get secret miko-admin -o jsonpath='{.data.passphrase}' | base64 -d
```

An existing secret is always taken over as it is. So an own passphrase can be given by creating
the secret before the `Ainari`-resource:

```bash
kubectl -n ainari create secret generic miko-admin --from-literal=passphrase=PASSPHRASE
```

`PASSPHRASE` MUST have between `8` and `4096` characters.

!!! warning

    A deleted secret is generated again with a new value. Everything, which depends on the old
    one, can't be read anymore, for example the secrets stored in omamori or the databases of an
    existing mysql-server. So back up these secrets together with the rest of the installation.

## Using

- check if all pods are running

    ```bash
    kubectl -n ainari get pods
    ```

- get IP-address of the ingresses

    ```bash
    kubectl -n ainari get ingress
    ```

- add the domains with this ip to `/etc/hosts`

    !!! example

        ```
        192.168.178.87  local-miko
        192.168.178.87  local-hanami
        192.168.178.87  local-ryokan
        192.168.178.87  local-omamori
        192.168.178.87  local-torii
        192.168.178.87  local-ainari
        ```

- trust the CA, which signed all certificates. Without an own CA in
    `global.certificates.caSecret`, cert-manager creates it in the secret `ainari-ca`:

    ```bash
    kubectl -n ainari get secret ainari-ca -o jsonpath='{.data.ca\.crt}' | base64 -d > ainari-ca.crt
    sudo cp ainari-ca.crt /usr/local/share/ca-certificates/
    sudo update-ca-certificates
    ```

- use the address of miko for the CLI:

    ```bash
    export AINARI_ADDRESS=https://local-miko
    ```

- the dashboard is available under `https://local-ainari`

## Uninstall

```bash
kubectl -n ainari delete ainari ainari
kubectl delete namespace ainari
```

Deleting the `Ainari` removes all components, but keeps the generated secrets (see
[Secrets](#secrets)), so a new `Ainari` in the same namespace takes them over. They are removed
with the namespace. The operator itself and its custom-resource-definition are removed with:

```bash
kubectl delete -k deploy/operator/config/default
```

This deletes all `Ainari`-resources of the cluster as well.

The sakura-hosts and their gateways keep their data in the directory `sakura.hostDataPath`
(default `/etc/ainari`) on the sakura-nodes, which is not removed with the namespace. To remove
the virtual machines and their disks as well, delete it on every sakura-node:

```bash
sudo rm -rf /etc/ainari
```
