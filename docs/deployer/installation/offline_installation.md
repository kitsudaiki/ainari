# Offline-installation

!!! warning

    The installation process is still only for testing, because many important parts are not
    implemented yet.

The whole stack can be installed on a kubernetes without access to the internet (air-gapped) with
[Zarf](https://zarf.dev). On a machine with access to the internet, an offline-installation is
created, which contains everything: the operator, all images of the components, cert-manager and
Envoy Gateway. It is moved into the air-gapped environment and deployed there.

Zarf first initializes the cluster with its init-package, which deploys a registry within the
cluster and the zarf-agent. All images of the package are pushed into this registry and the
zarf-agent rewrites the images of every new pod towards it. This also applies to the pods, which
the operator creates later, so the operator itself doesn't know about zarf.

The package is defined in `deploy/zarf/zarf.yaml` and contains the following components:

| Component         | Default  | Content                                                                         |
| ----------------- | -------- | ------------------------------------------------------------------------------- |
| `cert-manager`    | yes      | cert-manager, which issues the certificates of all components                   |
| `envoy-gateway`   | yes      | Envoy Gateway with the CRDs of the gateway-api                                  |
| `gateway`         | yes      | the gateway `ainari-gateway` of `deploy/k8s/gateway.yaml`                       |
| `ainari-operator` | required | the CRD and the operator with the images of all components, mysql and nginx    |
| `ainari`          | yes      | the `Ainari`-resource of `deploy/zarf/ainari.yaml` in the namespace `ainari`   |

Components, which already exist in the cluster, like cert-manager, can be skipped on the deployment.

## Create the offline-installation

This step needs access to the internet, `docker` and `curl`.

```bash
git clone https://github.com/kitsudaiki/ainari.git
cd ainari
./scripts/create_zarf_package.sh --version VERSION
```

`VERSION` is the tag of the images on docker-hub, for example `0.21.0` or `develop` (default). The
script has the following further options:

- `--arch arm64`

    - Creates the installation for arm64 instead of amd64.

- `--local`

    - Builds the images of the working tree with `testing/kind/build_local_images.sh` and the
      operator, and uses them instead of the ones of docker-hub, with the version `local`.
      `--no-build` uses the already existing local images.

- `--output DIR`

    - Output directory instead of `temporary_files/zarf`.

The output directory contains everything, which is needed in the air-gapped environment:

| File                                   | Content                                                     |
| -------------------------------------- | ----------------------------------------------------------- |
| `zarf`                                 | binary of zarf for the target-architecture                  |
| `zarf-init-ARCH-ZARF_VERSION.tar.zst`  | init-package of zarf with its registry and the zarf-agent   |
| `zarf-package-ainari-ARCH-VERSION.tar.zst` | the package of ainari                                   |

The whole directory has a size of about 1.4 GiB. Copy it into the air-gapped environment, for
example on a usb-stick, to a machine with `kubectl`-access to the cluster.

## Requirements in the air-gapped environment

- **Kubernetes** with `kubectl` access and a default storage-class, on which the registry of zarf
  stores the images

    The requirements of the nodes, like `/dev/kvm` for the sakura-nodes, are the same as in
    the [Kubernetes-installation](kubernetes_installation.md#requirements).

    !!! example

        If there is no kubernetes yet, the init-package of zarf can install a single-node `k3s`
        on the current machine. This has to run as root and replaces the step *Initialize the
        cluster* below:

        ```bash
        sudo su
        ./zarf init zarf-init-amd64-*.tar.zst --components k3s --confirm

        export KUBECONFIG=/root/.kube/config
        ```

- **Node labels**

    The same as in the [Kubernetes-installation](kubernetes_installation.md#installation). On a
    cluster with only one node, the strict scheduling can be disabled instead with
    `STRICT_SCHEDULING=false` (see below).

## Installation

All commands run within the copied directory.

1. **Initialize the cluster**

    Deploys the registry and the zarf-agent of zarf. This is only needed once per cluster. Zarf
    starts its registry with an image, which already exists on a node, so the cluster needs at
    least one running pod, for example of `kube-system`, which is the case shortly after its
    creation.

    ```bash
    ./zarf init zarf-init-amd64-*.tar.zst --confirm
    ```

1. **Deploy the package**

    ```bash
    ./zarf package deploy zarf-package-ainari-amd64-*.tar.zst --confirm \
        --set-variables ADMIN_ID=USER_ID,ADMIN_NAME=USER_NAME \
        --set-variables FLOATING_IP_CIDR=FLOATING_IP_CIDR,KVM_GID=KVM_GID \
        --set-variables UPLINK_IFACE=UPLINK_IFACE,UPLINK_NEXT_HOP=UPLINK_NEXT_HOP
    ```

    Without `--confirm`, zarf shows the content of the package and asks for the values of all
    variables. The variables are the values of the `Ainari`-resource, the ones in capital letters
    are described in the step *Ainari-resource* of the
    [Kubernetes-installation](kubernetes_installation.md#installation):

    | Variable            | Default   | Field of the `Ainari`-resource                         |
    | ------------------- | --------- | ------------------------------------------------------ |
    | `ADMIN_ID`          | `admin`   | `miko.admin.id`                                        |
    | `ADMIN_NAME`        | `admin`   | `miko.admin.name`                                      |
    | `FLOATING_IP_CIDR`  |           | `hanami.network.floatingIpCidr`                        |
    | `UPLINK_IFACE`      | `veth-gw` | `torii.public.uplinkIface` and `overlayIface`          |
    | `UPLINK_NEXT_HOP`   |           | `torii.public.uplinkNextHop`                           |
    | `KVM_GID`           | `993`     | `sakura.kvmGid`                                        |
    | `STRICT_SCHEDULING` | `true`    | `global.strictScheduling`                              |
    | `GATEWAY_ENABLED`   | `true`    | `global.gateway.enabled`                               |
    | `STORAGE_CLASS`     |           | `global.storageClass`, `local-path`, if empty          |

    Components, which already exist in the cluster, are deselected with a leading `-` in
    `--components`, for example without cert-manager:

    ```bash
    --components=-cert-manager
    ```

    The required component `ainari-operator` is always deployed.

1. **Check the state**

    ```bash
    kubectl -n ainari get ainari
    kubectl -n ainari get pods
    ```

    The [secrets](kubernetes_installation.md#secrets), like the passphrase of the admin-user, and
    the [usage](kubernetes_installation.md#using) are the same as in the Kubernetes-installation.

### Own Ainari-resource

For fields, which are not covered by the variables, the component `ainari` can be skipped with
`--components=-ainari` and an own `Ainari`-resource created instead, see the step
*Ainari-resource* of the [Kubernetes-installation](kubernetes_installation.md#installation). It
has to set the field `image` of every component to the tag of the package (`VERSION`), because
only these images are within the registry of zarf. Without the field, a component uses the tag
`develop`, which is only within a package of the version `develop`.

The pods pull the images from the registry of zarf with the secret `private-registry`, which zarf
only creates in the namespaces of its packages. So it has to be copied into the namespace of an
own `Ainari`-resource, for example `ainari`:

```bash
kubectl create namespace ainari
kubectl -n ainari-system get secret private-registry -o yaml \
    | sed '/namespace:/d;/resourceVersion:/d;/uid:/d;/creationTimestamp:/d' \
    | kubectl -n ainari apply -f -
kubectl -n ainari apply -f my_ainari.yaml
```

## Update

A package of a newer version is deployed in the same way as above. The zarf-agent and the
registry stay, so the cluster doesn't have to be initialized again, if the version of zarf is the
same.

## Uninstall

```bash
./zarf package remove ainari --confirm
```

This removes all components of the package, but not their namespaces. Like in the
[Kubernetes-installation](kubernetes_installation.md#uninstall), the generated secrets stay in the
namespace `ainari` and the data of the sakura-hosts on their nodes. An own `Ainari`-resource has
to be deleted before, because the operator is removed with the package. The registry and the agent of
zarf are removed with:

```bash
./zarf destroy --confirm
```
