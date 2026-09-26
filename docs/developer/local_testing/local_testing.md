# Local test environments

There are three environments, which run every component of ainari locally. They are meant for
development and testing only. Every one of them is described on its own page, so each page can be
read without the others:

1. [Docker-compose setup](docker_compose_setup.md) (`make up local`): all components as
   docker-containers on the host, plain http
2. [Kind setup](kind_setup.md) (`make up kind`): the helm-chart on a kubernetes-cluster of one
   node in docker, https
3. [Vagrant setup](vagrant_setup.md) (`make up vagrant`): the helm-chart on a kubernetes-cluster
   of four virtual machines, https
4. [The CA of the kind- and the vagrant-setup](https_ca.md):

All of them use the same floating ip-addresses (`10.0.0.0/24`), so only one of them can run at a
time. Every page of a setup also describes, how to run the setup out of a container, which brings
all required tools, for a test without installing them on the host.

## Differences between the setups

|                                     | Docker-compose setup                  | Kind setup                                      | Vagrant setup                                           |
| ----------------------------------- | ------------------------------------- | ----------------------------------------------- | ------------------------------------------------------- |
| Start / stop                        | `make up local` / `make down local`   | `make up kind` / `make down kind`               | `make up vagrant` / `make down vagrant`                 |
| Setup-script                        | `scripts/setup_local_stack.sh`        | `scripts/setup_kind_stack.sh`                   | `scripts/setup_vagrant_stack.sh`                        |
| Deployment                          | `docker-compose.yml`                  | helm-chart `deploy/k8s/ainari`                  | helm-chart `deploy/k8s/ainari`                          |
| Configuration                       | `testing/local_stack/configs`          | `deploy/k8s/kind/values.yaml`                   | `testing/vagrant/values.yaml`                           |
| Runs on                             | docker-containers on the host         | one kubernetes-node in docker (kind)            | four virtual machines with k3s (vagrant, libvirt)       |
| Nodes                               | none                                  | 1                                               | 4 (management, edge-gateway, 2 sakura-hosts)            |
| Placement of the components         | all on the host                       | all on the one node                             | on the virtual machines with their label                |
| Protocol of the api                 | http                                  | https                                           | https                                                   |
| Certificates                        | none                                  | own CA, `temporary_files/kind`                  | own CA, `temporary_files/vagrant`                       |
| Address of the api                  | `http://127.0.0.1:<port>`             | `https://127.0.0.1:<port>`                      | `https://192.168.56.10:<port>`, torii on `.11`          |
| Dashboard                           | has to be deployed manually           | yes, port 11422                                 | yes, port 11422                                         |
| Network between the gateways        | one docker-bridge (shared link)       | routed pod-network of kind                      | flannel (vxlan) between the virtual machines            |
| Uplink of the edge-gateway          | `veth-host` on the host               | `veth-kind` on the host                         | `veth-uplink` on the virtual machine `ainari-torii`     |
| Reaching the floating ip-addresses  | directly over `veth-host`             | directly over `veth-kind`                       | over the route `10.0.0.0/24 via 192.168.56.11`          |
| NAT of the virtual machines         | on the host                           | on the host                                     | on the virtual machine `ainari-torii`                   |
| Virtualization of the sakura-hosts  | kvm of the host                       | kvm of the host                                 | nested kvm within the virtual machines                  |
| Needs sudo for                      | the whole setup-script                | the veth-pair and the NAT-rules                 | only the route on the host                              |
| Tools on the host                   | docker compose                        | kind, kubectl, helm, openssl                    | vagrant-libvirt, ansible, openssl                       |
| Host-sockets for the tools-container | docker                               | docker                                          | docker, libvirt                                         |
| eBPF-support of the host-kernel     | required                              | required                                        | not required                                            |
| Resources                           | smallest                              | small                                           | about 20 GiB memory, 25 GiB disk                        |
| Time to start                       | fast                                  | a few minutes                                   | longest (virtual machines, k3s)                         |
| Suited for                          | fast development of the components    | testing the helm-chart                          | testing a real multi-node deployment                    |
