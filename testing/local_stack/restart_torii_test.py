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
Restart-test of the gateways of the local docker-compose setup.

Every torii persists the routes, packet-filters, floating ip-addresses, interfaces, TAP-devices
and proxies, which are configured over its api, and restores them, when it starts again. This
test restarts the gateways one after another and checks, that nothing got lost:

    0. every virtual machine, which the gateway has a route to, gets packet-filters, which allow
       everything, so the restore of the filters is covered as well, without blocking any traffic
    1. the state of the gateway is taken: its routes, packet-filters and proxies over its api,
       and its TAP-devices, XDP-attachments, policy-rules, routes and permanent neighbours from
       the kernel
    2. every virtual machine is reachable over ssh on its floating ip-address and reaches all
       other virtual machines on their internal addresses
    3. the container of the gateway is restarted. Its network-namespace is owned by a separate
       holder-container, so the network itself survives the restart, but the eBPF-datapath and the
       whole state of the torii start from scratch.
    4. step 1 and 2 are repeated and the state has to be the same as before

The stack has to run and the virtual machines have to exist before this script is started:

    sudo ./scripts/setup_local_stack.sh
    python3 testing/local_stack/prepare_resources.py
    python3 testing/local_stack/restart_torii_test.py [gateway ...]

Without arguments the gateways torii-public, torii-vmm and torii-vmm-2 are restarted.
"""

import json
import os
import sqlite3
import subprocess
import sys
import tempfile
import time

import requests

REPO_DIR = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# the addresses and credentials of the local docker-compose setup
MIKO_ADDRESS = os.getenv("AINARI_MIKO_ADDRESS", "http://127.0.0.1:11417")
USER_ID = os.getenv("AINARI_USER", "asdf")
PASSPHRASE = os.getenv("AINARI_PASSPHRASE", "asdfasdf")
INTERNAL_API_KEY = os.getenv("INTERNAL_API_KEY", "local-internal-api-key")

# the gateways and the address of their internal api. The internal port is not published, so it
# is reached directly over the docker-network.
GATEWAYS = {
    "torii-public": "http://172.30.0.10:10419",
    "torii-vmm": "http://172.30.0.20:10419",
    "torii-vmm-2": "http://172.30.0.21:10419",
}
# the gateway, which holds the floating ip-addresses of all virtual machines
EDGE_GATEWAY = "torii-public"
DB_PATH = "/etc/ainari/torii_db"

# the key, which prepare_resources.py generated and deployed into the virtual machines
SSH_KEY_PATH = os.path.join(REPO_DIR, "temporary_files", "local_stack_test", "id_ed25519")
VM_USER = "ubuntu"

# a restarted gateway needs a moment, before its api answers again, and the first packets after
# the restart may need a fresh arp-resolution
RESTART_TIMEOUT = 120
SSH_TIMEOUT = 120


def log(message: str):
    print(f"[restart-test] {message}", flush=True)


def request_token() -> str:
    """
    Logs in at miko and returns the token for the api of the gateways.
    """
    body = (f"token_format=jwt&grant_type=client_credentials"
            f"&client_id={USER_ID}&client_secret={PASSPHRASE}")
    resp = requests.post(f"{MIKO_ADDRESS}/v1alpha/token", data=body, timeout=10)
    resp.raise_for_status()
    return resp.json()["access_token"]


def api_get(gateway: str, token: str, path: str) -> dict:
    """
    Sends a GET-request to the api of a gateway and returns the json-response.
    """
    headers = {
        "Authorization": f"Bearer {token}",
        "X-Internal-API-Key": INTERNAL_API_KEY,
    }
    resp = requests.get(f"{GATEWAYS[gateway]}/v1alpha{path}", headers=headers, timeout=10)
    resp.raise_for_status()
    return resp.json()


def api_call(method: str, gateway: str, token: str, path: str, body: dict = None) -> dict:
    """
    Sends a modifying request to the internal api of a gateway and returns the json-response.
    """
    headers = {
        "Authorization": f"Bearer {token}",
        "X-Internal-API-Key": INTERNAL_API_KEY,
    }
    resp = requests.request(method, f"{GATEWAYS[gateway]}/v1alpha{path}", headers=headers,
                            json=body, timeout=10)
    resp.raise_for_status()
    return resp.json() if resp.content else {}


# include-list, which allows every address, so the test-filter blocks nothing
ALLOW_ALL = "0.0.0.0/0"


def filter_path(vni: int, ip: str, direction: str) -> str:
    """
    Returns the path of the packet-filter of one direction of an address within a tenant.
    """
    return f"/network_filter/{vni}/{ip}/{direction}"


def add_test_filters(gateway: str, token: str, internal_ips: list) -> list:
    """
    Adds packet-filters, which allow everything, to every address of a virtual machine, which the
    gateway has a route to and which has no filter yet. Every such address gets an ingress-filter,
    the ones behind a TAP-device of the gateway an egress-filter as well.

    Returns the tenant, the address and the direction of every filter, which was added.
    """
    filters = api_get(gateway, token, "/network_filter/internal")["filters"]
    filtered = {(entry["vni"], entry["ip"], entry["direction"]) for entry in filters}
    added = []
    for route in api_get(gateway, token, "/route/internal")["routes"]:
        if route["dest_ip"] not in internal_ips:
            continue
        for direction in ("ingress", "egress"):
            key = (route["vni"], route["dest_ip"], direction)
            if key in filtered or key in added:
                continue
            try:
                api_call("POST", gateway, token, filter_path(*key) + "/ip_range/internal",
                         {"ranges": [ALLOW_ALL]})
            except requests.HTTPError as error:
                # only the addresses behind a TAP-device of the gateway have an egress-filter
                if direction == "egress" and error.response.status_code == 404:
                    continue
                raise
            added.append(key)
    return added


def remove_test_filters(gateway: str, token: str, filter_keys: list):
    """
    Drops the packet-filters again, which were added by the test.
    """
    for key in filter_keys:
        api_call("DELETE", gateway, token, filter_path(*key) + "/internal")


def docker_exec(container: str, command: str) -> str:
    """
    Runs a shell-command in a container and returns its output.
    """
    result = subprocess.run(["docker", "exec", container, "sh", "-c", command],
                            capture_output=True, text=True, check=True)
    return result.stdout


def read_database(gateway: str) -> dict:
    """
    Copies the database out of a gateway and returns its active entries, table by table.
    """
    with tempfile.TemporaryDirectory() as tmp_dir:
        db_file = os.path.join(tmp_dir, "torii_db")
        subprocess.run(["docker", "cp", f"{gateway}:{DB_PATH}", db_file],
                       capture_output=True, check=True)
        conn = sqlite3.connect(db_file)
        queries = {
            "floating_ips": "SELECT floating_ip, internal_ip, vni FROM floating_ips",
            "routes": "SELECT uuid, vni, dest_ip, target_iface FROM routes",
            "taps": "SELECT tap_name, vni, vm_ip, vm_mac FROM taps",
            "proxys": "SELECT uuid, port, target_address FROM proxys",
        }
        tables = {}
        for table, query in queries.items():
            rows = conn.execute(f"{query} WHERE status = 'ACTIVE'").fetchall()
            tables[table] = sorted(tuple(str(col) for col in row) for row in rows)
        conn.close()
    return tables


def kernel_state(gateway: str) -> dict:
    """
    Returns the parts of the kernel-state of a gateway, which torii programs and restores.

    The ids of the XDP-programs change with every load, so only whether an interface carries one
    is compared.
    """
    links = []
    for line in docker_exec(gateway, "ip -o link show").splitlines():
        name = line.split(":")[1].strip().split("@")[0]
        has_xdp = "xdp" in line
        links.append(f"{name} xdp={has_xdp}")

    def lines(command: str) -> list:
        return sorted(line.strip() for line in docker_exec(gateway, command).splitlines()
                      if line.strip())

    return {
        "links": sorted(links),
        "rules": lines("ip rule show"),
        "routes": lines("ip -4 route show table all | grep -v 'table local'"),
        "neighbours": lines("ip neigh show nud permanent"),
    }


def gateway_state(gateway: str, token: str) -> dict:
    """
    Returns the whole state of a gateway, which has to survive a restart.
    """
    routes = api_get(gateway, token, "/route/internal")["routes"]
    filters = api_get(gateway, token, "/network_filter/internal")["filters"]
    proxies = api_get(gateway, token, "/proxy").get("proxys", [])
    return {
        "routes": sorted(json.dumps(route, sort_keys=True) for route in routes),
        "filters": sorted(json.dumps(entry, sort_keys=True) for entry in filters),
        "proxies": sorted(json.dumps({key: proxy[key] for key in
                                      ("uuid", "port", "target_address")}, sort_keys=True)
                          for proxy in proxies),
        "kernel": kernel_state(gateway),
    }


def run_over_ssh(floating_ip_address: str, command: str) -> str:
    """
    Runs a command in the virtual machine over ssh and returns its output.
    """
    ssh_command = [
        "ssh",
        "-i", SSH_KEY_PATH,
        "-o", "StrictHostKeyChecking=no",
        "-o", "UserKnownHostsFile=/dev/null",
        "-o", "ConnectTimeout=5",
        "-o", "LogLevel=ERROR",
        f"{VM_USER}@{floating_ip_address}",
        command,
    ]
    result = subprocess.run(ssh_command, capture_output=True, text=True, check=True, timeout=60)
    return result.stdout.strip()


def check_connectivity(floating_ips: list):
    """
    Checks, that every virtual machine is reachable over ssh on its floating ip-address and
    reaches all other virtual machines on their internal addresses.
    """
    internal_ips = [internal_ip for _, internal_ip, _ in floating_ips]
    for floating_ip, internal_ip, _ in floating_ips:
        end_time = time.time() + SSH_TIMEOUT
        last_error = ""
        while True:
            try:
                run_over_ssh(floating_ip, "true")
                break
            except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as error:
                last_error = getattr(error, "stderr", "") or str(error)
            if time.time() > end_time:
                raise AssertionError(f"no ssh-access to {floating_ip} within {SSH_TIMEOUT}s: "
                                     f"{last_error.strip()}")
            time.sleep(3.0)

        for peer_ip in internal_ips:
            if peer_ip == internal_ip:
                continue
            run_over_ssh(floating_ip, f"ping -c 3 -W 2 {peer_ip} > /dev/null")
        log(f"    {floating_ip} ({internal_ip}) is reachable and reaches "
            f"{len(internal_ips) - 1} other virtual machine(s)")


def restart_gateway(gateway: str, token: str):
    """
    Restarts the container of a gateway and waits, until its api answers again.
    """
    subprocess.run(["docker", "restart", gateway], capture_output=True, check=True)
    end_time = time.time() + RESTART_TIMEOUT
    while time.time() < end_time:
        try:
            api_get(gateway, token, "/route/internal")
            return
        except requests.RequestException:
            time.sleep(2.0)
    raise TimeoutError(f"{gateway} didn't come back within {RESTART_TIMEOUT}s")


def compare(name: str, before, after) -> list:
    """
    Compares two parts of the state and returns the differences in a readable form.
    """
    if isinstance(before, dict):
        diffs = []
        for key in sorted(set(before) | set(after)):
            diffs += compare(f"{name}.{key}", before.get(key), after.get(key))
        return diffs

    if before == after:
        return []
    before, after = set(before or []), set(after or [])
    return ([f"{name}: lost     {entry}" for entry in sorted(before - after)]
            + [f"{name}: appeared {entry}" for entry in sorted(after - before)])


def main() -> int:
    gateways = sys.argv[1:] or list(GATEWAYS)
    unknown = [gateway for gateway in gateways if gateway not in GATEWAYS]
    if unknown:
        log(f"unknown gateway(s): {', '.join(unknown)}")
        return 1

    token = request_token()

    floating_ips = read_database(EDGE_GATEWAY)["floating_ips"]
    if not floating_ips:
        log("no floating ip-addresses found. Run prepare_resources.py first.")
        return 1
    log(f"virtual machines: {', '.join(f'{fip} -> {ip}' for fip, ip, _ in floating_ips)}")

    log("checking the connectivity before the restarts")
    check_connectivity(floating_ips)

    internal_ips = [internal_ip for _, internal_ip, _ in floating_ips]
    failed = []
    for gateway in gateways:
        log(f"=== {gateway}")
        test_filters = add_test_filters(gateway, token, internal_ips)
        log(f"    added {len(test_filters)} test-filter(s)")
        database = read_database(gateway)
        log("    persisted: " + ", ".join(f"{len(rows)} {table}"
                                          for table, rows in database.items()))
        before = gateway_state(gateway, token)
        log(f"    before the restart: {len(before['routes'])} route(s), "
            f"{len(before['filters'])} filter(s), {len(before['proxies'])} proxy(s)")

        restart_gateway(gateway, token)
        log(f"    {gateway} restarted")

        try:
            check_connectivity(floating_ips)
        except AssertionError as error:
            log(f"    FAILED: {error}")
            failed.append(gateway)

        after = gateway_state(gateway, token)
        diffs = compare(gateway, before, after)
        if diffs:
            log("    FAILED: the state differs after the restart:")
            for diff in diffs:
                log(f"        {diff}")
            failed.append(gateway)
        else:
            log("    the state is the same as before the restart")

        remove_test_filters(gateway, token, test_filters)

    if failed:
        log(f"FAILED: {', '.join(sorted(set(failed)))}")
        return 1

    log("SUCCESS: all gateways restored their state and the virtual machines stayed reachable")
    return 0


if __name__ == "__main__":
    sys.exit(main())
