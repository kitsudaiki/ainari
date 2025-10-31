# Installation

!!! warning

    The installation process is also very basic at the moment. So for example only self-signed certificates are used at the moment. Because of this the given SDK also has a problem to connect correctly to these installation currently.

## On Kubernetes

For the installation on a kubernetes `helm` is used.

| Supported versions                                  |
| --------------------------------------------------- |
| [![kubernetes-1_30][img_kubernetes-1_30]][Workflow] |
| [![kubernetes-1_31][img_kubernetes-1_31]][Workflow] |
| [![kubernetes-1_32][img_kubernetes-1_32]][Workflow] |
| [![kubernetes-1_33][img_kubernetes-1_33]][Workflow] |

[img_kubernetes-1_30]:
    https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/Ainari-badges/develop/kubernetes_version/kubernetes-1_30/shields.json&style=flat-square
[img_kubernetes-1_31]:
    https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/Ainari-badges/develop/kubernetes_version/kubernetes-1_31/shields.json&style=flat-square
[img_kubernetes-1_32]:
    https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/Ainari-badges/develop/kubernetes_version/kubernetes-1_32/shields.json&style=flat-square
[img_kubernetes-1_33]:
    https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/Ainari-badges/develop/kubernetes_version/kubernetes-1_33/shields.json&style=flat-square
[Workflow]: https://github.com/kitsudaiki/ainari/actions/workflows/build_test.yml

### Requirements

1.  **Kubernetes**

    No specific version a the moment known. There are no special features used at the moment, so any
    version, which is not EOL should work.

    !!! example

        For fast, easy and minimal installation a `k3s` as single-node installation can be used without traefik. Installation with for example:

        ```
        sudo curl -sfL https://get.k3s.io | INSTALL_K3S_EXEC="--disable traefik" sh -

        export KUBECONFIG=/etc/rancher/k3s/k3s.yaml
        ```

2.  **Helm**

    [official Installation-Guide](https://helm.sh/docs/intro/install/)

3.  **Nginx-ingress-controller** (if not already exist in your kubernetes)

    run: 

    ```
    kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/controller-v1.8.1/deploy/static/provider/baremetal/deploy.yaml
    ```

    and apply file with the content:

    ```
    ---

    apiVersion: v1
    kind: Service
    metadata:
      name: ingress-nginx-controller-loadbalancer
      namespace: ingress-nginx
    spec:
      selector:
        app.kubernetes.io/component: controller
        app.kubernetes.io/instance: ingress-nginx
        app.kubernetes.io/name: ingress-nginx
      ports:
        - name: http
          port: 80
          protocol: TCP
          targetPort: 80
        - name: https
          port: 443
          protocol: TCP
          targetPort: 443
      type: LoadBalancer
    ```

4.  **Cert-Manager**

    Installation:

    ```
    helm repo add jetstack https://charts.jetstack.io
    helm repo update
    kubectl create namespace cert-manager
    helm install cert-manager jetstack/cert-manager --namespace cert-manager --set installCRDs=true
    ```

5.  **Install longhorn**

    Install required apt-package:

    ```
    sudo apt-get install -y open-iscsi nfs-common
    sudo systemctl enable --now iscsid
    ```

    Install longhorn

    ```
    helm repo add longhorn https://charts.longhorn.io
    helm repo update
    kubectl create namespace longhorn-system
    helm install longhorn longhorn/longhorn --namespace longhorn-system
    ```

    Create storage-class by applying:

    ```
    ---

    apiVersion: storage.k8s.io/v1
    kind: StorageClass
    metadata:
      name: longhorn
    provisioner: driver.longhorn.io
    parameters:
      numberOfReplicas: "1"
      staleReplicaTimeout: "30"
    volumeBindingMode: Immediate
    reclaimPolicy: Delete
    allowVolumeExpansion: true
    mountOptions: []
    parameters: {}
    ```

6.  **Node label**

    To all avaialbe nodes, where it is allowed to be deployed:

    ```
    kubectl label nodes NODE_NAME sakura-node=true
    kubectl label nodes NODE_NAME miko-node=true
    kubectl label nodes NODE_NAME bento-node=true
    ```

    !!! info

        At the moment Ainari is only a single-node application. This will change in the near future, but at the moment it doesn't make sense to label more than one node.

<!-- 3. If measuring of the cpu power consumption should be available, then the following requirements must be fulfilled on the hosts of the kubernetes-deployment:

    - Required specific CPU-architecture:
        - **Intel**:
            - Sandy-Bridge or newer
        - **AMD** :
            - Zen-Architecture or newer
            - for CPUs of AMD Zen/Zen2 Linux-Kernel of version `5.8` or newer must be used, for Zen3 Linux-Kernel of version `5.11` or newer

    - the `msr`-kernel module has to be loaded with `modeprobe msr`. -->

### Installation

**From repository**

```bash
git clone https://github.com/kitsudaiki/ainari.git

cd Ainari/deploy/k8s

helm install \
    --set docker.tag=DOCKER_IMAGE_TAG \
    --set user.id=USER_ID  \
    --set user.name=USER_NAME  \
    --set user.passphrase=PASSPHRASE  \
    --set token.data=TOKEN_KEY  \
    --set api.domain=DOMAIN_NAME  \
    ainari \
    ./ainari/
```

**From pre-build**

Download the helm-chart from the [file-share](https://files.ainari.cloud/)

```bash
helm install \
    --set docker.tag=DOCKER_IMAGE_TAG \
    --set user.id=USER_ID  \
    --set user.name=USER_NAME  \
    --set user.passphrase=PASSPHRASE  \
    --set token.data=TOKEN_KEY  \
    --set api.domain=DOMAIN_NAME  \
    ainari \
    ainari-x.y.z.tgz
```

The `--set`-flag defining the login-information for the initial admin-user of the instance:

-   `USER_ID`

    -   **required**
    -   Identifier for the new user. It is used for login and internal references to the user.
    -   String, which MUST match the regex `[a-zA-Z][a-zA-Z_0-9@]*` with between `4` and `256`
        characters length

-   `USER_NAME`

    -   **required**
    -   Better readable name for the user, which doesn't have to be unique in the system.
    -   String, which MUST match the regex `[a-zA-Z][a-zA-Z_0-9 ]*` with between `4` and `256`
        characters length

-   `PASSPHRASE`

    -   **required**
    -   Passphrase for the initial user
    -   String, with between `8` and `4096` characters length

-   `TOKEN_KEY`

    -   **required**
    -   Key for the JWT-Tokens
    -   String

-   `DOMAIN_NAME`

    -   Domain for https-access.
    -   String
    -   default: *local-sakura*

-   `DOCKER_IMAGE_TAG`
    -   Docker-tag used from
        [docker-hub](https://hub.docker.com/repository/docker/kitsudaiki/sakura/tags)
    -   String
    -   default: *develop*

After a successful installation the `USER_ID` and `PASSPHRASE` have to be used for login to the
system.

### Using

-   check if all pods are running

    !!! example

          ```bash
          kubectl get pods

          NAME                      READY   STATUS    RESTARTS   AGE
          sakura-56fc87c8f5-6k77r   1/1     Running   0          14s
          ```

-   get IP-address

    !!! example

          ```bash
          kubectl get ingress

          NAME                      CLASS     HOSTS          ADDRESS          PORTS     AGE
          sakura-ingress-redirect   traefik   local-sakura   192.168.178.87   80        8s
          sakura-ingress            traefik   local-sakura   192.168.178.87   80, 443   8s
          ```

-   add domain with ip to `/etc/hosts`

    !!! example

          ```
          192.168.178.87  local-sakura
          ```

-   use the address in your browser:

    `https://DOMAIN_NAME`

    !!! example

          ```
          https://local-sakura/
          ```

-   login with `USER_ID` and `PASSPHRASE`

    !!! note

        Persistent data of sakura (checkpoints and so on) within the pod are mounted into the host directory `/etc/ainari_data`, in case you want to save or delete them.

