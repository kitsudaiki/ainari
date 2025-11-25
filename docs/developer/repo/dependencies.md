# Dependencies

## Backend

### Build sakura

#### apt-packages

| apt-package    | Purpose                                             |
| -------------- | --------------------------------------------------- |
| libsqlite3-dev | Library to interact with the SQLite3 databases      |
| libssl-dev     | Used internally for the https-client                |
| pkg-config     | Necessary for the build-process to find the ssl-lib |

#### Rust-version

Minimum version: **1.85.1**

### Runtime

| apt-package  | Purpose                                        |
| ------------ | ---------------------------------------------- |
| libsqlite3-0 | Library to interact with the SQLite3 databases |
| openssl      | Used internally for the https-client           |

## Python-SDK

### Packages

see
[requirements.txt](https://github.com/kitsudaiki/ainari/blob/develop/src/sdk/python/ainari_sdk/requirements.txt)

### Suppored Python-versions

| Python (SDK)                                |
| ------------------------------------------- |
| [![python-3_10][img_python-3_10]][workflow] |
| [![python-3_11][img_python-3_11]][workflow] |
| [![python-3_12][img_python-3_12]][workflow] |

## Go CLI-client

see [go.sum](https://github.com/kitsudaiki/ainari/blob/develop/src/cli/ainarictl/go.sum)

[img_python-3_10]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/Ainari-badges/develop/python_version/python-3_10/shields.json&style=flat-square
[img_python-3_11]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/Ainari-badges/develop/python_version/python-3_11/shields.json&style=flat-square
[img_python-3_12]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/Ainari-badges/develop/python_version/python-3_12/shields.json&style=flat-square
[workflow]: https://github.com/kitsudaiki/ainari/actions/workflows/build_test.yml
