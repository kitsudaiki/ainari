# Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#    http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""
End-to-end test of the local setup.

It runs the suites of ./suites against a running stack:

    auth              login, tokens, endpoints, versions and quota
    hosts             registration and resources of the sakura- and onsen-hosts
    resources         public key, cloud-image, network and secrets
    host_isolation    isolated sakura-hosts, which are only used by a single project
    virtual_machines  reserve and create the virtual machines on their sakura-hosts
    proxies           proxies of the torii towards the sakura-hosts
    floating_ips      create, attach, detach, get and list floating ip-addresses
    ssh               ssh-access and the hardware within the virtual machines
    networking        traffic between the virtual machines and moving floating ip-addresses
    network_filters   ingress- and egress-filters of the virtual machines
    users             second user, the isolation between the users and the passphrase-changes
    projects          default-projects, project-roles and project-assignments
    power             reboot, stop and start
    snapshots         save and restore a snapshot of the root-disk
    tasks             the tasks, which were created on the sakura-hosts
    cleanup           delete and verify all created resources

The stack has to run before this script is started:

    sudo ./testing/local_stack/setup_local_stack.sh
    python3 testing/ainari_test/vm_lifecycle_test.py

Examples:

    # list all suites and tests
    python3 testing/ainari_test/vm_lifecycle_test.py --list
    # only the snapshots, together with the suites, which they depend on
    python3 testing/ainari_test/vm_lifecycle_test.py --suite snapshots
    # keep the virtual machines for manual debugging
    python3 testing/ainari_test/vm_lifecycle_test.py --keep

The settings are read from environment-variables, see ./config.py. If the floating
ip-addresses are only reachable within a network-namespace, like in the single-node setup of
VSCode, this is set with AINARI_SSH_NETNS.
"""

import argparse
import os
import sys
import uuid

# The script is located within the package of the framework. Its own directory is replaced by the
# parent-directory, so the package is importable and its modules, like 'ssh' or 'config', don't
# shadow other top-level modules.
sys.path[0] = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

from ainari_test import framework  # noqa: E402
from ainari_test.config import Config  # noqa: E402
from ainari_test.ssh import SshError  # noqa: E402
from ainari_test.suites import ALL_SUITES  # noqa: E402
from ainari_sdk import login  # noqa: E402

import urllib3  # noqa: E402

# The kind-setup serves the api over https with self-signed certificates, which the test doesn't
# verify on purpose, so urllib3 would warn about every single request.
urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)


def parse_arguments():
    parser = argparse.ArgumentParser(description="End-to-end test of the local setup")
    parser.add_argument("--list", action="store_true",
                        help="list all suites and their tests and exit")
    parser.add_argument("--suite", action="append", default=[], metavar="NAME",
                        help="run only this suite and the suites, which it depends on "
                             "(repeatable)")
    parser.add_argument("--skip", action="append", default=[], metavar="NAME",
                        help="skip this suite (repeatable)")
    parser.add_argument("--keep", action="store_true",
                        help="keep all created resources and skip the cleanup")
    parser.add_argument("--fail-fast", action="store_true",
                        help="stop at the first failure and only run the cleanup afterwards")
    parser.add_argument("--junit", metavar="FILE",
                        help="write the results as junit-xml into this file")
    parser.add_argument("--no-color", action="store_true", help="disable colored output")
    return parser.parse_args()


def print_suites(suites: list):
    for suite in suites:
        depends = f" (depends on: {', '.join(suite.depends)})" if suite.depends else ""
        print(f"{suite.name}: {suite.description}{depends}")
        for test in suite.tests:
            print(f"    - {test.name}")


def print_access(ctx):
    virtual_machines = ctx.state.get("virtual_machines", [])
    if not virtual_machines:
        return
    print("\n=== kept virtual machines")
    for entry in virtual_machines:
        if "floating_ip" in entry:
            print(f"  {entry['name']}: {ctx.ssh.login_command(entry['floating_ip'])}")
        else:
            print(f"  {entry['name']}: {entry['uuid']} (no floating ip)")


def main() -> int:
    arguments = parse_arguments()

    try:
        suites = framework.select_suites(ALL_SUITES, arguments.suite, arguments.skip)
    except ValueError as error:
        print(error, file=sys.stderr)
        return 2
    if arguments.list:
        print_suites(suites)
        return 0

    config = Config()
    os.makedirs(config.work_dir, exist_ok=True)
    ctx = framework.Context(config, str(uuid.uuid4())[:8])

    print(f"test-run {ctx.test_id}: {', '.join(suite.name for suite in suites)}")
    if config.ssh_netns:
        try:
            ctx.ssh.check_netns()
        except SshError as error:
            print(error, file=sys.stderr)
            return 2
        print(f"ssh runs within the network-namespace '{config.ssh_netns}', which requires sudo")
        ctx.ssh.refresh_sudo()

    print(f"login as '{config.user_id}' at {config.miko_address}")
    try:
        ctx.api = login.request_context(config.miko_address,
                                        config.user_id,
                                        config.passphrase,
                                        verify_connection=False)
    except Exception as error:  # noqa: BLE001
        print(f"login failed, is the stack running? {type(error).__name__}: {error}",
              file=sys.stderr)
        return 2

    runner = framework.Runner(suites,
                              fail_fast=arguments.fail_fast,
                              keep=arguments.keep,
                              use_color=not arguments.no_color and sys.stdout.isatty())
    try:
        runner.run(ctx)
    except KeyboardInterrupt:
        print("\ninterrupted", file=sys.stderr)
        if not arguments.keep:
            print("removing the created resources ...", file=sys.stderr)
            for error in ctx.cleanup.run():
                print(f"cleanup failed: {error}", file=sys.stderr)
        return 130

    runner.print_summary()
    if arguments.junit:
        runner.write_junit(arguments.junit)
    if arguments.keep:
        print_access(ctx)

    return 0 if runner.successful() else 1


if __name__ == "__main__":
    sys.exit(main())
