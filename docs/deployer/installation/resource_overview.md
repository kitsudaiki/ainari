# Resource-overview

All kubernetes-resources, which the operator in `deploy/operator` creates directly for an
`Ainari`-resource. All of them are created in the namespace of the `Ainari`-resource and get the
labels `app.kubernetes.io/managed-by: ainari-operator`, `app.kubernetes.io/instance: <NAME>` and
`app.kubernetes.io/part-of: ainari`.

Except the [generated secrets](#generated-secrets), all resources are owned by the
`Ainari`-resource, so they are deleted together with it, and the operator removes them again,
when they are not required anymore after a change of the resource.

Resources without a condition are always created. The conditions refer to the fields of the
`Ainari`-resource.

## Issuer

| Name                | Condition                            | Used for                                      |
| ------------------- | ------------------------------------ | --------------------------------------------- |
| `selfsigned-issuer` | `global.certificates.caSecret` empty | signs the self-signed CA `ainari-ca`          |
| `ainari-ca-issuer`  |                                      | issues all TLS-certificates of the components |

## Certificate

| Name                | Condition                            | Used for                                       |
| ------------------- | ------------------------------------ | ---------------------------------------------- |
| `ainari-ca`         | `global.certificates.caSecret` empty | self-signed CA of the stack                    |
| `miko-cert`         |                                      | TLS of miko                                    |
| `hanami-cert`       |                                      | TLS of hanami                                  |
| `ryokan-cert`       |                                      | TLS of ryokan                                  |
| `omamori-cert`      |                                      | TLS of omamori                                 |
| `omikuji-cert`      |                                      | TLS of omikuji                                 |
| `isekai-cert`       |                                      | TLS of isekai                                  |
| `izakaya-cert`      |                                      | TLS of izakaya                                 |
| `sakura-cert`       |                                      | TLS of the sakura-hosts                        |
| `sakura-proxy-cert` |                                      | wildcard-certificate of the proxies to the VMs |
| `torii-cert`        |                                      | TLS of the torii-gateways                      |
| `dashboard-cert`    | `dashboard.enabled`                  | TLS of the dashboard                           |

## ConfigMap

| Name                     | Condition                      | Used for                                         |
| ------------------------ | ------------------------------ | ------------------------------------------------ |
| `miko-config`            |                                | config-file of miko                              |
| `hanami-config`          |                                | config-file of hanami                            |
| `ryokan-config`          |                                | config-file of ryokan                            |
| `omikuji-config`         |                                | config-file of omikuji                           |
| `isekai-config`          |                                | config-file of isekai                            |
| `izakaya-config`         |                                | config-file of izakaya                           |
| `onsen-config`           |                                | config-template of onsen                         |
| `onsen-backup-config`    |                                | config-template of the onsen for backups         |
| `sakura-config`          |                                | config-template of sakura                        |
| `torii-config`           |                                | config-templates of the public and the VMM-torii |
| `torii-scripts`          |                                | start-script of torii                            |
| `dashboard-config`       | `dashboard.enabled`            | config of the dashboard and its web-server       |
| `miko-nginx-config`      |                                | nginx-config of the TLS-sidecars of miko         |
| `hanami-nginx-config`    |                                | nginx-config of the TLS-sidecars of hanami       |
| `ryokan-nginx-config`    |                                | nginx-config of the TLS-sidecars of ryokan       |
| `omamori-nginx-config`   |                                | nginx-config of the TLS-sidecars of omamori      |
| `omikuji-nginx-config`   |                                | nginx-config of the TLS-sidecars of omikuji      |
| `isekai-nginx-config`    |                                | nginx-config of the TLS-sidecars of isekai       |
| `izakaya-nginx-config`   |                                | nginx-config of the TLS-sidecar of izakaya       |
| `sakura-nginx-config`    |                                | nginx-config of the TLS-sidecars of sakura       |
| `torii-nginx-config`     |                                | nginx-config of the TLS-sidecars of torii        |
| `dashboard-nginx-config` | `dashboard.enabled`            | nginx-config of the TLS-sidecar of the dashboard |
| `mysql-init`             | `mysql.deploy`                 | script, which creates the databases and users    |
| `wireguard-scripts`      | `global.wireguard.enabled`     | sync-script of the wireguard-sidecars            |

## Secret

| Name                     | Condition                  | Used for                                                  |
| ------------------------ | -------------------------- | --------------------------------------------------------- |
| `omamori-config`         |                            | config-file of omamori, which contains its encryption-key |
| `wg-onsen-secret`        | `global.wireguard.enabled` | wireguard-configs and peers of the onsen-pods             |
| `wg-onsen-backup-secret` | `global.wireguard.enabled` | wireguard-configs and peers of the onsen-pods for backups |
| `wg-ryokan-secret`       | `global.wireguard.enabled` | wireguard-configs and peers of the ryokan-pods            |
| `wg-sakura-secret`       | `global.wireguard.enabled` | wireguard-configs and peers of the sakura-pods            |

### Generated secrets

These secrets are generated only once and never changed afterwards. They have no owner, so they
stay, when the `Ainari`-resource is deleted, and are taken over by the next one.

| Name                      | Condition                      | Used for                                       |
| ------------------------- | ------------------------------ | ---------------------------------------------- |
| `internal-api-key`        |                                | key of the internal api between the components |
| `onsen-registration-key`  |                                | registration of the onsen-hosts at ryokan      |
| `sakura-registration-key` |                                | registration of the sakura-hosts at hanami     |
| `token-key`               |                                | key, with which miko signs the tokens          |
| `miko-admin`              |                                | passphrase of the admin-user                   |
| `omamori-encryption-key`  |                                | key, with which omamori encrypts the secrets   |
| `mls-grant-signing-key`   |                                | key, with which hanami signs the MLS-grants    |
| `mysql-credentials`       | `mysql.deploy`                 | root-password and passwords of the databases   |
| `wireguard-keys`          | `global.wireguard.enabled`     | private wireguard-keys of all tunnel-pods      |

## Service

| Name                    | Condition                      | Used for                                         |
| ----------------------- | ------------------------------ | ------------------------------------------------ |
| `miko-tls-service`      |                                | internal api of miko                             |
| `hanami-tls-service`    |                                | internal api of hanami                           |
| `ryokan-tls-service`    |                                | internal api of ryokan                           |
| `omamori-tls-service`   |                                | internal api of omamori                          |
| `omikuji-tls-service`   |                                | internal api of omikuji                          |
| `isekai-tls-service`    |                                | internal api of isekai                           |
| `izakaya-tls-service`   |                                | internal api of izakaya                          |
| `torii-tls-service`     |                                | api of the public torii                          |
| `dashboard-tls-service` | `dashboard.enabled`            | internal access to the dashboard                 |
| `miko-external`         |                                | public api of miko                               |
| `hanami-external`       |                                | public api of hanami                             |
| `ryokan-external`       |                                | public api of ryokan                             |
| `omamori-external`      |                                | public api of omamori                            |
| `omikuji-external`      |                                | public api of omikuji                            |
| `isekai-external`       |                                | public api of isekai                             |
| `dashboard-external`    | `dashboard.enabled`            | public access to the dashboard                   |
| `ryokan`                |                                | headless, DNS-names and wireguard of the pods    |
| `onsen`                 |                                | headless, gRPC and wireguard of the onsen-pods   |
| `onsen-backup`          |                                | headless, gRPC and wireguard of the backup-onsen |
| `sakura`                |                                | headless, apis and wireguard of the sakura-pods  |
| `torii-public`          |                                | headless, api of the public torii-pods           |
| `torii-gateway-service` |                                | proxy-port of the gateway to the VMs             |
| `mysql`                 | `mysql.deploy`                 | access to the MySQL-database                     |

## Deployment

| Name           | Condition                      | Used for                                    |
| -------------- | ------------------------------ | ------------------------------------------- |
| `miko`         |                                | users, projects and authentication          |
| `hanami`       |                                | scheduling of VMs and the virtual networks  |
| `omamori`      |                                | secrets and public keys                     |
| `omikuji`      |                                | error- and audit-logs                       |
| `isekai`       |                                | backup and restore                          |
| `izakaya`      |                                | MLS-coordination of the network-encryption  |
| `torii-public` |                                | public gateway in front of the sakura-hosts |
| `dashboard`    | `dashboard.enabled`            | web-interface                               |

## StatefulSet

| Name           | Condition      | Used for                                                 |
| -------------- | -------------- | -------------------------------------------------------- |
| `ryokan`       |                | images, snapshots and backup-packages                    |
| `onsen`        |                | storage of the image-files                               |
| `onsen-backup` |                | storage of the backups                                   |
| `sakura`       |                | hypervisor-hosts of the VMs, each with its own VMM-torii |
| `mysql`        | `mysql.deploy` | database of the components                               |

## PersistentVolumeClaim

| Name        | Condition | Used for                     |
| ----------- | --------- | ---------------------------- |
| `torii-pvc` |           | database of the public torii |

## TLSRoute

| Name              | Condition                                        | Used for                           |
| ----------------- | ------------------------------------------------ | ---------------------------------- |
| `miko-route`      | `global.gateway.enabled`                         | public api of miko over gateway    |
| `hanami-route`    | `global.gateway.enabled`                         | public api of hanami over gateway  |
| `ryokan-route`    | `global.gateway.enabled`                         | public api of ryokan over gateway  |
| `omamori-route`   | `global.gateway.enabled`                         | public api of omamori over gateway |
| `omikuji-route`   | `global.gateway.enabled`                         | public api of omikuji over gateway |
| `isekai-route`    | `global.gateway.enabled`                         | public api of isekai over gateway  |
| `torii-route`     | `global.gateway.enabled`                         | api of torii over gateway          |
| `dashboard-route` | `global.gateway.enabled` and `dashboard.enabled` | dashboard over gateway             |

## Created by cert-manager

These secrets are not created by the operator itself, but by cert-manager for the
[certificates](#certificate). Each contains the certificate (`tls.crt`), its private key (`tls.key`)
and the CA (`ca.crt`) and is renewed by cert-manager before the certificate expires. By default
cert-manager sets no owner on them (only with its flag `--enable-certificate-owner-ref`), so they
stay, when the `Ainari`-resource is deleted. Deleting `ainari-ca` creates a new CA, which the
existing certificates are not signed by, until they are issued again.

| Name                      | Condition                            | Used for                                          |
| ------------------------- | ------------------------------------ | ------------------------------------------------- |
| `ainari-ca`               | `global.certificates.caSecret` empty | CA of the stack, used by `ainari-ca-issuer`       |
| `miko-tls-secret`         |                                      | TLS-sidecars of miko                              |
| `hanami-tls-secret`       |                                      | TLS-sidecars of hanami                            |
| `ryokan-tls-secret`       |                                      | TLS-sidecars of ryokan                            |
| `omamori-tls-secret`      |                                      | TLS-sidecars of omamori                           |
| `omikuji-tls-secret`      |                                      | TLS-sidecars of omikuji                           |
| `isekai-tls-secret`       |                                      | TLS-sidecars of isekai                            |
| `izakaya-tls-secret`      |                                      | TLS-sidecar of izakaya                            |
| `sakura-tls-secret`       |                                      | internal TLS-sidecar of sakura                    |
| `sakura-proxy-tls-secret` |                                      | external TLS-sidecar of sakura for the VM-proxies |
| `torii-tls-secret`        |                                      | TLS-sidecars of the public and the VMM-torii      |
| `dashboard-tls-secret`    | `dashboard.enabled`                  | TLS-sidecar of the dashboard                      |

For every issuance of a certificate, cert-manager additionally creates a `CertificateRequest`
named `<CERTIFICATE>-<NUMBER>`, like `miko-cert-1`, which documents the request at the issuer.

!!! note

    Not listed are the resources, which are created indirectly by the deployments and
    stateful-sets: their pods and replica-sets and the volumes `data-*` of the stateful-sets of
    `mysql`, `onsen` and `onsen-backup`.
