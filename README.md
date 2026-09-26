# Ainari

![Latest
Release](https://img.shields.io/github/v/release/kitsudaiki/ainari?include_prereleases&label=Version&style=flat-square)
![License](https://img.shields.io/github/license/kitsudaiki/ainari?style=flat-square)
![Platform](https://img.shields.io/badge/Platform-Linux-blue?style=flat-square)
![Architecture](https://img.shields.io/badge/Architecture-amd64%20%2B%20arm64-blue?style=flat-square)

[![Github workflow
status](https://img.shields.io/github/actions/workflow/status/kitsudaiki/ainari/build_test.yml?branch=develop&style=flat-square&label=Build%20and%20Test)](https://github.com/kitsudaiki/ainari/actions/workflows/build_test.yml)
[![RS
Report](https://rust-reportcard.xuri.me/badge/github.com/kitsudaiki/ainari?style=flat-square)](https://rust-reportcard.xuri.me/report/github.com/kitsudaiki/ainari)
[![CodeQL](https://img.shields.io/github/actions/workflow/status/kitsudaiki/ainari/codeql.yml?branch=develop&style=flat-square&label=CodeQL)](https://github.com/kitsudaiki/ainari/actions/workflows/codeql.yml)
[![OpenSSF
Scorecard](https://img.shields.io/ossf-scorecard/github.com/kitsudaiki/ainari?branch=develop&style=flat-square&label=OpenSSF-Scorecard)](https://scorecard.dev/viewer/?uri=github.com/kitsudaiki/ainari)

## **IMPORTANT**: This project is still a prototype and NOT ready for any productive usage

<p align="center">
  <img src="docs/img/ainari_logo.jpg" alt="Ainari logo" width="500" height="500" />
</p>

<!-- ## Supported Environment

| Python-SDK                                  | Deployment                                          |
| ------------------------------------------- | --------------------------------------------------- |
| [![python-3_10][img_python-3_10]][workflow] | [![kubernetes-1_30][img_kubernetes-1_30]][workflow] |
| [![python-3_11][img_python-3_11]][workflow] | [![kubernetes-1_31][img_kubernetes-1_31]][workflow] |
| [![python-3_12][img_python-3_12]][workflow] | [![kubernetes-1_32][img_kubernetes-1_32]][workflow] |
|                                             | [![kubernetes-1_33][img_kubernetes-1_33]][workflow] |
-->

## About

This project contains an entire IaaS (Infrastructure-as-a-Service) stack to manage virtual machines
and networks between them. It is focused on security and easy handling for users, admins and
developers, not on scalability. It is entirely a spare-time project of mine, without a bigger
purpose I need it for. At work I have and had a lot to do with the IaaS Openstack in many ways:
administration, deployment and even modifications of the code. At my time in the company SecuStack,
which was closed years ago, we developed and implemented security features directly into the
existing code of Openstack. I saw many bad things in the handling and the code and the Ainari
project here became basically a platform, where I currently implement my own vision of an IaaS, by
implementing my own architecture, avoiding every pain point I had with Openstack, implementing
security features we had or at least planned for Openstack back then, and implementing my own
feature ideas. See [feature overview](https://docs.ainari.cloud/home/features/).

Originally the project started years ago with an experimental neural network in C++. Later it was
moved to the server side with a REST-API. Then came a user-management, key-management, installation
automation, dashboard and so on. Over time, this effectively turned it into a
Neural-Network-as-a-Service. Not because I needed it, but because I like programming and I was
simply interested in implementing features into a project freely. In 2025 I manually refactored the
entire C++ code base into Rust and the simple dashboard from plain JavaScript into Vue.js with
TypeScript. The problem was that the experimental neural network core was still an experimental
construction site, which worked well for small tests, but was never as good as hoped and was
rewritten nearly every year for new conceptual ideas. So the infrastructure had a much better
quality than the core and was way over the top for what the neural network was usable for. So in
late summer 2026 the experimental neural network core was pulled out of this repo here and moved
into the small side-project [Saki](https://github.com/kitsudaiki/saki), where it runs as a small
library, which can be included in Python scripts. To reuse the remaining infrastructure of this
project and give it a real productive purpose, Ainari was moved into the direction of an IaaS
project. Instead of managing neural networks, virtual machines are now managed and datasets were
replaced by disk-images. Big parts were already compatible with the new direction and required no or
only a bit of modification work for the new core function.

Until I started to push the project into this new direction, everything in the project, except some
tiny code snippets, was written by hand: backend, frontend, documentation and automation. Because I
ran into big problems while implementing the new desired network stack for the new IaaS approach
with eBPF, I was more or less forced into using AI coding tools. The low-level network stuff with
silently dropped packets in kernel space was horrible to debug. I have to admit that I was impressed
by how well it was debugged and fixed by AI and how much time and nerves it saved me. Since then, I
have been using AI more actively, to speed up the progress. I still fix many things myself, review
each generated piece of code and make manual modifications to it. In the end I have to admit that I
actually use more AI in this project now than I ever expected. But because the very big foundation
of the project was clearly structured and written by hand, and because I keep control over
everything generated, this project has nearly no technical debt. Whenever I see something to
refactor in this project, I do it, because I know I avoid a lot of pain if I fix it as soon as
possible.

## Getting started

- [Example-Workflow](https://docs.ainari.cloud/user/cli_sdk/example_workflow/)

- [Installation-Guide](https://docs.ainari.cloud/deployer/installation/kubernetes_installation/)

- [Dashboard documentation](https://docs.ainari.cloud/user/dashboard/dashboard/)

- [SDK and CLI documentation](https://docs.ainari.cloud/user/cli_sdk/cli_sdk_docu/)

## Development

- [Repository-Overview](https://docs.ainari.cloud/developer/repo_structure/)

- [How to build](https://docs.ainari.cloud/developer/repo/build_guide/)

- [Development-Guide](https://docs.ainari.cloud/developer/repo/development/)

## Quick start

Runs the whole stack locally with docker-compose. See the [docker-compose
setup](https://docs.ainari.cloud/developer/local_testing/docker_compose_setup/#running-from-the-tools-container)
for details.

### Requirements

- Linux host with docker
- `/dev/kvm` and `/dev/net/tun`
- kernel with eBPF/XDP support
- at least 4 GiB free memory for the two test virtual machines

### Deploy

In the root of the repository, build the toolbox image and start it:

```bash
docker build -f dockerfiles/Dockerfile_local_test_tools -t ainari/local-test-tools .

docker run --rm -it --privileged --network host --pid host \
    -e HOST_UID=$(id -u) -e HOST_GID=$(id -g) \
    -v /var/run/docker.sock:/var/run/docker.sock \
    -v "$PWD:$PWD" -w "$PWD" \
    ainari/local-test-tools
```

Within the container, start the stack:

```bash
make up local
```

### Create virtual machines

Still within the container, create an ssh-key, an image, a network and two virtual machines with
floating ip-addresses:

```bash
python3 testing/local_stack/vm_lifecycle_test.py
```

### Access the virtual machines

At the end, the script prints one ssh-command for each virtual machine, which also works from
the host:

```bash
ssh -i temporary_files/local_stack_test/id_ed25519 ubuntu@<FLOATING_IP>
```

### Use the CLI

Within the container, build the CLI and point it at the stack:

```bash
cd src/cli/ainarictl && go build .
source test_auth.sh    # user 'asdf', passphrase 'asdfasdf'
./ainarictl vm list
./ainarictl --help
```

### Use the dashboard

On the host, create the config of the dashboard and start it:

```bash
sudo mkdir -p /etc/ainari
echo '{"apiUrl": "http://localhost:11417"}' | sudo tee /etc/ainari/dashboard_config.json

cd src/dashboard
docker compose up --build
```

Open [http://localhost:5173](http://localhost:5173) and log in with `asdf` / `asdfasdf`.

### Stop

Stop the dashboard with `Ctrl+C` and the stack within the toolbox container with:

```bash
make down local
```

## Pre-build objects

All objects are automatically build and uploaded by the
[CI-pipeline](https://github.com/kitsudaiki/ainari/actions/workflows/build_test.yml) for each merge
on `develop`-branch and for each tag.

- [Docker-images](https://hub.docker.com/u/kitsudaiki)

- [client, SDK and helm-chart](https://files.ainari.cloud/)

## Author

Tobias Anker

eMail: <tobias.anker@kitsunemimi.moe>

## License

The complete project is under
[Apache 2 license](https://github.com/kitsudaiki/ainari/blob/develop/LICENSE).

<!-- [img_kubernetes-1_30]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/kubernetes_version/kubernetes-1_30/shields.json&style=flat-square
[img_kubernetes-1_31]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/kubernetes_version/kubernetes-1_31/shields.json&style=flat-square
[img_kubernetes-1_32]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/kubernetes_version/kubernetes-1_32/shields.json&style=flat-square
[img_kubernetes-1_33]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/kubernetes_version/kubernetes-1_33/shields.json&style=flat-square
[img_python-3_10]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/python_version/python-3_10/shields.json&style=flat-square
[img_python-3_11]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/python_version/python-3_11/shields.json&style=flat-square
[img_python-3_12]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/python_version/python-3_12/shields.json&style=flat-square
[workflow]: https://github.com/kitsudaiki/ainari/actions/workflows/build_test.yml -->
