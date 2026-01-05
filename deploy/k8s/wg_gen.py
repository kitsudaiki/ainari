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
  python wg_k8s_gen.py --namespace $NAMESPACE
  (edit the CONFIG section below for endpoints, clients, etc.)
"""

import argparse
import subprocess
import sys
from jinja2 import Template

# ----------------------------
# CONFIG (base defaults)
# ----------------------------
BASE_NETWORK = "10.10."
NETWORK_MASK = "/24"

SERVER_LISTEN_PORT = 51820

SERVER_TEMPLATE = """[Interface]
PrivateKey = {{ server.priv }}
Address = {{ server.address }}
ListenPort = {{ server.listen_port }}

{% for peer in peers -%}
[Peer]
PublicKey = {{ peer.pub }}
AllowedIPs = {{ peer.allowed_ips }}
{% endfor -%}
"""

CLIENT_TEMPLATE = """[Interface]
PrivateKey = {{ client.priv }}
Address = {{ client.address }}

[Peer]
PublicKey = {{ server.pub }}
Endpoint = {{ server.endpoint }}
AllowedIPs = {{ client.allowed_ips }}
PersistentKeepalive = 25
"""

# ----------------------------
# Helpers (UNCHANGED 💕)
# ----------------------------
def run_cmd(cmd, input_bytes=None, check=True):
    try:
        return subprocess.run(
            cmd,
            input=input_bytes,
            shell=True,
            capture_output=True,
            check=check
        )
    except subprocess.CalledProcessError:
        raise

def gen_keypair():
    proc = run_cmd("wg genkey")
    priv = proc.stdout.decode().strip()
    proc2 = run_cmd(f"echo '{priv}' | wg pubkey")
    pub = proc2.stdout.decode().strip()
    return priv, pub

def k8s_create_secret_from_string(secret_name, key_name, content, namespace="default"):
    cmd = (
        f"kubectl create secret generic {secret_name} "
        f"--from-file={key_name}=/dev/stdin "
        f"--namespace {namespace} "
        f"--dry-run=client -o yaml | kubectl apply -f -"
    )
    run_cmd(cmd, input_bytes=content.encode())

# ----------------------------
# Main
# ----------------------------
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--namespace", required=True)
    parser.add_argument("--servers", type=int, required=True)
    parser.add_argument("--sakura-clients", type=int, required=True)
    parser.add_argument("--ryokan-clients", type=int, required=True)
    args = parser.parse_args()

    namespace = args.namespace

    for s in range(1, args.servers + 1):
        print(f"\n🌸 Generating server {s}...")
        net = f"{BASE_NETWORK}{s}"
        server_ip = f"{net}.1{NETWORK_MASK}"

        server_priv, server_pub = gen_keypair()

        server_data = {
            "priv": server_priv,
            "pub": server_pub,
            "address": server_ip,
            "listen_port": SERVER_LISTEN_PORT,
            "endpoint": f"wg-onsen-{s}.{namespace}.svc.cluster.local:{SERVER_LISTEN_PORT}",
        }

        peers_for_server = []
        client_index = 2

        clients = []

        for i in range(args.sakura_clients):
            name = f"sakura-{s}-{i}"
            priv, pub = gen_keypair()
            addr = f"{net}.{client_index}/32"
            client_index += 1

            clients.append((name, priv, pub, addr))
            peers_for_server.append({"pub": pub, "allowed_ips": addr})

        for i in range(args.ryokan_clients):
            name = f"ryokan-{s}-{i}"
            priv, pub = gen_keypair()
            addr = f"{net}.{client_index}/32"
            client_index += 1

            clients.append((name, priv, pub, addr))
            peers_for_server.append({"pub": pub, "allowed_ips": addr})

        server_cfg = Template(SERVER_TEMPLATE).render(
            server=server_data,
            peers=peers_for_server
        )

        k8s_create_secret_from_string(
            f"wg-server-{s}-secret",
            "wg0.conf",
            server_cfg,
            namespace
        )

        for name, priv, pub, addr in clients:
            client_cfg = Template(CLIENT_TEMPLATE).render(
                client={
                    "priv": priv,
                    "address": addr,
                    "allowed_ips": f"{net}.0/24",
                },
                server={
                    "pub": server_pub,
                    "endpoint": server_data["endpoint"],
                }
            )

            k8s_create_secret_from_string(
                f"wg-{name}-secret",
                "wg0.conf",
                client_cfg,
                namespace
            )

    print("\n✨ All WireGuard configs generated successfully, Onii-chan~ 💕")

if __name__ == "__main__":
    main()
