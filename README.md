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

<p align="center">
  <img src="docs/img/ainari_logo.jpg" alt="Ainari logo" width="500" height="500" />
</p>

## **IMPORTANT**: This project is still a prototype and NOT ready for any productive usage

Ainari is a secure and easy to use, deploy and develop IaaS stack to manage virtual machines.
See more in the [About-section](https://docs.ainari.cloud/home/about/) of the documentation.

## Project

- [About](https://docs.ainari.cloud/home/about/)

- [Features](https://docs.ainari.cloud/home/features/)

- [Architecture Overview](https://docs.ainari.cloud/home/overview/)

## Getting started

- [Quick Start Guide](https://docs.ainari.cloud/home/quick_start/)

- [Example-Workflow](https://docs.ainari.cloud/user/cli_sdk/example_workflow/)

- [Installation-Guide](https://docs.ainari.cloud/deployer/installation/kubernetes_installation/)

- [Dashboard documentation](https://docs.ainari.cloud/user/dashboard/dashboard/)

- [SDK and CLI documentation](https://docs.ainari.cloud/user/cli_sdk/cli_sdk_docu/)

## Development

- [Repository-Overview](https://docs.ainari.cloud/developer/repo_structure/)

- [How to build](https://docs.ainari.cloud/developer/build_guide/)

- [Development-Guide](https://docs.ainari.cloud/developer/development/)

## Pre-build objects

All objects are automatically build and uploaded by the
[CI-pipeline](https://github.com/kitsudaiki/ainari/actions/workflows/build_test.yml) for each merge
on `develop`-branch and for each tag.

- [Docker-images](https://hub.docker.com/u/kitsudaiki)

- [client and SDK](https://files.ainari.cloud/)

## Docker-Images

Vulnerabilities of the images of the `develop`-branch, scanned by
[Docker Scout](https://docs.docker.com/scout/) in the CI-pipeline.

| Image                                                                    | Vulnerabilities                                                                                                                                                                                                |
| ------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [hanami](https://hub.docker.com/r/kitsudaiki/hanami)                     | [![critical][scout_hanami_c]][workflow] [![high][scout_hanami_h]][workflow] [![medium][scout_hanami_m]][workflow] [![low][scout_hanami_l]][workflow] [![unspecified][scout_hanami_u]][workflow]                |
| [miko](https://hub.docker.com/r/kitsudaiki/miko)                         | [![critical][scout_miko_c]][workflow] [![high][scout_miko_h]][workflow] [![medium][scout_miko_m]][workflow] [![low][scout_miko_l]][workflow] [![unspecified][scout_miko_u]][workflow]                          |
| [ryokan](https://hub.docker.com/r/kitsudaiki/ryokan)                     | [![critical][scout_ryokan_c]][workflow] [![high][scout_ryokan_h]][workflow] [![medium][scout_ryokan_m]][workflow] [![low][scout_ryokan_l]][workflow] [![unspecified][scout_ryokan_u]][workflow]                |
| [izakaya](https://hub.docker.com/r/kitsudaiki/izakaya)                   | [![critical][scout_izakaya_c]][workflow] [![high][scout_izakaya_h]][workflow] [![medium][scout_izakaya_m]][workflow] [![low][scout_izakaya_l]][workflow] [![unspecified][scout_izakaya_u]][workflow]           |
| [sakura](https://hub.docker.com/r/kitsudaiki/sakura)                     | [![critical][scout_sakura_c]][workflow] [![high][scout_sakura_h]][workflow] [![medium][scout_sakura_m]][workflow] [![low][scout_sakura_l]][workflow] [![unspecified][scout_sakura_u]][workflow]                |
| [torii](https://hub.docker.com/r/kitsudaiki/torii)                       | [![critical][scout_torii_c]][workflow] [![high][scout_torii_h]][workflow] [![medium][scout_torii_m]][workflow] [![low][scout_torii_l]][workflow] [![unspecified][scout_torii_u]][workflow]                     |
| [omamori](https://hub.docker.com/r/kitsudaiki/omamori)                   | [![critical][scout_omamori_c]][workflow] [![high][scout_omamori_h]][workflow] [![medium][scout_omamori_m]][workflow] [![low][scout_omamori_l]][workflow] [![unspecified][scout_omamori_u]][workflow]           |
| [omikuji](https://hub.docker.com/r/kitsudaiki/omikuji)                   | [![critical][scout_omikuji_c]][workflow] [![high][scout_omikuji_h]][workflow] [![medium][scout_omikuji_m]][workflow] [![low][scout_omikuji_l]][workflow] [![unspecified][scout_omikuji_u]][workflow]           |
| [onsen](https://hub.docker.com/r/kitsudaiki/onsen)                       | [![critical][scout_onsen_c]][workflow] [![high][scout_onsen_h]][workflow] [![medium][scout_onsen_m]][workflow] [![low][scout_onsen_l]][workflow] [![unspecified][scout_onsen_u]][workflow]                     |
| [ainari_dashboard](https://hub.docker.com/r/kitsudaiki/ainari_dashboard) | [![critical][scout_dashboard_c]][workflow] [![high][scout_dashboard_h]][workflow] [![medium][scout_dashboard_m]][workflow] [![low][scout_dashboard_l]][workflow] [![unspecified][scout_dashboard_u]][workflow] |
| [ainari_operator](https://hub.docker.com/r/kitsudaiki/ainari_operator)   | [![critical][scout_operator_c]][workflow] [![high][scout_operator_h]][workflow] [![medium][scout_operator_m]][workflow] [![low][scout_operator_l]][workflow] [![unspecified][scout_operator_u]][workflow]      |

## Author

Tobias Anker

eMail: <tobias.anker@kitsunemimi.moe>

## License

The complete project is under
[Apache 2 license](https://github.com/kitsudaiki/ainari/blob/develop/LICENSE).

[scout_hanami_c]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/hanami/critical/shields.json&style=flat-square
[scout_hanami_h]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/hanami/high/shields.json&style=flat-square
[scout_hanami_m]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/hanami/medium/shields.json&style=flat-square
[scout_hanami_l]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/hanami/low/shields.json&style=flat-square
[scout_hanami_u]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/hanami/unspecified/shields.json&style=flat-square
[scout_miko_c]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/miko/critical/shields.json&style=flat-square
[scout_miko_h]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/miko/high/shields.json&style=flat-square
[scout_miko_m]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/miko/medium/shields.json&style=flat-square
[scout_miko_l]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/miko/low/shields.json&style=flat-square
[scout_miko_u]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/miko/unspecified/shields.json&style=flat-square
[scout_ryokan_c]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/ryokan/critical/shields.json&style=flat-square
[scout_ryokan_h]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/ryokan/high/shields.json&style=flat-square
[scout_ryokan_m]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/ryokan/medium/shields.json&style=flat-square
[scout_ryokan_l]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/ryokan/low/shields.json&style=flat-square
[scout_ryokan_u]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/ryokan/unspecified/shields.json&style=flat-square
[scout_izakaya_c]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/izakaya/critical/shields.json&style=flat-square
[scout_izakaya_h]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/izakaya/high/shields.json&style=flat-square
[scout_izakaya_m]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/izakaya/medium/shields.json&style=flat-square
[scout_izakaya_l]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/izakaya/low/shields.json&style=flat-square
[scout_izakaya_u]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/izakaya/unspecified/shields.json&style=flat-square
[scout_sakura_c]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/sakura/critical/shields.json&style=flat-square
[scout_sakura_h]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/sakura/high/shields.json&style=flat-square
[scout_sakura_m]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/sakura/medium/shields.json&style=flat-square
[scout_sakura_l]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/sakura/low/shields.json&style=flat-square
[scout_sakura_u]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/sakura/unspecified/shields.json&style=flat-square
[scout_torii_c]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/torii/critical/shields.json&style=flat-square
[scout_torii_h]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/torii/high/shields.json&style=flat-square
[scout_torii_m]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/torii/medium/shields.json&style=flat-square
[scout_torii_l]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/torii/low/shields.json&style=flat-square
[scout_torii_u]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/torii/unspecified/shields.json&style=flat-square
[scout_omamori_c]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/omamori/critical/shields.json&style=flat-square
[scout_omamori_h]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/omamori/high/shields.json&style=flat-square
[scout_omamori_m]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/omamori/medium/shields.json&style=flat-square
[scout_omamori_l]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/omamori/low/shields.json&style=flat-square
[scout_omamori_u]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/omamori/unspecified/shields.json&style=flat-square
[scout_omikuji_c]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/omikuji/critical/shields.json&style=flat-square
[scout_omikuji_h]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/omikuji/high/shields.json&style=flat-square
[scout_omikuji_m]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/omikuji/medium/shields.json&style=flat-square
[scout_omikuji_l]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/omikuji/low/shields.json&style=flat-square
[scout_omikuji_u]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/omikuji/unspecified/shields.json&style=flat-square
[scout_onsen_c]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/onsen/critical/shields.json&style=flat-square
[scout_onsen_h]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/onsen/high/shields.json&style=flat-square
[scout_onsen_m]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/onsen/medium/shields.json&style=flat-square
[scout_onsen_l]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/onsen/low/shields.json&style=flat-square
[scout_onsen_u]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/onsen/unspecified/shields.json&style=flat-square
[scout_dashboard_c]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/dashboard/critical/shields.json&style=flat-square
[scout_dashboard_h]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/dashboard/high/shields.json&style=flat-square
[scout_dashboard_m]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/dashboard/medium/shields.json&style=flat-square
[scout_dashboard_l]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/dashboard/low/shields.json&style=flat-square
[scout_dashboard_u]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/dashboard/unspecified/shields.json&style=flat-square
[scout_operator_c]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/operator/critical/shields.json&style=flat-square
[scout_operator_h]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/operator/high/shields.json&style=flat-square
[scout_operator_m]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/operator/medium/shields.json&style=flat-square
[scout_operator_l]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/operator/low/shields.json&style=flat-square
[scout_operator_u]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/ainari-badges/develop/docker_scout/operator/unspecified/shields.json&style=flat-square
[workflow]: https://github.com/kitsudaiki/ainari/actions/workflows/build_test.yml
