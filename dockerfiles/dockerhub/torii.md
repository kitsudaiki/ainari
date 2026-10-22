# Ainari - Torii

eBPF-based network-gateway of Ainari for the networks, floating IPs and API-proxy of the VMs.

> **IMPORTANT:** Ainari is still a prototype and NOT ready for any productive usage.

## About

Torii is the gateway-component of the stack. It is configured by Hanami and handles two pathways:

1. **API-proxy:** a layer-3-proxy with one port per virtual machine, over which the user talks
   directly to the Sakura-host of the virtual machine. Torii doesn't terminate the
   HTTPS-connection, so it stays end-to-end encrypted between the user and the Sakura-host.
2. **Networks:** the networks between the virtual machines and from the virtual machines to the
   internet, including the floating IPs. The datapath runs as eBPF-programs.

The same image is used for every gateway, the role is only a question of the config: one public
Torii at the edge of the network with an uplink, and one Torii in front of every Sakura-host,
which owns the TAP-devices of the virtual machines of that host.

It is one component of [Ainari](https://github.com/kitsudaiki/ainari), a secure and easy to use,
deploy and develop IaaS stack to manage virtual machines.

## Image

The image is built `FROM scratch` with [nix](https://nixos.org): it contains only the pinned
runtime-packages of the component and its binary, no shell-tools for debugging. The list of all
packages is in `/etc/nix-packages.txt` of the image:

```bash
docker run --rm --entrypoint cat kitsudaiki/torii:develop /etc/nix-packages.txt
```

## Links

- Source: <https://github.com/kitsudaiki/ainari>
- Documentation: <https://docs.ainari.cloud>
- Architecture overview: <https://docs.ainari.cloud/home/overview/>
- Installation guide: <https://docs.ainari.cloud/deployer/installation/kubernetes_installation/>

## License

Apache License 2.0, see [LICENSE](https://github.com/kitsudaiki/ainari/blob/develop/LICENSE).
