# Ainari - Izakaya

Key-exchange of Ainari, which distributes the IPsec-keys to the hosts with MLS.

> **IMPORTANT:** Ainari is still a prototype and NOT ready for any productive usage.

## About

Izakaya provides the key-exchange, which brings the keys for the IPsec-encryption of the traffic
between the hosts to the Sakura-hosts and Torii-gateways. It uses MLS (Messaging Layer Security,
RFC 9420) for the exchange. The membership-grants of the MLS-groups are signed by Hanami.

It is one component of [Ainari](https://github.com/kitsudaiki/ainari), a secure and easy to use,
deploy and develop IaaS stack to manage virtual machines.

## Image

The image is built `FROM scratch` with [nix](https://nixos.org): it contains only the pinned
runtime-packages of the component and its binary, no shell-tools for debugging. The list of all
packages is in `/etc/nix-packages.txt` of the image:

```bash
docker run --rm --entrypoint cat kitsudaiki/izakaya:develop /etc/nix-packages.txt
```

The container runs as the non-root user `ainari` (uid `1000`).

## Links

- Source: <https://github.com/kitsudaiki/ainari>
- Documentation: <https://docs.ainari.cloud>
- Architecture overview: <https://docs.ainari.cloud/home/overview/>
- Installation guide: <https://docs.ainari.cloud/deployer/installation/kubernetes_installation/>

## License

Apache License 2.0, see [LICENSE](https://github.com/kitsudaiki/ainari/blob/develop/LICENSE).
