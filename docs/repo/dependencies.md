# Dependencies

## Backend

### Build hanami

| apt-package         | Purpose                                                                              |
| ------------------- | ------------------------------------------------------------------------------------ |
| libsqlite3-dev      | Library to interact with the SQLite3 databases                                       |

### Runtime

| apt-package      | Purpose                                                                               |
| ---------------- | ------------------------------------------------------------------------------------- |
| libsqlite3-0     | Library to interact with the SQLite3 databases                                        |

## Python-SDK

### Packages

see
[requirements.txt](https://github.com/kitsudaiki/ainari/blob/develop/src/sdk/python/ainari_sdk/requirements.txt)

### Suppored Python-versions

| Python (SDK)                                |
| ------------------------------------------- |
| [![python-3_10][img_python-3_10]][Workflow] |
| [![python-3_11][img_python-3_11]][Workflow] |
| [![python-3_12][img_python-3_12]][Workflow] |

## Go CLI-client

see [go.sum](https://github.com/kitsudaiki/ainari/blob/develop/src/cli/ainarictl/go.sum)

[img_python-3_10]:
    https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/Ainari-badges/develop/python_version/python-3_10/shields.json&style=flat-square
[img_python-3_11]:
    https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/Ainari-badges/develop/python_version/python-3_11/shields.json&style=flat-square
[img_python-3_12]:
    https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/kitsudaiki/Ainari-badges/develop/python_version/python-3_12/shields.json&style=flat-square
[Workflow]: https://github.com/kitsudaiki/ainari/actions/workflows/build_test.yml
