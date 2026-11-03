# To-Do: From Prototype to Production-Ready

Result of a repository review of `ainari` (v0.22.0, branch `qa/refactor-phase`, 2026-10-29).

## Current state (short summary)

- ~100k lines of Rust across 12 services (`hanami`, `torii`, `sakura`, `isekai`, `ryokan`, `miko`,
  `omamori`, `izakaya`, `omikuji`, `onsen`, `neko`, `torii-ebpf`) and 7 shared libs, plus a Go
  operator, Go SDK/CLI, Python SDK, Vue dashboard, Zarf package and three local test setups.
- Good foundation is already in place: Argon2id password hashing, AES-256-GCM encrypted
  images/snapshots/backups, audit log, cargo-deny, CodeQL, OpenSSF Scorecard, secret detection,
  commit lint, SBOM, Nix-based reproducible images, ~450 Rust unit tests and a Python end-to-end
  suite (`testing/ainari_test`).
- README, `SECURITY.md` and `docs/home/features.md` all say the project is a prototype and
  "not ready for productive usage".

---

## 1. Security

### Authentication and authorization

- [ ] Restrict CORS: `src/libs/rust/ainari_api/src/cors_middleware.rs` sends
      `access-control-allow-origin: *`, and the comment there already says "adjust in production".
      Make the allowed origins configurable, defaulting to the dashboard origin.
- [ ] Implement 2-factor authentication (TOTP/WebAuthn). It is listed as *planned* in `features.md`.
- [ ] Add brute-force protection to the login/token endpoints of `miko` (rate limiting, account
      lockout, backoff). The repo currently has no rate limiting.
- [ ] Add global API rate limiting and request-size limits to every public endpoint (e.g. via
      `actix-governor` or at the gateway).
- [ ] Review the JWT design: HS256 with a shared secret, and every component validates tokens with
      an HTTP call to `miko`. Consider asymmetric signing (EdDSA/ES256) with local verification,
      token revocation, short-lived access tokens plus refresh tokens, and a cache to cut the load on
      `miko`.
- [ ] Remove the remaining legacy password-hash path in `miko` (it needs `subtle` for constant-time
      comparison), after forcing a rehash on login.
- [ ] Replace the shared `INTERNAL_API_KEY` and `ONSEN_REGISTRATION_KEY` with per-host
      credentials: mTLS client certificates with rotation and revocation. This is already announced
      in `features.md` under "dumb compute hosts".
- [ ] Do a full authorization review (project isolation, admin vs. project-admin vs. user) on all
      132 `/v1alpha/` endpoints, and add negative tests ("user A cannot access project B") for
      each resource type.
- [ ] Optional: implement the four-eyes principle for projects (planned).

### Key management and secrets

- [ ] Replace the single static `key_b64` from the `omamori` config with a real KMS/HSM backend
      (Vault Transit, PKCS#11, YubiKey/HSM). This is planned in `architecture_overview.md`.
- [ ] Add key rotation and re-encryption for the omamori master key, the JWT signing key and the
      internal API keys.
- [ ] Implement "Hold your own key" (planned).
- [ ] Make sure no example key or default secret can reach production. `docker-compose.yml` falls
      back to `local-internal-api-key`, `local-onsen-registration-key` and similar values. Services
      should refuse to start with known example or default keys.
- [ ] Zeroize key material in memory everywhere, not only for passphrases (`zeroize`/`secrecy`).

### Transport security

- [ ] Make TLS mandatory between all internal services, not only via the WireGuard tunnel in the
      Kubernetes setup.
- [ ] Remove or strictly gate `skip_tls_verification` (sakura config, and `InsecureSkipVerify` in
      `src/sdk/go/ainari_sdk/http_request.go`). At minimum, log a loud warning, and reject it in a
      "production" mode.
- [ ] Add authentication to the `onsen` gRPC interface instead of relying only on network
      isolation and WireGuard.

### Hardening and process

- [ ] Write the security-concept document: `docs/home/security_concpets.md` is empty apart from
      its (misspelled) heading.
- [ ] Create a threat model (STRIDE or similar) for the control plane, compute hosts, the network
      data plane (eBPF/XFRM) and storage.
- [ ] Commission an external security audit or penetration test, especially for `torii` (eBPF/XDP,
      L3 proxy), `izakaya` (MLS key exchange) and `isekai` (backup crypto).
- [ ] Review all 21 `unsafe` blocks outside `torii-ebpf`, and document their safety invariants.
- [ ] Add fuzzing (`cargo-fuzz`) for parsers and network-facing code: torii packet handling, API
      input structs, backup manifest parsing.
- [ ] Turn the QA/security backlog mentioned in `SECURITY.md` ("bunch of known missing checks")
      into tracked issues, and close them.
- [ ] Set up a coordinated vulnerability disclosure process (GitHub Security Advisories, a PGP key
      and response-time targets).
- [ ] Harden containers: run as non-root, use a read-only root filesystem, drop capabilities, add
      seccomp/AppArmor profiles, and give sakura/torii only the capabilities they need (eBPF, tap,
      KVM).
- [ ] Sign container images and release artifacts (cosign/Sigstore), and publish SLSA provenance.

## 2. Correctness and robustness

- [ ] Resolve the open `TODO`s:
  - [ ] `src/binaries/sakura/src/api/http_endpoints/virtual_machine/create_virtual_machine_v1_0.rs:132`:
        the rollback on a failed task creation is commented out ("TODO: revert all"), so
        temporary files can leak.
  - [ ] `src/binaries/sakura/src/core/processing/worker_thread.rs:88-102`: task errors are only
        logged ("TODO: better error-handling"). They should set the task state, notify the user
        and send the error to `omikuji`.
  - [ ] `src/binaries/hanami/src/api/http_endpoints/virtual_machine/reserve_virtual_machine_v1_0.rs:167`:
        check that the network, image and public key UUIDs exist before reserving.
  - [ ] `src/libs/rust/ainari_clients/src/onsen_file_transfer.rs:60`: check that the local file
        path exists.
  - [ ] `src/libs/rust/ainari_hardware/src/host.rs:150`: handle hosts whose big cores are not
        identical (hybrid CPUs).
- [ ] Replace the busy-wait in the sakura worker thread (`thread::sleep(1ms)` polling) with a
      channel or notify-based queue.
- [ ] Audit the ~1,500 `unwrap()`/`expect()`/`panic!` calls. Most are in tests, but the remaining
      ones in non-test code, including `lock().unwrap()`, must not crash a service. Note that
      `panic = "abort"` is set for both release and dev.
- [ ] Add `clippy::unwrap_used` / `clippy::expect_used` (deny in non-test code) and review the
      existing `#[allow(clippy::…)]` exceptions.
- [ ] Make multi-service operations transactional or saga-based with compensation (VM create,
      migration, backup/restore across hanami/sakura/torii/ryokan/omamori), so that partial
      failures don't leave orphaned resources.
- [ ] Add a periodic reconciliation job that detects and cleans up drift between the hanami, sakura,
      torii and ryokan databases (orphaned VMs, IPs, floating IPs, images).
- [ ] Use idempotency keys for create/delete requests, so clients can retry safely.
- [ ] Add a consistent retry/backoff and timeout policy for all inter-service clients
      (`ainari_clients`). Only about 5 files currently contain retry or backoff logic.
- [ ] Add graceful shutdown (SIGTERM handling, connection draining, finishing or cleanly aborting
      tasks) for all services.
- [ ] Reconsider the design decision that all open sakura tasks go to `ERROR` after a restart
      (`docs/home/design_decisions.md`). At least provide automatic cleanup or a guided recovery
      path.
- [ ] Make input validation (`validator`) complete and consistent on all API structs (sizes,
      lengths, CIDR/port ranges, names).

## 3. Testing

- [ ] Measure code coverage (`cargo llvm-cov`) and set a minimum threshold in CI.
- [ ] Add unit tests for the crates with few tests: `onsen`, `omikuji`, `izakaya`, `ainari_clients`
      (5.8k LOC, 1 test file) and `ainari_files`.
- [ ] Add dashboard tests: there are none (`src/dashboard/app`). Add Vitest unit tests and
      Playwright end-to-end tests, plus type-checking (`vue-tsc`) and ESLint in CI.
- [ ] Add tests for the Go SDK and CLI: only 4 `_test.go` files exist across 79 Go files.
- [ ] Add tests for the Python SDK: there are none in `src/sdk/python`.
- [ ] Run the end-to-end suite (`testing/ainari_test`) automatically in CI, for example on a kind
      cluster, at least nightly and before every release.
- [ ] Add upgrade tests: deploy version N, create resources, upgrade to N+1, and verify that data
      and VMs survive.
- [ ] Add chaos/failure tests: kill services, partition the network, fill disks, restart hosts with
      running VMs.
- [ ] Add load and performance tests (API throughput, VM boot time, network throughput with
      IPsec, image upload sizes), with documented limits.
- [ ] Add restore tests for backups (`isekai`), including disaster recovery into another deployment.
- [ ] Re-enable or remove the commented-out ansible-lint job in `.github/workflows/build_test.yml`.

## 4. Observability and operations

- [ ] Add health endpoints (`/healthz`, `/readyz`) to every service, with real dependency checks
      (database, `miko`, `onsen`). None exist in the Rust code yet.
- [ ] Add Prometheus metrics to every service: request rate, latency, errors, task queue length,
      VM counts, eBPF/network counters, and backup durations.
- [ ] Add distributed tracing (OpenTelemetry) with a request ID that is passed through all service
      hops.
- [ ] Switch to structured JSON logging (`tracing` instead of `log`/`env_logger`), with
      configurable levels and no secrets or tokens in the logs.
- [ ] Ship Grafana dashboards and alerting rules (Prometheus rules), and write runbooks for every
      alert.
- [ ] Define and document SLOs (API availability, VM create latency).
- [ ] Add retention, rotation and export (SIEM) for the audit log in `omikuji`, and make it
      tamper-evident (hash chain, append-only storage).

## 5. Data and persistence

- [ ] Define a database strategy for production. The services use Diesel with SQLite, and
      MySQL only optionally. Document which components must use MySQL/MariaDB in production, and
      plan HA for it (Galera, operator-managed cluster, or managed DB).
- [ ] Add PostgreSQL support, or explicitly decide against it.
- [ ] Ensure migrations (`diesel_migrations`) are forward-only, tested, and safe to run online, and
      document the rollback strategy.
- [ ] Automate backup and restore of the control-plane databases themselves, not just tenant
      backups via `isekai`, and test it regularly.
- [ ] Back up the sakura SQLite databases on the compute hosts, or document how they can be
      rebuilt.
- [ ] Encrypt sensitive database columns at rest, and enable DB-level encryption.

## 6. High availability and scalability

- [ ] Make all control-plane services (`miko`, `hanami`, `ryokan`, `omamori`, `omikuji`,
      `izakaya`, `isekai`) run with more than one replica: no in-memory singleton state, leader
      election where needed, and shared state in the DB.
- [ ] Design HA for the public `torii` gateway (failover of floating IPs, multiple gateways).
- [ ] Add PodDisruptionBudgets, anti-affinity and topology spread constraints in the operator.
- [ ] Set resource requests and limits for all pods. The operator templates set limits only once.
- [ ] Add redundancy and replication for the storage backend (`onsen`), or integrate with
      established storage such as Ceph or S3.
- [ ] Implement live migration of VMs for host maintenance (drain host). Check how complete the
      existing migration endpoints are.
- [ ] Determine and document scaling limits: number of hosts, VMs and networks; scheduler
      performance in `hanami`.

## 7. Deployment and Kubernetes operator

- [ ] Promote the CRD from `v1alpha1` toward `v1beta1`/`v1`, with conversion webhooks and a
      validation webhook.
- [ ] Add an upgrade path in the operator: version-aware rolling upgrades, DB migrations as a
      separate job, and rollback.
- [ ] Add operator status conditions, events, and metrics.
- [ ] Integrate cert-manager (or similar) for TLS certificates.
- [ ] Integrate external secrets (External Secrets Operator, Vault) instead of secrets generated
      only by the operator.
- [ ] Add NetworkPolicies for all components. In particular, `onsen` must not be reachable except
      from `ryokan`, `sakura` and `isekai`.
- [ ] Add a Helm chart or OLM bundle for the operator itself.
- [ ] Clean up `src/archive/` (old Ansible deployment) or move it out of the main tree.
- [ ] Document a production reference architecture: minimum node count, hardware and kernel
      requirements, network layout.

## 8. API, SDKs and dashboard

- [ ] Stabilize the API: move `/v1alpha/` to `/v1/`, define a deprecation policy, and add
      OpenAPI-based contract tests and breaking-change detection in CI.
- [ ] Add pagination, filtering and sorting to all list endpoints.
- [ ] Use one consistent error format for all services (e.g. RFC 9457 Problem Details) with stable
      error codes.
- [ ] Publish the SDKs as versioned packages (PyPI, Go module tags) with changelogs.
- [ ] Dashboard:
  - [ ] Add a content security policy and security headers (nginx config).
  - [ ] Add session timeout and token handling (refresh, logout everywhere).
  - [ ] Review accessibility (a11y) and add internationalization (i18n) if needed.
  - [ ] Update `primevue` 3.x to the current major version.
- [ ] Write a Terraform/OpenTofu provider, or document integration with common tools (optional,
      but often expected for IaaS).

## 9. Code quality and maintainability

- [ ] Finish the ongoing refactor phase (`qa/refactor-phase`, #1053) and merge it.
- [ ] Fix typos in code and docs, e.g. `resouce_uuid`, "train-task" in sakura,
      `security_concpets.md`, "Design decitions", "Secutiry", and "detect-sectets" in CI.
- [ ] Add a `clippy.toml` with stricter workspace-wide lints, and run `cargo clippy -- -D warnings`
      in CI.
- [ ] Add `cargo-machete`/`cargo-udeps` to find unused dependencies.
- [ ] Add `cargo-semver-checks` for the libraries, if they are published.
- [ ] Use workspace-wide dependency versions (`[workspace.dependencies]`) for common crates such as
      `actix-web`, `diesel`, `serde` and `tokio`, to avoid version drift between services.
- [ ] Pin the nightly toolchain via `rust-toolchain.toml`, since CI needs nightly for eBPF.

## 10. Documentation

- [ ] Write the security concept (see §1).
- [ ] Write an operations handbook: installation, upgrade, backup/restore, disaster recovery,
      scaling, host maintenance, certificate and key rotation, and troubleshooting.
- [ ] Write a hardening guide for production deployments.
- [ ] Remove the "prototype" warnings in README, `SECURITY.md` and `features.md` once the items
      above are done.
- [ ] Add support and compatibility matrices: supported kernel, Kubernetes, cloud-hypervisor and
      guest OS versions.

## 11. Project and release process

- [ ] Reduce the bus factor: the project currently has a single maintainer (see `SECURITY.md`).
      Bring in more maintainers and reviewers, and require reviews in branch protection.
- [ ] Define a release process: release branches, LTS/support policy, security backports, and
      signed tags.
- [ ] Release 1.0.0 with SemVer guarantees for the API, CRD and SDKs.
- [ ] Decide on and document commercial support or an SLA, if the project is meant for production
      use by third parties.
