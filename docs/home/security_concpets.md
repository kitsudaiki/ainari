# Security concepts

This page summarizes the security concepts of Ainari and which threats they address. A detailed
list of all features with their implementation status can be found on the
[Features](/home/features/) page.

!!! warning

    The project is still in a prototypical state and not ready for productive usage. See also the
    `SECURITY.md` in the repository for known limitations and how to report vulnerabilities.

## Trust model

The components are not equally trusted. The basic idea is to keep the components, which are most
exposed to attackers, as "dumb" as possible.

| Component | Trust | Reason |
|---|---|---|
| **Miko** | highest | Issues and validates the JWT-tokens of all users |
| **Hanami** | high | Decides which hosts are allowed to join the encrypted tenant networks |
| **Omamori** | high | Holds the keys of the users |
| **Isekai** | medium | Orchestrates backups and restores, handles the backup-passphrase and the wrapped project-secrets, but stores no data itself |
| **Ryokan** | medium | Manages images and snapshots, sees uploaded images in plain text, until they are converted and encrypted |
| **Omikuji** | medium | Collects the error- and audit-logs, so it must be protected against manipulation, but holds no keys |
| **Izakaya** | low | Only relays MLS-messages, can not add members without a grant of Hanami |
| **Sakura / Torii** (compute hosts) | low | Run untrusted VMs of the users, so a VM-breakout is assumed to be possible |
| **Onsen** (storage) | lowest | Only stores encrypted data, never reachable from the internet |

## Threats and mitigations

### Compromised compute host

*Threat:* An attacker breaks out of a VM and takes over the Sakura-host.

- **No privileged credentials on the host:** Every action is executed with the token of the user,
  who triggered it. The host has no admin user, no database- or message-queue-credentials, only an
  internal API-key and a registration-key. So the host alone is not enough to take over the
  deployment. Even the internal API-key can not be used without a valid user token.
- **Restricted root-access:** Commands, which require root-privileges, are executed by Neko, which
  only allows commands of an allow-list.
- **Project host isolation:** Hosts can be dedicated to a single project, so an attacker must
  already be part of the same project, to land on the same hardware.
- **Small attack surface of the hypervisor:** [Cloud-Hypervisor](https://github.com/cloud-hypervisor/cloud-hypervisor)
  is used instead of qemu, which has a much smaller code base and is written in Rust.
- **Membership pinning:** The MLS-identity of each host is pinned by Hanami, so a compromised host
  can not impersonate another host or join networks, where it has no VM.

### Eavesdropping on the tenant network

*Threat:* An attacker with access to the underlay network between the compute hosts reads or
manipulates the traffic between the VMs.

- **IPsec encryption:** Traffic between VMs on different hosts is encrypted with IPsec ESP
  (AES-256-GCM) in tunnel mode, using the xfrm stack of the kernel. Inbound policies are set to
  `level required`, so unencrypted packets of an encrypted connection are dropped.
- **Keys never travel over the wire:** All Torii instances of a network form an
  [MLS](https://www.rfc-editor.org/rfc/rfc9420.html) group (via [OpenMLS](https://github.com/openmls/openmls))
  and derive the keys of the Security Associations from the secret of the current epoch.
- **Signed membership:** Only hosts with a grant signed by Hanami can join a group. Every member
  verifies the grants of a commit, so neither a compromised Izakaya nor a compromised Torii can add
  a foreign member.
- **Key rotation:** The keys are rotated regularly (default every hour) and whenever the members of
  a group change. Hosts without VMs of the network are removed from the group.

!!! info

    The encryption can be optionally disabled per network (`disable_encryption`). 
    Traffic to and from the edge Torii (internet gateway) is not encrypted by IPsec.

Additionally, network filter rules can restrict incoming and outgoing traffic of VMs to specific
ports and IP-ranges.

### Compromised storage backend

*Threat:* An attacker gets access to the Onsen or the backup storage and steals or manipulates data.

- **Encrypted images and snapshots:** All images and snapshots are encrypted with AES-256-GCM,
  before they are moved to the storage, so data can neither be read nor be manipulated unnoticed.
- **Encrypted backups:** VM- and project-backups are encrypted with a key derived from a passphrase
  of the user (Argon2id + HKDF-SHA256).
- **Isolated storage network:** The Onsen has no REST-API, is not reachable from the internet and is
  connected to Ryokan and Sakura over gRPC through a wireguard-tunnel in the Kubernetes setup.

### Untrusted key generation in the backend

*Threat:* The user doesn't trust the random generator or the key handling of the backend.

- **Bring your own key:** Users can upload their own keys to Omamori.

### Stolen user credentials

*Threat:* An attacker gets the passphrase or a token of a user.

- **Strong passphrase hashing:** Passphrases are hashed with Argon2id.
- **Expiring tokens:** Access is based on JWT-tokens with an expiration time, which are validated
  by Miko for every request.

### Man-in-the-middle on the VM control path

*Threat:* The gateway reads or manipulates the API-calls of a user towards the compute host.

- Torii only acts as layer-3-proxy and doesn't terminate the HTTPS-connection, so the connection
  between the user and the Sakura-host is end-to-end encrypted.

### Undetected misuse

*Threat:* Malicious actions of users or admins remain unnoticed.

- **Audit-log:** Every REST-request passes a middleware, which reports the access to the central
  log service Omikuji.
- **Task-log per VM:** Each action on a VM is logged on the compute host, with user, time and
  result, and is visible to all users of the VM.

### Supply chain and memory-safety

*Threat:* Vulnerable dependencies or memory-corruption bugs in the backend.

- **Rust:** The whole backend is written in Rust and avoids `unsafe` code outside of the low-level
  network layer.
- **Reproducible builds:** Rust dependencies are pinned in `Cargo.lock` and the published
  docker-images are built with [Nix](https://nixos.org/).
- **SBOM:** A list of all packages and versions of the images is generated, to track CVEs.
- **Secret handling:** Secrets in memory are zeroized on drop and are never printed in logs.

## Not covered yet

- The memory of the compute host is not encrypted, so a compromised host can read the memory of its
  VMs. AMD SEV-SNP support is planned.
- Omamori currently encrypts the stored keys only with a key from its config. HSM (YubiKey) and
  Vault backends are planned.
- The registration-key of the compute hosts is shared and will be replaced by host-specific
  certificates.
- Keys are always held by Omamori. With the planned **hold your own key**, keys are provided by the
  user only when needed and are only kept in memory of the compute host for the duration of the
  task.
- Logins are protected by the passphrase only. **2-factor authentication** and an optional
  **four-eyes principle** for projects are planned.
