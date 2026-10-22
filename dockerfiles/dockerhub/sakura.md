# Ainari - Sakura

Compute-host of Ainari, which runs the virtual machines with cloud-hypervisor.

> **IMPORTANT:** Ainari is still a prototype and NOT ready for any productive usage.

## About

Sakura is the compute-host of the stack. It manages the virtual machines, which run with
[cloud-hypervisor](https://www.cloudhypervisor.org), and is itself managed by Hanami. The users
access the control-endpoints of their virtual machines end-to-end encrypted over the proxy-ports
of the Torii-gateway.

It is one component of [Ainari](https://github.com/kitsudaiki/ainari), a secure and easy to use,
deploy and develop IaaS stack to manage virtual machines.

## Image

The image is built `FROM scratch` with [nix](https://nixos.org): it contains only the pinned
runtime-packages of the component and its binary, no shell-tools for debugging. The list of all
packages is in `/etc/nix-packages.txt` of the image:

```bash
docker run --rm --entrypoint cat kitsudaiki/sakura:develop /etc/nix-packages.txt
```

The container runs as the non-root user `ainari` (uid `1000`).

## Links

- Source: <https://github.com/kitsudaiki/ainari>
- Documentation: <https://docs.ainari.cloud>
- Architecture overview: <https://docs.ainari.cloud/home/overview/>
- Installation guide: <https://docs.ainari.cloud/deployer/installation/kubernetes_installation/>

## License

Apache License 2.0, see [LICENSE](https://github.com/kitsudaiki/ainari/blob/develop/LICENSE).
