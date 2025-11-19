#!/usr/bin/env python3
"""
wg_k8s_gen.py

Generate WireGuard configs (1 server + N clients) using system `wg genkey` and store
each config as a Kubernetes Secret using simple kubectl commands.

Requirements:
  - wg (wg genkey, wg pubkey)
  - kubectl configured to the desired cluster/context
  - pip install jinja2

Usage:
  python wg_k8s_gen.py
  (edit the CONFIG section below for endpoints, clients, namespace, etc.)
"""

import subprocess
import sys
from jinja2 import Template

# ----------------------------
# CONFIG (edit as needed)
# ----------------------------
NAMESPACE = "default"
SERVER_ENDPOINT = "wg-onsen.default.svc.cluster.local:51820"  # used in client Peer Endpoint
NETWORK = "10.10.0."
NETWORK_MASK = "/24"

# server definition
server = {
    "name": "wg-server",
    "listen_port": 51820,
    "address": NETWORK + "1" + NETWORK_MASK,
    "dns": None,
    "secret_name": "wg-onsen-secret",  # k8s secret name to create
}

# clients list -- add or remove entries to change clients
clients = [
    {"name": "ryokan", "ip_last_octet": 2, "allowed_ips": "10.10.0.0/24", "secret_name": "wg-ryokan-secret"},
    {"name": "sakura", "ip_last_octet": 3, "allowed_ips": "10.10.0.0/24", "secret_name": "wg-sakura-secret"},
]

# Jinja2 templates for configs
SERVER_TEMPLATE = """[Interface]
PrivateKey = {{ server.priv }}
Address = {{ server.address }}
ListenPort = {{ server.listen_port }}
{% if server.dns %}DNS = {{ server.dns }}{% endif %}

{% for peer in peers -%}
[Peer]
PublicKey = {{ peer.pub }}
AllowedIPs = {{ peer.allowed_ips }}
{% endfor -%}
"""

CLIENT_TEMPLATE = """[Interface]
PrivateKey = {{ client.priv }}
Address = {{ client.address }}
{% if client.dns %}DNS = {{ client.dns }}{% endif %}

[Peer]
PublicKey = {{ server.pub }}
Endpoint = {{ server.endpoint }}
AllowedIPs = {{ client.allowed_ips }}
PersistentKeepalive = 25
"""

# ----------------------------
# Helpers
# ----------------------------
def run_cmd(cmd, input_bytes=None, check=True):
    """Run shell command (string) optionally with stdin bytes; return CompletedProcess."""
    try:
        completed = subprocess.run(
            cmd,
            input=input_bytes,
            shell=True,
            capture_output=True,
            check=check
        )
        return completed
    except subprocess.CalledProcessError as e:
        # debug-output
        # print(f"Command failed: {cmd}\nReturn code: {e.returncode}\nStdout: {e.stdout.decode()}\nStderr: {e.stderr.decode()}", file=sys.stderr)
        raise

def gen_keypair():
    """Generate a private key with wg genkey and corresponding public key with wg pubkey."""
    # generate private key
    proc = run_cmd("wg genkey")
    priv = proc.stdout.decode().strip()
    # compute public key
    proc2 = run_cmd(f"echo '{priv}' | wg pubkey")
    pub = proc2.stdout.decode().strip()
    return priv, pub

def k8s_create_secret_from_string(secret_name: str, key_name: str, content: str, namespace: str = "default"):
    """
    Create or update a Kubernetes secret with the given content (string) as a file entry.
    Uses: kubectl create secret generic <secret_name> --from-file=<key_name>=/dev/stdin --dry-run=client -o yaml | kubectl apply -f -
    """
    cmd = (
        f"kubectl create secret generic {secret_name} "
        f"--from-file={key_name}=/dev/stdin "
        f"--namespace {namespace} "
        f"--dry-run=client -o yaml | kubectl apply -f -"
    )
    # pass content bytes to stdin of the shell pipeline; kubectl will read /dev/stdin
    # print(f"Creating/updating secret '{secret_name}' in namespace '{namespace}'...")
    result = run_cmd(cmd, input_bytes=content.encode())
    stdout = result.stdout.decode().strip()
    if stdout:
        print(stdout)
    else:
        print("kubectl command completed for secret (no output).")

# ----------------------------
# Main flow
# ----------------------------
def main():
    # 1) Generate server keypair
    print("Generating server keypair...")
    server_priv, server_pub = gen_keypair()
    server_data = {
        "name": server["name"],
        "priv": server_priv,
        "pub": server_pub,
        "listen_port": server["listen_port"],
        "address": server["address"],
        "dns": server["dns"],
        "endpoint": SERVER_ENDPOINT,
    }

    # 2) Generate client keypairs
    client_objs = []
    peers_for_server = []
    for c in clients:
        print(f"Generating keys for client '{c['name']}'...")
        priv, pub = gen_keypair()
        addr = f"{NETWORK}{c['ip_last_octet']}/32"
        client_obj = {
            "name": c["name"],
            "priv": priv,
            "pub": pub,
            "address": addr,
            "dns": server["dns"],
            "allowed_ips": c["allowed_ips"],
            "secret_name": c.get("secret_name", f"wg-{c['name']}-secret"),
        }
        client_objs.append(client_obj)
        # server peer entry (server side) should contain the client's public key + allowed IP (client WG address)
        peers_for_server.append({"pub": pub, "allowed_ips": addr})

    # 3) Render server config and store as k8s secret
    server_cfg = Template(SERVER_TEMPLATE).render(server=server_data, peers=peers_for_server)
    # optionally include server public key in secret metadata? For simplicity we store only the .conf as 'wg0.conf'
    k8s_create_secret_from_string(server["secret_name"], "wg0.conf", server_cfg, namespace=NAMESPACE)

    # 4) Render each client config and store as k8s secret
    for c in client_objs:
        client_cfg = Template(CLIENT_TEMPLATE).render(client=c, server={"pub": server_pub, "endpoint": SERVER_ENDPOINT})
        secret_name = c["secret_name"]
        k8s_create_secret_from_string(secret_name, "wg0.conf", client_cfg, namespace=NAMESPACE)

    # 5) Summary
    print("\nDone. Summary:")
    print(" - Server secret created.")
    for c in client_objs:
        print(" - Client secret created.")

if __name__ == "__main__":
    main()
    