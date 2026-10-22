# Ainari - Kubernetes-Operator

Kubernetes-operator, which deploys and manages the complete Ainari stack of an `Ainari`-resource.

> **IMPORTANT:** Ainari is still a prototype and NOT ready for any productive usage.

## About

The operator installs the whole stack of [Ainari](https://github.com/kitsudaiki/ainari), a secure
and easy to use, deploy and develop IaaS stack to manage virtual machines, on an existing
kubernetes. It deploys all components of a single custom-resource of the kind `Ainari`, generates
all keys and passwords itself and keeps the stack in the state of the resource. All components run
with the pre-built images of [docker-hub](https://hub.docker.com/u/kitsudaiki).

## Tags

| Tag | Content |
| --- | --- |
| `develop` | latest state of the `develop`-branch |
| `<VERSION>` | a release, without the leading `v`, for example `0.21.0` |

All images are multi-arch for `linux/amd64` and `linux/arm64`.

## Usage

The operator is not started by hand, but installed with the manifests of
[`deploy/operator`](https://github.com/kitsudaiki/ainari/tree/develop/deploy/operator) of the
repository. The cluster additionally needs cert-manager and an implementation of the gateway-api
with support of `TLSRoute` in the mode `Passthrough`, for example Envoy Gateway.

The complete steps, the requirements of the nodes and all options of the `Ainari`-resource are
described in the [installation-guide](https://docs.ainari.cloud/deployer/installation/kubernetes_installation/).
For clusters without access to the internet, see the
[offline-installation](https://docs.ainari.cloud/deployer/installation/offline_installation/).

## Image

The image is built `FROM scratch` with [nix](https://nixos.org): it contains only the statically
linked operator and its pinned runtime-packages, no shell-tools for debugging. It runs as the
non-root user with the id `1000`. The list of all packages is in `/etc/nix-packages.txt` of the
image:

```bash
docker run --rm --entrypoint cat kitsudaiki/ainari_operator:develop /etc/nix-packages.txt
```

## Links

- Source: <https://github.com/kitsudaiki/ainari>
- Documentation: <https://docs.ainari.cloud>
- Installation guide: <https://docs.ainari.cloud/deployer/installation/kubernetes_installation/>

## License

Apache License 2.0, see [LICENSE](https://github.com/kitsudaiki/ainari/blob/develop/LICENSE).
