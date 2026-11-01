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
Checks the encryption of the traffic between two virtual machines of the local docker-compose
setup, which run on different sakura-hosts. It automates the steps of "Verify the encryption
between two virtual machines" in docs/developer/test_guide.md:

    1. find a virtual machine behind torii-vmm and one of the same network behind torii-vmm-2.
       One of them needs a floating ip-address, over which the commands run in it.
    2. the policies of the connection between both: an outgoing one, pinned to the SPI of the
       active key, and incoming ones, which demand ESP
    3. the keys (SAs) of both directions: the SPI, which the one gateway sends with, is the one,
       which the other one receives with
    4. the routing-rules of both virtual machines and the reverse-path filter of their TAP-devices
    5. ping and tcp from the one virtual machine to the other one
    6. the packet-counters of the SAs increase with this traffic and match on both gateways
    7. during the traffic, the underlay carries only ESP between the two gateways, no
       unencrypted overlay-traffic (UDP 5555) and no address of a virtual machine in clear, and
       no traffic of the virtual machines leaves a gateway over ainari-net (eth0). Only the
       traffic between the sakura-hosts is encrypted: the traffic towards the gateway at the edge
       (torii-public) runs as unencrypted overlay-traffic, which is valid and not counted here.
    8. a forged source-address within the virtual machine is not encrypted, but dropped

The stack has to run and the virtual machines have to exist before this script is started:

    sudo ./testing/local_stack/setup_local_stack.sh
    python3 testing/local_stack/prepare_resources.py
    python3 testing/local_stack/verify_encryption_test.py

The captures need root, so the script asks for the password of sudo at its start. The virtual
machines of 'vm_lifecycle_test.py --keep' can be checked as well, with their own ssh-key:

    AINARI_SSH_KEY=temporary_files/ainari_test/id_ed25519 \\
        python3 testing/local_stack/verify_encryption_test.py
"""

import ipaddress
import os
import re
import sqlite3
import subprocess
import sys
import tempfile
import threading
import time

REPO_DIR = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# the gateways in front of the two sakura-hosts and the holder-containers of their
# network-namespaces, see docker-compose.yml
GATEWAYS = ["torii-vmm", "torii-vmm-2"]
NETNS_HOLDERS = {"torii-vmm": "torii-vmm-netns", "torii-vmm-2": "torii-vmm-2-netns"}
# the gateway at the edge, which holds the floating ip-addresses of all virtual machines
EDGE_GATEWAY = "torii-public"
DB_PATH = "/etc/ainari/torii_db"
UNDERLAY_IFACE = "underlay0"
# the docker-network of the underlay, found by the label of docker compose, so the name of the
# compose-project doesn't matter
UNDERLAY_NETWORK_LABEL = "com.docker.compose.network=ainari-underlay"
# the first kernel routing-table of a tenant is tenant_table_base + vni (default of torii)
TENANT_TABLE_BASE = 100
# port of the unencrypted overlay between the gateways
OVERLAY_PORT = 5555

# the key, which prepare_resources.py generated and deployed into the virtual machines
SSH_KEY_PATH = os.getenv("AINARI_SSH_KEY",
                         os.path.join(REPO_DIR, "temporary_files", "local_stack_test",
                                      "id_ed25519"))
VM_USER = "ubuntu"

PING_COUNT = 10
# the captures run a bit longer than the traffic, so they see all of it
CAPTURE_SECONDS = 12
CAPTURE_STARTUP_SECONDS = 2


def log(message: str):
    print(f"[encryption-test] {message}", flush=True)


class Checks:
    """
    Collects the results of the checks, so all of them run, also if one fails.
    """

    def __init__(self):
        self.failed = []

    def check(self, name: str, ok: bool, detail: str = ""):
        suffix = f": {detail}" if detail else ""
        log(f"    {'ok    ' if ok else 'FAILED'} {name}{suffix}")
        if not ok:
            self.failed.append(name)


def docker_exec(container: str, command: str) -> str:
    """
    Runs a command within a container and returns its output.
    """
    result = subprocess.run(["docker", "exec", container, "sh", "-c", command],
                            capture_output=True, text=True, check=True)
    return result.stdout


def read_rows(gateway: str, query: str) -> list:
    """
    Copies the database out of a gateway and returns the rows of a query.
    """
    with tempfile.TemporaryDirectory() as tmp_dir:
        db_file = os.path.join(tmp_dir, "torii_db")
        subprocess.run(["docker", "cp", f"{gateway}:{DB_PATH}", db_file],
                       capture_output=True, check=True)
        conn = sqlite3.connect(db_file)
        rows = conn.execute(query).fetchall()
        conn.close()
    return rows


def underlay_address(gateway: str) -> str:
    """
    Returns the address of a gateway on the underlay.
    """
    output = docker_exec(gateway, f"ip -4 -o addr show {UNDERLAY_IFACE}")
    match = re.search(r"inet (\S+)/", output)
    if not match:
        raise RuntimeError(f"{gateway} has no address on {UNDERLAY_IFACE}")
    return match.group(1)


def running_virtual_machines(gateway: str) -> list:
    """
    Returns the virtual machines, which run behind a gateway, as (vni, internal ip, tap).

    The TAP-devices of deleted virtual machines stay on the gateway, so only the ones with a
    carrier count, to which a running cloud-hypervisor is attached.
    """
    with_carrier = set()
    for line in docker_exec(gateway, "ip -o link show").splitlines():
        match = re.match(r"\d+: (tap-[^:@]+)[:@].*LOWER_UP", line)
        if match:
            with_carrier.add(match.group(1))
    return [(int(vni), vm_ip, tap_name)
            for tap_name, vni, vm_ip in read_rows(gateway, "SELECT tap_name, vni, vm_ip FROM taps")
            if tap_name in with_carrier]


def find_pair():
    """
    Finds two virtual machines of the same network on different gateways, of which the first one
    has a floating ip-address.

    Returns:
        (gateway, internal ip, tap, floating ip) of the first and (gateway, internal ip, tap) of
        the second virtual machine, or None
    """
    floating_ips = {(int(vni), internal_ip): floating_ip
                    for floating_ip, internal_ip, vni in read_rows(
                        EDGE_GATEWAY, "SELECT floating_ip, internal_ip, vni FROM floating_ips")}
    running = {gateway: running_virtual_machines(gateway) for gateway in GATEWAYS}
    for gateway, other_gateway in [GATEWAYS, list(reversed(GATEWAYS))]:
        for vni, vm_ip, tap in running[gateway]:
            floating_ip = floating_ips.get((vni, vm_ip))
            if floating_ip is None:
                continue
            for other_vni, other_ip, other_tap in running[other_gateway]:
                if other_vni == vni:
                    return ((gateway, vm_ip, tap, floating_ip, vni),
                            (other_gateway, other_ip, other_tap))
    return None


def run_over_ssh(floating_ip: str, command: str, check: bool = True) -> subprocess.CompletedProcess:
    """
    Runs a command in a virtual machine over ssh.
    """
    ssh_command = [
        "ssh",
        "-i", SSH_KEY_PATH,
        "-o", "StrictHostKeyChecking=no",
        "-o", "UserKnownHostsFile=/dev/null",
        "-o", "ConnectTimeout=5",
        "-o", "LogLevel=ERROR",
        f"{VM_USER}@{floating_ip}",
        command,
    ]
    return subprocess.run(ssh_command, capture_output=True, text=True, check=check, timeout=120)


def xfrm_blocks(gateway: str, what: str) -> list:
    """
    Splits the output of 'ip xfrm <what>' into its entries. The output contains the keys, so it
    is never logged.
    """
    blocks = []
    for line in docker_exec(gateway, f"ip -s xfrm {what}").splitlines():
        if line.startswith("src "):
            blocks.append(line)
        elif blocks:
            blocks[-1] += "\n" + line
    return blocks


def policies(gateway: str) -> list:
    """
    Returns the policies of a gateway as dicts with the selector, direction, template and SPI.
    """
    result = []
    for block in xfrm_blocks(gateway, "policy"):
        selector = re.match(r"src (\S+) dst (\S+)", block)
        direction = re.search(r"dir (\w+)", block)
        tmpl = re.search(r"tmpl src (\S+) dst (\S+)", block)
        spi = re.search(r"proto esp spi (0x[0-9a-f]+)", block)
        result.append({
            "src": selector.group(1),
            "dst": selector.group(2),
            "dir": direction.group(1) if direction else None,
            "tmpl": (tmpl.group(1), tmpl.group(2)) if tmpl else None,
            "esp": "proto esp" in block,
            "spi": int(spi.group(1), 16) if spi else None,
            "level_required": "level required" in block,
            "block": "action block" in block,
        })
    return result


def states(gateway: str) -> list:
    """
    Returns the SAs of a gateway as dicts with their addresses, SPI, selector, algorithm and
    packet-counter.
    """
    result = []
    for block in xfrm_blocks(gateway, "state"):
        addresses = re.match(r"src (\S+) dst (\S+)", block)
        spi = re.search(r"proto esp spi (0x[0-9a-f]+)", block)
        selector = re.search(r"sel src (\S+) dst (\S+)", block)
        counter = re.search(r"lifetime current:\s*\n\s*(\d+)\(bytes\), (\d+)\(packets\)", block)
        result.append({
            "src": addresses.group(1),
            "dst": addresses.group(2),
            "spi": int(spi.group(1), 16) if spi else None,
            "sel": (selector.group(1), selector.group(2)) if selector else None,
            "aead": "rfc4106(gcm(aes))" in block,
            "tunnel": "mode tunnel" in block,
            "packets": int(counter.group(2)) if counter else 0,
        })
    return result


def find_state(gateway: str, src: str, dst: str, spi: int):
    for state in states(gateway):
        if state["src"] == src and state["dst"] == dst and state["spi"] == spi:
            return state
    return None


def check_connection(checks: Checks, gateway: str, local: dict, remote: dict) -> int:
    """
    Checks the policies of the connection from a virtual machine to another one on its gateway
    and returns the SPI of the active outgoing key, or None.
    """
    local_sel, remote_sel = f"{local['ip']}/32", f"{remote['ip']}/32"
    gateway_policies = policies(gateway)

    outgoing = [policy for policy in gateway_policies
                if policy["src"] == local_sel and policy["dst"] == remote_sel
                and policy["dir"] == "out" and policy["esp"]]
    ok = (len(outgoing) == 1 and outgoing[0]["tmpl"] == (local["underlay"], remote["underlay"])
          and outgoing[0]["spi"] is not None and outgoing[0]["spi"] & 0x80000000 != 0)
    spi = outgoing[0]["spi"] if ok else None
    checks.check(f"{gateway}: outgoing policy {local['ip']} -> {remote['ip']}", ok,
                 f"ESP towards {remote['underlay']}, spi 0x{spi:08x}" if ok else
                 "missing, or not pinned to an SPI with the highest bit set")

    for direction in ["in", "fwd"]:
        incoming = [policy for policy in gateway_policies
                    if policy["src"] == remote_sel and policy["dst"] == local_sel
                    and policy["dir"] == direction and policy["esp"]
                    and policy["tmpl"] == (remote["underlay"], local["underlay"])
                    and policy["level_required"]]
        checks.check(f"{gateway}: '{direction}' policy {remote['ip']} -> {local['ip']} demands ESP",
                     len(incoming) == 1)

    # The encrypted route towards the other virtual machine blocks everything towards it, which
    # isn't covered by the policies of a connection with a key, so nothing leaves in clear.
    blocking = {policy["dir"] for policy in gateway_policies if policy["block"]
                and ((policy["dir"] == "out" and policy["dst"] == remote_sel)
                     or (policy["dir"] == "fwd" and policy["src"] == remote_sel))}
    checks.check(f"{gateway}: fail-closed policies towards {remote['ip']}",
                 blocking == {"out", "fwd"},
                 f"blocking {', '.join(sorted(blocking)) or 'nothing'}")
    return spi


def check_keys(checks: Checks, sender: dict, receiver: dict, spi: int):
    """
    Checks, that the key of a direction exists on both gateways with the same SPI.
    """
    name = f"key {sender['ip']} -> {receiver['ip']} (spi 0x{spi:08x})"
    for gateway in [sender["gateway"], receiver["gateway"]]:
        state = find_state(gateway, sender["underlay"], receiver["underlay"], spi)
        ok = (state is not None and state["aead"] and state["tunnel"]
              and state["sel"] == (f"{sender['ip']}/32", f"{receiver['ip']}/32"))
        checks.check(f"{gateway}: {name}", ok,
                     "aead rfc4106(gcm(aes)), tunnel-mode" if ok else "missing or wrong")


def check_rules(checks: Checks, local: dict, remote: dict):
    """
    Checks the routing-rules of a virtual machine and the reverse-path filter of its TAP-device.
    """
    gateway, tap = local["gateway"], local["tap"]
    table = TENANT_TABLE_BASE + local["vni"]
    rules = docker_exec(gateway, "ip rule")
    expected = [
        rf"from {re.escape(local['ip'])} iif {tap} lookup {table}\b",
        rf"from all iif {tap} unreachable",
        rf"from {re.escape(remote['ip'])} to {re.escape(local['ip'])} lookup {table}\b",
        rf"from {re.escape(remote['ip'])} to {re.escape(local['ip'])} unreachable",
    ]
    missing = [pattern for pattern in expected if not re.search(pattern, rules)]
    checks.check(f"{gateway}: routing-rules of {local['ip']} on {tap}", not missing,
                 f"{len(missing)} rule(s) missing" if missing else "")

    rp_filter = docker_exec(gateway, f"cat /proc/sys/net/ipv4/conf/{tap}/rp_filter").strip()
    checks.check(f"{gateway}: reverse-path filter of {tap} is off", rp_filter == "0",
                 f"rp_filter={rp_filter}")


class Capture:
    """
    Counts the packets of a tcpdump-filter for a fixed time in the background.
    """

    def __init__(self, name: str, command: list, expect_packets: bool):
        self.name = name
        self.expect_packets = expect_packets
        self.count = 0
        self.error = ""
        self.process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                        text=True)
        self.thread = threading.Thread(target=self._read, daemon=True)
        self.thread.start()

    def _read(self):
        # tcpdump writes an empty line, when it is stopped, so only the other lines are packets
        for line in self.process.stdout:
            if line.strip():
                self.count += 1

    def wait(self):
        self.process.wait()
        self.thread.join()
        # tcpdump ends with exit-code 124 of timeout, everything else is an error
        if self.process.returncode not in (0, 124):
            self.error = self.process.stderr.read().strip()


def tcpdump(iface: str, packet_filter: str, netns_pid: str = None) -> list:
    command = ["sudo", "-n"]
    if netns_pid:
        command += ["nsenter", "-t", netns_pid, "-n"]
    return command + ["timeout", str(CAPTURE_SECONDS), "tcpdump", "-nqli", iface, packet_filter]


def start_captures(first: dict, second: dict) -> list:
    """
    Starts the captures on the underlay and on the ainari-net of both gateways.
    """
    network_id = subprocess.run(
        ["docker", "network", "ls", "-q", "--no-trunc", "--filter",
         f"label={UNDERLAY_NETWORK_LABEL}"],
        capture_output=True, text=True, check=True).stdout.split()
    if len(network_id) != 1:
        raise RuntimeError("the docker-network ainari-underlay was not found")
    bridge = f"br-{network_id[0][:12]}"

    # Only the traffic between the two sakura-hosts has to be encrypted. The one towards the
    # gateway at the edge, like the ssh-session into the virtual machine, is unencrypted
    # overlay-traffic, so the captures of the overlay are limited to these two gateways. A packet
    # of the virtual machines in clear has their addresses in its outer header, while the overlay
    # towards the edge wraps them into the addresses of the gateways.
    between = f"host {first['underlay']} and host {second['underlay']}"
    vm_subnet = str(ipaddress.ip_network(f"{first['ip']}/24", strict=False))
    captures = [
        Capture(f"ESP between {first['underlay']} and {second['underlay']}",
                tcpdump(bridge, f"{between} and esp"), True),
        Capture("unencrypted overlay-traffic between the sakura-hosts",
                tcpdump(bridge, f"{between} and udp port {OVERLAY_PORT}"), False),
        Capture(f"addresses of {vm_subnet} in clear on the underlay",
                tcpdump(bridge, f"icmp and net {vm_subnet}"), False),
    ]
    for gateway in GATEWAYS:
        pid = subprocess.run(["docker", "inspect", "-f", "{{.State.Pid}}", NETNS_HOLDERS[gateway]],
                             capture_output=True, text=True, check=True).stdout.strip()
        captures.append(Capture(
            f"traffic of the virtual machines on eth0 of {gateway}",
            tcpdump("eth0", f"esp or udp port {OVERLAY_PORT} or net {vm_subnet}", pid), False))
    return captures


def packet_counters(first: dict, second: dict, spis: dict) -> dict:
    """
    Reads the packet-counters of the SAs of both directions on both gateways.
    """
    counters = {}
    for sender, receiver in [(first, second), (second, first)]:
        spi = spis[sender["ip"]]
        for gateway in [sender["gateway"], receiver["gateway"]]:
            state = find_state(gateway, sender["underlay"], receiver["underlay"], spi)
            counters[(spi, gateway)] = state["packets"] if state else 0
    return counters


def free_address(first: dict, used: set) -> str:
    """
    Returns an address of the subnet of the first virtual machine, which no virtual machine uses.
    """
    network = ipaddress.ip_network(f"{first['ip']}/24", strict=False)
    for host in reversed(list(network.hosts())):
        if str(host) not in used:
            return str(host)
    raise RuntimeError(f"no free address in {network}")


def main() -> int:
    if not os.path.exists(SSH_KEY_PATH):
        log(f"the ssh-key {SSH_KEY_PATH} doesn't exist. Run prepare_resources.py first or set "
            f"AINARI_SSH_KEY.")
        return 1

    # the captures need root. The password is asked here, because the output of the captures is
    # read by the script, so a password-prompt there would not be visible.
    log("the captures need root, so sudo asks for the password")
    subprocess.run(["sudo", "-v"], check=True)

    # 1. the virtual machines
    pair = find_pair()
    if pair is None:
        log("no two running virtual machines of the same network on different sakura-hosts, of "
            "which one has a floating ip-address. Run prepare_resources.py (again), hanami picks "
            "the host of every virtual machine anew.")
        return 1
    (gateway, vm_ip, tap, floating_ip, vni), (other_gateway, other_ip, other_tap) = pair
    first = {"gateway": gateway, "ip": vm_ip, "tap": tap, "vni": vni,
             "underlay": underlay_address(gateway)}
    second = {"gateway": other_gateway, "ip": other_ip, "tap": other_tap, "vni": vni,
              "underlay": underlay_address(other_gateway)}
    log(f"virtual machine 1: {vm_ip} (floating ip {floating_ip}) behind {gateway} "
        f"({first['underlay']} on the underlay)")
    log(f"virtual machine 2: {other_ip} behind {other_gateway} "
        f"({second['underlay']} on the underlay), tenant {vni}")

    checks = Checks()

    # 2. the policies of both directions
    log("policies")
    spis = {
        first["ip"]: check_connection(checks, first["gateway"], first, second),
        second["ip"]: check_connection(checks, second["gateway"], second, first),
    }
    if None in spis.values():
        log("FAILED: the connection between the virtual machines is not encrypted. Is the "
            "network created with --disable-encryption?")
        return 1

    # 3. the keys of both directions on both gateways
    log("keys (SAs)")
    check_keys(checks, first, second, spis[first["ip"]])
    check_keys(checks, second, first, spis[second["ip"]])

    # 4. routing-rules and reverse-path filter
    log("routing-rules and reverse-path filter")
    check_rules(checks, first, second)
    check_rules(checks, second, first)

    # 5. - 7. traffic, counters and captures
    log(f"traffic from {first['ip']} to {second['ip']}, captured for {CAPTURE_SECONDS}s")
    before = packet_counters(first, second, spis)
    captures = start_captures(first, second)
    time.sleep(CAPTURE_STARTUP_SECONDS)
    ping = run_over_ssh(floating_ip, f"ping -c {PING_COUNT} -i 0.5 {second['ip']}", check=False)
    banner = run_over_ssh(floating_ip,
                          f"timeout 3 bash -c 'head -c 30 </dev/tcp/{second['ip']}/22'",
                          check=False)
    for capture in captures:
        capture.wait()

    checks.check(f"ping {first['ip']} -> {second['ip']}", " 0% packet loss" in ping.stdout,
                 next((line for line in ping.stdout.splitlines() if "packet loss" in line),
                      ping.stderr.strip()))
    checks.check(f"tcp {first['ip']} -> {second['ip']}:22",
                 banner.stdout.startswith("SSH-2.0-"), banner.stdout.strip()[:30])

    log("packet-counters of the SAs")
    after = packet_counters(first, second, spis)
    for sender, receiver in [(first, second), (second, first)]:
        spi = spis[sender["ip"]]
        sent = after[(spi, sender["gateway"])] - before[(spi, sender["gateway"])]
        received = after[(spi, receiver["gateway"])] - before[(spi, receiver["gateway"])]
        checks.check(f"spi 0x{spi:08x} {sender['ip']} -> {receiver['ip']}",
                     sent >= PING_COUNT and sent == received,
                     f"sent {sent} on {sender['gateway']}, received {received} on "
                     f"{receiver['gateway']}")

    log("captures")
    for capture in captures:
        if capture.error:
            checks.check(capture.name, False, f"capture failed: {capture.error}")
        elif capture.expect_packets:
            checks.check(capture.name, capture.count > 0, f"{capture.count} packet(s)")
        else:
            checks.check(f"no {capture.name}", capture.count == 0, f"{capture.count} packet(s)")

    # 8. a forged source-address is not encrypted
    used = {vm_ip for gw in GATEWAYS for _, vm_ip, _ in read_rows(
        gw, "SELECT tap_name, vni, vm_ip FROM taps")}
    forged = free_address(first, used)
    log(f"forged source-address {forged} within virtual machine 1")
    route = run_over_ssh(floating_ip, f"ip -o -4 route get {second['ip']}").stdout
    iface = route.split(" dev ")[1].split()[0]
    before = packet_counters(first, second, spis)
    try:
        run_over_ssh(floating_ip, f"sudo ip addr add {forged}/32 dev {iface}")
        forged_ping = run_over_ssh(floating_ip, f"ping -c 3 -W 1 -I {forged} {second['ip']}",
                                   check=False)
    finally:
        run_over_ssh(floating_ip, f"sudo ip addr del {forged}/32 dev {iface}", check=False)
    after = packet_counters(first, second, spis)
    spi = spis[first["ip"]]
    checks.check(f"ping from the forged {forged} is dropped",
                 "100% packet loss" in forged_ping.stdout,
                 next((line for line in forged_ping.stdout.splitlines() if "packet loss" in line),
                      forged_ping.stderr.strip()))
    checks.check(f"the forged packets don't use the key 0x{spi:08x}",
                 after[(spi, first["gateway"])] == before[(spi, first["gateway"])])

    if checks.failed:
        log(f"FAILED: {len(checks.failed)} check(s) failed")
        return 1

    log("SUCCESS: the traffic between the virtual machines is encrypted")
    return 0


if __name__ == "__main__":
    sys.exit(main())
