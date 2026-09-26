# Kubernetes-installation

!!! warning

    The installation process is still only for testing, because many important parts are not
    implemented yet.

The whole stack is installed with the helm-chart in `deploy/k8s/ainari` on an existing kubernetes.

## Requirements

- **Kubernetes** with `kubectl` access

    The nodes for sakura need `/dev/kvm`. Sakura and torii run as privileged pods, because they
    attach eBPF-programs to their interfaces. The volumes use the storage-class `local-path` by
    default, which can be changed with `global.storage_class`.

    !!! example

        For a minimal single-node installation `k3s` without traefik can be used:

        ```bash
        curl -sfL https://get.k3s.io | INSTALL_K3S_EXEC="--disable traefik" sh -

        export KUBECONFIG=/etc/rancher/k3s/k3s.yaml
        ```

- **Helm**

    [official Installation-Guide](https://helm.sh/docs/intro/install/)

- **wireguard-tools** and **python3** with `jinja2` on the host, to create the wireguard-configs

    ```bash
    sudo apt-get install wireguard-tools
    python3 -m venv .venv
    source .venv/bin/activate
    pip3 install jinja2
    ```

## Installation

1. **Get the helm-chart**

    ```bash
    git clone https://github.com/kitsudaiki/ainari.git
    cd ainari/deploy/k8s
    ```

    Alternatively the pre-built chart `ainari-x.y.z.tgz` can be downloaded from the
    [file-share](https://files.ainari.cloud/) and used instead of `./ainari` in step 7.

1. **Ingress-nginx-controller** (if not already exist in your kubernetes)

    The ingresses of the chart use the ingress-class `nginx`.

    ```bash
    helm upgrade --install ingress-nginx ingress-nginx \
        --repo https://kubernetes.github.io/ingress-nginx \
        --namespace ingress-nginx --create-namespace
    ```

    Without an ingress-controller set `global.ingress.enabled: false` and
    `global.external_services.enabled: true` in step 6, to publish the apis over node-ports.

1. **Cert-Manager** (if not already exist in your kubernetes)

    Required in any case, because all components talk https to each other with certificates of
    cert-manager.

    ```bash
    kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.18.2/cert-manager.yaml
    kubectl -n cert-manager wait deployment --all --for=condition=Available --timeout=300s
    ```

1. **Namespace and wireguard-configs**

    The connection of ryokan and sakura to onsen runs through wireguard. The configs are created and
    uploaded as secrets into the namespace of the installation:

    ```bash
    kubectl create namespace ainari
    python3 wg_gen.py --namespace ainari
    ```

1. **Node labels**

    Every component is only scheduled on nodes with its label and never twice on the same node:

    ```bash
    kubectl label nodes NODE_NAME miko-node=true
    kubectl label nodes NODE_NAME hanami-node=true
    kubectl label nodes NODE_NAME ryokan-node=true
    kubectl label nodes NODE_NAME omamori-node=true
    kubectl label nodes NODE_NAME onsen-node=true
    kubectl label nodes NODE_NAME sakura-node=true
    kubectl label nodes NODE_NAME torii-node=true
    kubectl label nodes NODE_NAME ainari-dashboard-node=true
    ```

    On a cluster with only one node set `global.strict_scheduling: false` in step 6 instead.

1. **Values**

    Create a file `my_values.yaml` with at least the following content. All other values and their
    defaults are described in `deploy/k8s/ainari/values.yaml`.

    ```yaml
    secrets:
      internal_api_key: "RANDOM_KEY_1"
      onsen_registration_key: "RANDOM_KEY_2"
      sakura_registration_key: "RANDOM_KEY_3"

    miko:
      user:
        id: "USER_ID"
        name: "USER_NAME"
        passphrase: "PASSPHRASE"
      token:
        data: "TOKEN_KEY"

    sakura:
      # output of 'stat -c %g /dev/kvm' on the sakura-nodes
      kvm_gid: KVM_GID

    torii:
      public:
        network:
          overlay_iface: "UPLINK_IFACE"
          uplink_iface: "UPLINK_IFACE"
          uplink_next_hop: "UPLINK_NEXT_HOP"

    hanami:
      network:
        floating_ip_cidr: "FLOATING_IP_CIDR"
    ```

    - `USER_ID`, `USER_NAME`, `PASSPHRASE`

        - **required**
        - Login of the initial admin-user.
        - `USER_ID` MUST match the regex `[a-zA-Z][a-zA-Z_0-9@]*`, `USER_NAME` the regex
            `[a-zA-Z][a-zA-Z_0-9 ]*`, both with between `4` and `256` characters length.
            `PASSPHRASE` MUST have between `8` and `4096` characters.

    - `TOKEN_KEY`

        - **required**
        - Key to sign the tokens of the users. See [Token-Key](../config/token_key.md).

    - `RANDOM_KEY_*`

        - Keys for the internal communication. The defaults of the chart are public, so always
          replace them.

    - `UPLINK_IFACE`, `UPLINK_NEXT_HOP`, `FLOATING_IP_CIDR`

        - Uplink of the gateway `torii-public`, on which the floating IPs out of
          `FLOATING_IP_CIDR` are served, and the router behind it. The interface has to exist
          within the pod of `torii-public`. If it is moved into the pod after its start, set its
          name additionally as `torii.public.wait_for_iface`. `scripts/setup_kind_stack.sh`
          shows an example, which injects a veth-pair into the pod.

    - Docker-images

        - Every component uses the tag `develop` of
          [docker-hub](https://hub.docker.com/u/kitsudaiki) by default. Another version is set
          per component with `<component>.docker.tag`, for example `miko.docker.tag`.

    - Domains

        - Every component is reachable over the domain `<component>.api.domain`, by default
          `local-miko`, `local-hanami`, `local-ryokan`, `local-omamori`, `local-torii` and
          `local-ainari` for the dashboard.

1. **Install**

    ```bash
    helm install ainari ./ainari --namespace ainari --values my_values.yaml
    ```

    After a successful installation the `USER_ID` and `PASSPHRASE` have to be used for login to
    the system.

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

- trust the CA, which signed all certificates. If `global.certificates.ca_secret` is not set,
    cert-manager creates it in the secret `ainari-ca`:

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
helm uninstall ainari --namespace ainari
kubectl delete namespace ainari
```
