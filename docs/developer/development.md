# Development

This document helps to set up a local environment for development, where the whole stack runs
directly on the host out of VSCode or VSCodium, without any container. For the setups with
containers or kubernetes, see [Local test environments](local_testing/local_testing.md).

## Requirements

- Everything to build the services, see [Preparation of the build-guide](build_guide.md#preparation)
- `ip` and `ethtool` (packages `iproute2` and `ethtool`), used by torii and the uplink-script
- `qemu-img` and `cloud-localds` (packages `qemu-utils` and `cloud-image-utils`), used by sakura
- `cloud-hypervisor` and its firmware `CLOUDHV.fd`
    ([releases](https://github.com/cloud-hypervisor/cloud-hypervisor/releases),
    [firmware](https://github.com/cloud-hypervisor/edk2/releases)). Their paths are set in
    `[hypervisor]` of `sakura.toml`, the defaults are `/usr/local/bin/cloud-hypervisor` and
    `/usr/local/share/CLOUDHV.fd`.
- cloud-hypervisor is started by sakura as normal user, but has to attach the virtual machines to
    the TAP-devices, which torii creates as root. For this it needs `CAP_NET_ADMIN`, which has to be
    set again after every update of the binary:

    ```bash
    sudo setcap cap_net_admin+ep /usr/local/bin/cloud-hypervisor
    ```

- the user needs access to `/dev/kvm` (group `kvm`)
- `sudo`, because torii runs as root

## Import into VSCode or VSCodium

- Clone the repository and open its root-directory with `File > Open Folder...`. The
    launch-configurations and tasks in `.vscode` are loaded automatically.

- Install the extensions:

    | Extension                                 | Usage                                          |
    | ----------------------------------------- | ---------------------------------------------- |
    | `rust-lang.rust-analyzer`                 | language-support for rust                      |
    | `vadimcn.vscode-lldb` (CodeLLDB)          | debugger, which is used by the launch-configs  |
    | `ms-python.python`                        | python, for the SDK and the end-to-end test    |
    | `golang.go`                               | go, for the CLI                                |
    | `Vue.volar`                               | vue and typescript, for the dashboard          |

    All of them are also available for VSCodium on [Open VSX](https://open-vsx.org/).

## Prepare /etc/ainari

All services except torii read their config from `/etc/ainari/<service>.toml`. The directory has
to be writable by the user, because the databases are stored there as well. Copy the
example-configs into it:

```bash
sudo mkdir -p /etc/ainari
sudo chown $USER: /etc/ainari
cp example_configs/ainari/{miko,omamori,hanami,ryokan,sakura,onsen}.toml \
   example_configs/ainari/token_key \
   /etc/ainari/
```

Torii reads `example_configs/ainari/torii_single_node.toml` directly from the repository. The
secrets and the admin-user `asdf` with the passphrase `asdfasdf` are set as env-variables in
`.vscode/launch.json` and `.vscode/tasks.json`, so nothing else has to be configured.

!!! info

    The databases are created with the first start. To start again with empty databases, stop the
    stack and delete the `*_db`-files in `/etc/ainari`.

## Run the stack

The stack uses the same ports and the same floating ip-addresses (`10.0.0.0/24`) as the other
local test environments, so stop them first, for example with `make down local`.

- Open the view `Run and Debug` and start one of the compounds:

    | Compound         | Description                                   |
    | ---------------- | --------------------------------------------- |
    | `Ainari Debug`   | debug-builds, which can be debugged           |
    | `Ainari Release` | release-builds, which are faster              |

- The compound starts the task `Run Torii (single-node)` first. Torii requires root, so it runs in
    this task instead of the debugger and asks for the sudo-password in its terminal. It also
    creates the uplink `uplink0` towards the network-namespace `torii-outside`, where the floating
    ip-addresses are served.

- Afterwards miko, omamori, hanami, ryokan, sakura and onsen are started as debug-sessions. Their
    api is reachable on `http://127.0.0.1:<port>`, for example miko on `http://127.0.0.1:11417`.

- Stop the stack with the stop-button of the compound. Torii keeps running, so stop it too by
    terminating its task (`Terminal > Terminate Task...`).

- The uplink and the network-namespace stay after torii was stopped. Starting torii again replaces
    them, so they only have to be removed, when the setup is not needed anymore, with the task
    `remove single-node uplink` or directly:

    ```bash
    sudo ./testing/local_stack/setup_single_node_uplink.sh down
    ```

## Use the stack

- The CLI is built as described in [Build CLI-client](build_guide.md#build-cli-client) and used
    like in the [Example-Workflow](../../user/cli_sdk/example_workflow.md) with:

    ```bash
    export AINARI_ADDRESS=http://127.0.0.1:11417
    export AINARI_USER=asdf
    export AINARI_PASSPHRASE=asdfasdf
    ```

- The dashboard is built and started as described in [Build dashboard](build_guide.md#build-dashboard)
    and connects to miko on `http://localhost:11417`.

- The floating ip-addresses are only reachable from the network-namespace `torii-outside`, not
    from the host itself. So the ssh-login into a virtual machine runs within this namespace:

    ```bash
    sudo ip netns exec torii-outside ssh -i <PRIVATE_KEY> ubuntu@<FLOATING_IP>
    ```
