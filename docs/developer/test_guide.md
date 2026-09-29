# How to test

## Prepare test environment for manually tests

`testing/local_stack/prepare_resources.py` prepares virtual machines for testing
with the Python-SDK: it uploads a public key and an ubuntu-cloud-image, creates a network, two
virtual machines and their floating ip-addresses and logs into them over ssh.

- Prepare a python-environment with the dependencies of the SDK once in the root of the repository:

    ```bash
    python3 -m venv .venv
    .venv/bin/pip install -r src/sdk/python/ainari_sdk/requirements.txt
    ```

- Run the test against the running stack:

    ```bash
    AINARI_SAKURA_HOSTS=1 AINARI_SSH_NETNS=torii-outside \
        .venv/bin/python testing/local_stack/prepare_resources.py
    ```

    | Variable                  | Description                                                                                    |
    | ------------------------- | ---------------------------------------------------------------------------------------------- |
    | `AINARI_SAKURA_HOSTS`     | number of sakura-hosts, which the test waits for. This setup has only one, the default is 2.   |
    | `AINARI_SSH_NETNS`        | network-namespace, in which the ssh-commands run, because only there the floating ip-addresses are reachable. The test asks for the sudo-password for this at its start. |
    | `AINARI_VIRTUAL_MACHINES` | number of virtual machines (default 2, each with 2 cores and 2 GiB memory)                     |
    | `AINARI_MIKO_ADDRESS`     | address of miko (default `http://127.0.0.1:11417`)                                             |
    | `AINARI_USER`, `AINARI_PASSPHRASE` | login (default `asdf` / `asdfasdf`)                                                   |

- The ubuntu-cloud-image and the ssh-key are stored in `temporary_files/local_stack_test`. The
    image is only downloaded with the first run. At the end, the test prints the ssh-commands to
    log into the virtual machines.

!!! info

    The script doesn't delete its resources, so they stay for further manual tests. Every run
    creates new ones with a new id in their names.

## Test complete life-cycle

`testing/ainari_test/vm_lifecycle_test.py` tests the complete setup automatically end-to-end with
the Python-SDK. It creates all resources, which it needs, by itself, walks through the whole
life-cycle of the virtual machines, checks every step, including the expected errors of invalid
requests, and deletes all of its resources at the end again.

The test is built on a small test-framework in `testing/ainari_test`:

| File / Directory       | Content                                                                                      |
| ---------------------- | -------------------------------------------------------------------------------------------- |
| `vm_lifecycle_test.py` | entry-point, which selects and runs the suites and prints the summary                        |
| `framework.py`         | suites, tests, the state shared between the tests, the cleanup-registry and the runner       |
| `config.py`            | settings of the tested setup, which can be overwritten by environment-variables             |
| `checks.py`            | assertions with readable error-messages                                                      |
| `waiting.py`           | polling of asynchronous states, like tasks and states of virtual machines                    |
| `ssh.py`               | ssh-key-pair and commands within the virtual machines                                        |
| `suites/`              | the tests, one file per topic. The order of the suites is defined in `suites/__init__.py`.  |

The tests share their resources, like the virtual machines, over a common state. If a test fails,
the tests, which depend on its resources, are skipped instead of failing with follow-up errors.
Every created resource is registered for the cleanup, so it is deleted at the end, even if the
run was aborted.

### Covered tests

| Suite              | Checks                                                                                        |
| ------------------ | --------------------------------------------------------------------------------------------- |
| `auth`             | validate/renew token, wrong passphrase, invalid token, endpoints, component-versions, quota   |
| `hosts`            | sakura-hosts registered, get/list, reported resources, unknown host → 404, onsen-hosts        |
| `resources`        | public key upload/get/list plus an invalid key, image upload/get/list/count, network, secrets |
| `virtual_machines` | reserve, host-resources allocated, oversized virtual machine rejected, create → `RUNNING`, get/list/count |
| `proxies`          | list/get proxies, sakura reachable through each proxy-port, tasks listed through the proxy    |
| `floating_ips`     | create+attach and attach separately, get/list, duplicate/out-of-range rejected, detach and re-attach |
| `ssh`              | ssh into each virtual machine, checks number of cores, memory, disk-size, internal ip, writable root-filesystem |
| `networking`       | ping and tcp between the virtual machines, default-route, a detached floating ip stops answering, swapping floating ip-addresses between virtual machines (checked via machine-id) |
| `users`            | second user: no admin-access, can't see or use the virtual machines, network or floating ip-addresses of the other user; deleted afterwards |
| `power`            | reboot (new boot-id), stop (unreachable), stop twice, other virtual machines unaffected, start, start while running |
| `snapshots`        | write a marker-file → save snapshot → listed as snapshot-image → change the disk → restore → marker is back and the later file is gone; invalid restores rejected |
| `tasks`            | every task created during the run is `Finished`, has consistent timestamps and shows up in the list |
| `cleanup`          | deletes everything and checks, that it is gone, including the release of the host-resources  |

Tests, which require two virtual machines, are skipped, if the test runs with only one.

### Usage

- Prepare a python-environment with the dependencies of the SDK once in the root of the repository:

    ```bash
    python3 -m venv .venv
    .venv/bin/pip install -r src/sdk/python/ainari_sdk/requirements.txt
    ```

- Run all tests against the running stack of this single-node setup:

    ```bash
    AINARI_SAKURA_HOSTS=1 AINARI_SSH_NETNS=torii-outside \
        .venv/bin/python testing/ainari_test/vm_lifecycle_test.py
    ```

    `AINARI_SAKURA_HOSTS` and `AINARI_SSH_NETNS` are only required for this setup. Setups, whose
    floating ip-addresses are reachable from the host directly, like the
    [docker-compose-setup](local_testing/docker_compose_setup.md), run without them:

    ```bash
    .venv/bin/python testing/ainari_test/vm_lifecycle_test.py
    ```

- Options:

    | Option               | Description                                                                             |
    | -------------------- | --------------------------------------------------------------------------------------- |
    | `--list`             | list all suites and their tests                                                         |
    | `--suite <NAME>`     | run only this suite and the suites, which it depends on (repeatable)                    |
    | `--skip <NAME>`      | skip this suite (repeatable)                                                            |
    | `--fail-fast`        | stop at the first failure. The cleanup runs anyway.                                     |
    | `--keep`             | keep all created resources and print the ssh-commands to log into the virtual machines |
    | `--junit <FILE>`     | write the results as junit-xml, for example for a CI-pipeline                           |
    | `--no-color`         | disable colored output                                                                  |

    For example, only the snapshots together with the suites, which they require:

    ```bash
    .venv/bin/python testing/ainari_test/vm_lifecycle_test.py --suite snapshots
    ```

- The settings are read from environment-variables:

    | Variable                           | Description                                                          |
    | ---------------------------------- | -------------------------------------------------------------------- |
    | `AINARI_SAKURA_HOSTS`              | number of sakura-hosts, which the test waits for. This setup has only one, the default is 2. |
    | `AINARI_SSH_NETNS`                 | network-namespace, in which the ssh-commands run, because only there the floating ip-addresses are reachable. The test asks for the sudo-password for this at its start and stops immediately, if the namespace doesn't exist. Unset, if the floating ip-addresses are reachable from the host. |
    | `AINARI_VIRTUAL_MACHINES`          | number of virtual machines (default 2, each with 2 cores and 2 GiB memory) |
    | `AINARI_MIKO_ADDRESS`              | address of miko (default `http://127.0.0.1:11417`)                   |
    | `AINARI_USER`, `AINARI_PASSPHRASE` | login (default `asdf` / `asdfasdf`)                                  |
    | `AINARI_NETWORK_SUBNET`            | subnet of the network of the test (default `192.168.100.1/24`)       |
    | `AINARI_IMAGE_URL`                 | cloud-image of the virtual machines (default ubuntu noble)           |
    | `AINARI_TEST_WORK_DIR`             | directory for image and ssh-key (default `temporary_files/ainari_test`) |
    | `AINARI_HOST_REGISTRATION_TIMEOUT` | seconds to wait for the registration of the hosts (default 300)      |
    | `AINARI_VM_CREATE_TIMEOUT`         | seconds to wait for the creation of a virtual machine (default 600)  |
    | `AINARI_SSH_TIMEOUT`               | seconds to wait for the ssh-access (default 300)                     |
    | `AINARI_TASK_TIMEOUT`              | seconds to wait for a task, like a snapshot (default 600)            |
    | `AINARI_DELETE_TIMEOUT`            | seconds to wait for a deletion (default 180)                         |

- The ubuntu-cloud-image is only downloaded with the first run. A new ssh-key is generated with
    every run.

- The exit-code is `0`, if all tests passed or were skipped, `1`, if a test failed, and `2`, if
    the login or the selection of the suites failed.

!!! info "New tests"

    A new test is a function in one of the files of `testing/ainari_test/suites`, which is
    decorated with `@suite.test(...)`. With `requires` it names the entries of the shared state,
    which it needs, and with `provides` the ones, which it creates. A new suite is a new file with
    a `suite = Suite(...)`, which is added to the list in `suites/__init__.py`.
