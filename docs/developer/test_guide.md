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
| `host_isolation`   | isolate an unused sakura-host, normal virtual machine avoids it, isolated virtual machine binds it to the project, change of a host in use rejected, deleting the last virtual machine releases it, unknown host → 404 |
| `virtual_machines` | reserve, host-resources allocated, oversized virtual machine rejected, create → `RUNNING`, get/list/count |
| `proxies`          | list/get proxies, sakura reachable through each proxy-port, tasks listed through the proxy    |
| `floating_ips`     | create+attach and attach separately, get/list, duplicate/out-of-range rejected, detach and re-attach |
| `ssh`              | ssh into each virtual machine, checks number of cores, memory, disk-size, internal ip, writable root-filesystem |
| `networking`       | ping and tcp between the virtual machines, default-route, a detached floating ip stops answering, swapping floating ip-addresses between virtual machines (checked via machine-id) |
| `network_filters`  | invalid filter-requests rejected, add/get/list/remove of ip-ranges and ports incl. canonical notation, ingress ip-ranges and ports and egress ip-ranges block and allow the traffic between the virtual machines, filters of deleted virtual machines are gone |
| `users`            | second user: no admin-access, can't see or use the virtual machines, network or floating ip-addresses of the other user; deleted afterwards |
| `power`            | reboot (new boot-id), stop (unreachable), stop twice, other virtual machines unaffected, start, start while running |
| `snapshots`        | write a marker-file → save snapshot → listed as snapshot-image → change the disk → restore → marker is back and the later file is gone; invalid restores rejected |
| `tasks`            | every task created during the run is `Finished`, has consistent timestamps and shows up in the list |
| `cleanup`          | deletes everything and checks, that it is gone, including the release of the host-resources  |

Tests, which require two virtual machines, are skipped, if the test runs with only one.

The tests of `host_isolation` are skipped, if every sakura-host already runs virtual machines,
because the isolation can only be changed on a host without them.

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

## Verify the encryption between two virtual machines

The traffic between two virtual machines of a network, which run on different sakura-hosts, is
encrypted with IPsec (see
[Key-exchange of the network-encryption](sequence_diagrams/network_crypto_key_exchange.md)). These
steps check it in the docker-compose setup (`make up local`) with the two virtual machines of
`prepare_resources.py`. Below, virtual machine 1 runs behind `torii-vmm` (`172.30.0.20`) and
virtual machine 2 behind `torii-vmm-2` (`172.30.0.21`). All commands run in the root of the
repository.

1. Check, that the virtual machines run on different hosts:

    ```bash
    docker exec torii-vmm   ip -br link | grep tap-
    docker exec torii-vmm-2 ip -br link | grep tap-
    ```

    Each gateway has to show one `tap-…`. If both are on the same gateway, nothing is encrypted:
    run `prepare_resources.py` again, hanami picks the host of every virtual machine anew.

2. Get the internal addresses of the virtual machines:

    ```bash
    .venv/bin/python - <<'EOF'
    import sys; sys.path.insert(0, "src/sdk/python/ainari_sdk")
    from ainari_sdk import login, floating_ip
    ctx = login.request_context("http://127.0.0.1:11417", "asdf", "asdfasdf", verify_connection=False)
    for f in floating_ip.list_floating_ips(ctx)["floating_ips"]:
        print(f["floating_ip"], "->", f["internal_ip"])
    EOF
    ```

    The examples below use `10.0.0.2 -> 192.168.100.2` and `10.0.0.3 -> 192.168.100.3`.

3. Define a helper for the ssh-commands into virtual machine 1. A function is used, because zsh
    doesn't split a command, which is stored in a variable, into its words:

    ```bash
    vm1() { ssh -i temporary_files/local_stack_test/id_ed25519 -o StrictHostKeyChecking=no \
            -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR ubuntu@10.0.0.2 "$@"; }
    ```

4. Check, that the MLS-group of the network was created:

    ```bash
    docker logs torii-vmm   2>&1 | grep -E "MLS-group|key-packages"
    docker logs torii-vmm-2 2>&1 | grep -E "MLS-group|key-packages"
    docker logs izakaya     2>&1 | grep -E "MLS-group|Rotate the keys"
    ```

    One gateway logs `Created the MLS-group of tenant <vni>`, the other one
    `Joined the MLS-group of tenant <vni>` and izakaya `Rotate the keys of tenant <vni>`. The
    components log with the level of `RUST_LOG` (`info` by default). Step 5 shows the same result
    in the kernel, if the logs are not available.

5. Check the policies and keys (SAs) on both gateways:

    ```bash
    for c in torii-vmm torii-vmm-2; do echo "== $c"; docker exec $c ip xfrm policy | grep -E "^src|dir|proto esp"; done
    for c in torii-vmm torii-vmm-2; do echo "== $c"; docker exec $c ip xfrm state | grep -E "^src|proto esp|aead"; done
    ```

    Every gateway has a `dir out` policy with `tmpl … proto esp spi 0x8…` (the derived SPIs always
    have their highest bit set), `dir in` and `dir fwd` policies and `action block` policies as
    fallback. There are two SAs of the type `aead rfc4106(gcm(aes))` on every gateway, and the SPI,
    which the one gateway sends with, is the one, which the other one receives with.

    !!! warning

        `ip xfrm state` prints the keys. This is fine in the local setup, but its output must not
        be shared.

6. Check the routing-rules and the reverse-path filter:

    ```bash
    for c in torii-vmm torii-vmm-2; do echo "== $c"; docker exec $c sh -c \
      'ip rule; echo "rp_filter all=$(cat /proc/sys/net/ipv4/conf/all/rp_filter) eth0=$(cat /proc/sys/net/ipv4/conf/eth0/rp_filter) tap=$(cat /proc/sys/net/ipv4/conf/tap-*/rp_filter)"'; done
    ```

    `torii-vmm` shows the following, `torii-vmm-2` the same with swapped addresses:

    ```text
    1000: from 192.168.100.2 iif tap-00000001 lookup 101
    1001: from all iif tap-00000001 unreachable
    1100: from 192.168.100.3 to 192.168.100.2 lookup 101
    1101: from 192.168.100.3 to 192.168.100.2 unreachable
    rp_filter all=0 eth0=2 tap=0
    ```

    The rules `1000` and `1001` let only the address of the virtual machine into the table of its
    tenant, the rules `1100` and `1101` deliver the decrypted packets to the virtual machine instead
    of the default route. Without the rules or with a `tap` other than `0`, the encrypted traffic
    is dropped or leaves the gateway in clear.

7. Send traffic from virtual machine 1 to virtual machine 2:

    ```bash
    vm1 "ping -c 10 192.168.100.3"
    vm1 "timeout 3 bash -c 'head -c 30 </dev/tcp/192.168.100.3/22'"
    ```

    The ping has `0% packet loss` and the second command prints the banner
    `SSH-2.0-OpenSSH_…` of virtual machine 2.

8. Check, that the traffic went through the SAs. Run this before and after step 7:

    ```bash
    for c in torii-vmm torii-vmm-2; do docker exec $c ip -s xfrm state \
      | awk -v c=$c '/^src/{s=$0} /proto esp/{spi=$4} /packets\)$/&&/bytes/{print c": "s" spi "spi" ->"$0}'; done
    ```

    The packet-counters increase, and every SPI has the same count on both gateways: sent on the
    one, received on the other one.

9. Check, that only ESP crosses the underlay. The image of torii has no `tcpdump`, so it runs on
    the host within the network-namespace of the gateway. Start each capture, while the ping of
    step 7 runs:

    ```bash
    PID=$(docker inspect -f '{{.State.Pid}}' torii-vmm-netns)
    sudo nsenter -t $PID -n tcpdump -ni eth0 'host 172.30.0.21 and esp'            # ESP in both directions
    sudo nsenter -t $PID -n tcpdump -ni eth0 'host 172.30.0.21 and udp port 5555'  # nothing
    sudo nsenter -t $PID -n tcpdump -ni eth0 'icmp and net 192.168.100.0/24'        # nothing
    ```

    Between `172.30.0.20` and `172.30.0.21` there are only ESP-packets with the SPIs of step 5,
    no unencrypted overlay-traffic (UDP `5555`) and no addresses of the virtual machines in clear.

    !!! info

        The ssh-session itself appears as UDP `5555` from and to `172.30.0.10`. This is expected:
        the traffic towards the gateway at the edge of the network is never encrypted.

10. Check, that no decrypted packet leaves the receiving gateway in clear:

    ```bash
    PID2=$(docker inspect -f '{{.State.Pid}}' torii-vmm-2-netns)
    sudo nsenter -t $PID2 -n tcpdump -ni eth0 -e 'icmp and net 192.168.100.0/24'
    ```

    The only packets are the ones with the MAC-address of `eth0` of `torii-vmm-2` itself as
    destination (`docker exec torii-vmm-2 cat /sys/class/net/eth0/address`), which the kernel
    passes to itself after the decryption. There must be no packet towards the docker-gateway
    `172.30.0.1`.

11. Optional: check, that a forged source-address is not encrypted:

    ```bash
    vm1 "sudo ip addr add 192.168.100.99/32 dev ens4; ping -c 3 -W 1 -I 192.168.100.99 192.168.100.3; sudo ip addr del 192.168.100.99/32 dev ens4"
    ```

    The ping has `100% packet loss` and the counter of the outgoing SA of step 8 doesn't change.

12. Optional: compare with an unencrypted network. Create a network with
    `ainarictl network create --disable-encryption …` and two virtual machines in it on different
    hosts, and repeat the steps 5, 7 and 9. The virtual machines have no SAs, and their traffic
    crosses the underlay as UDP `5555`.

!!! info "Kind-setup"

    In the kind-setup, the gateways are the containers `torii` of the pods `sakura-0` and
    `sakura-1`, for example `kubectl -n ainari exec sakura-0 -c torii -- ip xfrm state`. Their
    underlay-addresses are the addresses of the pods. For the captures, the network-namespace of a
    pod is found with `crictl` within the node-container `ainari-control-plane`.
