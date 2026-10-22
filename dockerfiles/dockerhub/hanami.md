# Ainari - Hanami

Manager of the compute-hosts of Ainari: scheduling of VMs, networks and floating IPs.

> **IMPORTANT:** Ainari is still a prototype and NOT ready for any productive usage.

## About

Hanami manages the Sakura compute-hosts. Every request for a new virtual machine goes to Hanami,
which selects the Sakura-host for it and configures the Torii-gateways for the new connection. It
also handles networks, floating IPs and network-filters.

It is one component of [Ainari](https://github.com/kitsudaiki/ainari), a secure and easy to use,
deploy and develop IaaS stack to manage virtual machines.

## Image

The image is built `FROM scratch` with [nix](https://nixos.org): it contains only the pinned
runtime-packages of the component and its binary, no shell-tools for debugging. The list of all
packages is in `/etc/nix-packages.txt` of the image:

```bash
docker run --rm --entrypoint cat kitsudaiki/hanami:develop /etc/nix-packages.txt
```

The container runs as the non-root user `ainari` (uid `1000`).

## Links

- Source: <https://github.com/kitsudaiki/ainari>
- Documentation: <https://docs.ainari.cloud>
- Architecture overview: <https://docs.ainari.cloud/home/overview/>
- Installation guide: <https://docs.ainari.cloud/deployer/installation/kubernetes_installation/>

## License

Apache License 2.0, see [LICENSE](https://github.com/kitsudaiki/ainari/blob/develop/LICENSE).
