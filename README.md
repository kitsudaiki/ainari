# Ainari

![Latest Release](https://img.shields.io/github/v/release/kitsudaiki/ainari?include_prereleases&label=Version&style=flat-square)
![License](https://img.shields.io/github/license/kitsudaiki/ainari?style=flat-square)
![Platform](https://img.shields.io/badge/Platform-Linux-blue?style=flat-square)
![Architecture](https://img.shields.io/badge/Architecture-amd64%20%2B%20arm64-blue?style=flat-square)

[![Github workflow status](https://img.shields.io/github/actions/workflow/status/kitsudaiki/ainari/build_test.yml?branch=develop&style=flat-square&label=Build%20and%20Test)](https://github.com/kitsudaiki/ainari/actions/workflows/build_test.yml)
[![RS Report](https://rust-reportcard.xuri.me/badge/github.com/kitsudaiki/ainari?style=flat-square)](https://rust-reportcard.xuri.me/report/github.com/kitsudaiki/ainari)
[![CodeQL](https://img.shields.io/github/actions/workflow/status/kitsudaiki/ainari/codeql.yml?branch=develop&style=flat-square&label=CodeQL)](https://github.com/kitsudaiki/ainari/actions/workflows/codeql.yml)
[![OpenSSF Scorecard](https://img.shields.io/ossf-scorecard/github.com/kitsudaiki/ainari?branch=develop&style=flat-square&label=OpenSSF-Scorecard)](https://scorecard.dev/viewer/?uri=github.com/kitsudaiki/ainari)

# **IMPORTANT**: Currently under heavy reconstruction

<img align="left" width="210" height="210" src="assets/wip.jpg">

### **This project is currently under heavy reconstruction,** 
### **by moving this project from an AIaaS to an IaaS**
### **The old neural network core was outsourced to the side-project [Saki](https://github.com/kitsudaiki/saki)**
### **Last version of this project with the old neural network core is v0.11.1**


<!-- ## Supported Environment

| Python-SDK                                  | Deployment                                          |
| ------------------------------------------- | --------------------------------------------------- |
| [![python-3_10][img_python-3_10]][workflow] | [![kubernetes-1_30][img_kubernetes-1_30]][workflow] |
| [![python-3_11][img_python-3_11]][workflow] | [![kubernetes-1_31][img_kubernetes-1_31]][workflow] |
| [![python-3_12][img_python-3_12]][workflow] | [![kubernetes-1_32][img_kubernetes-1_32]][workflow] |
|                                             | [![kubernetes-1_33][img_kubernetes-1_33]][workflow] | -->

## Overview

Ainari is split into a micro-service architecture. See here for
[Overview-Description](https://docs.ainari.cloud/home/overview/)

<p align="center">
  <img src="assets/ainari_overview.jpg" width="1500" height="700" />
</p>

<!-- ## Getting started

- [Example-Workflow](https://docs.ainari.cloud/user/cli_sdk/example_workflow/)

- [Installation-Guide](https://docs.ainari.cloud/deployer/installation/kubernetes_installation/)

- [SDK and CLI documentation](https://docs.ainari.cloud/user/cli_sdk/cli_sdk_docu/)

- [Automatic generated OpenAPI documentation](https://docs.ainari.cloud/user/rest_api/rest_api_docu_sakura/)

## Development

- [How to build](https://docs.ainari.cloud/developer/repo/build_guide/)

- [Development-Guide](https://docs.ainari.cloud/developer/repo/development/)

- [Dependency-Overview](https://docs.ainari.cloud/developer/repo/dependencies/) -->

## Pre-build objects

All objects are automatically build and uploaded by the
[CI-pipeline](https://github.com/kitsudaiki/ainari/actions/workflows/build_test.yml) for each merge
on `develop`-branch and for each tag.

- [Docker-images](https://hub.docker.com/u/kitsudaiki)

- [client, SDK and helm-chart](https://files.ainari.cloud/)

## Author

**Tobias Anker**

eMail: tobias.anker@kitsunemimi.moe

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
