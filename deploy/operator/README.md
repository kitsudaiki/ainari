# ainari-operator

Kubernetes-operator, which deploys the complete ainari-stack, which is otherwise deployed by the
helm-chart in `deploy/k8s/ainari`. The whole stack is described by a single custom-resource of the
kind `Ainari`. The operator deploys all components into the namespace of the resource, so there
can only be one `Ainari` per namespace. A second one is marked with the reason `Conflict` and
ignored.

## Requirements

Like for the helm-chart:

- [cert-manager](https://cert-manager.io), which issues the certificates of all components
- [ingress-nginx](https://kubernetes.github.io/ingress-nginx/) with ssl-passthrough, if the
  ingresses are enabled
- the node-labels like `hanami-node=true`, if `global.strictScheduling` is enabled

## Installation

```bash
# build and push the image
make docker-build docker-push IMG=<registry>/ainari_operator:<tag>

# install the CRD and the operator into the namespace 'ainari-system'
make deploy IMG=<registry>/ainari_operator:<tag>

# deploy a stack
kubectl create namespace ainari
kubectl apply -n ainari -f config/samples/ainari_v1alpha1_ainari.yaml
kubectl get ainari -n ainari
```

`config/samples/ainari_v1alpha1_ainari.yaml` is a minimal stack with the defaults of all
components, `config/samples/ainari_v1alpha1_ainari_kind.yaml` is the same setup as
`deploy/k8s/kind/values.yaml`. All fields and their defaults are shown by
`kubectl explain ainari.spec --recursive` or can be found in `api/v1alpha1/ainari_types.go`.

## Secrets

The `Ainari`-resource contains no secrets. The operator generates every key and password once
and stores it in a secret of the namespace:

| secret                    | content                                                         |
|---------------------------|-----------------------------------------------------------------|
| `internal-api-key`        | key, with which the components authenticate each other         |
| `onsen-registration-key`  | key, with which onsen registers at ryokan                       |
| `sakura-registration-key` | key, with which sakura registers at hanami                      |
| `mls-grant-signing-key`   | Ed25519-key of hanami for the MLS-groups, its public key is in `status.mlsGrantPublicKey` |
| `token-key`               | key, with which miko signs its tokens                           |
| `miko-admin`              | passphrase of the admin                                         |
| `omamori-encryption-key`  | key, with which omamori encrypts the stored secrets             |
| `mysql-credentials`       | passwords of the deployed mysql-server                          |
| `wireguard-keys`          | private keys of the pods of the wireguard-tunnel                |

These secrets are never overwritten, so no key or password, which data depends on, gets lost:

- An existing secret is always taken over as it is, also one, which was created before by
  someone else. Only keys, which it misses, are added. This way own values can be given, for
  example the passphrase of the admin:
  `kubectl create secret generic miko-admin --from-literal=passphrase=<passphrase>`
- The operator creates them `immutable`, so nobody can change them by accident. Only
  `wireguard-keys` gets new keys for new pods, but never changes the existing ones.
- They have no owner, so they stay, when the `Ainari` is deleted, and a new one takes them over
  again, together with the volumes of mysql. They are labeled with
  `ainari.kitsunemimi.moe/generated=true` and have to be deleted by hand to remove a stack
  completely. A deleted secret is generated again with a new value, which makes the data, that
  depends on the old one, unreadable.

An external mysql-server (`mysql.deploy: false`) already has its users, so its passwords are
given with `mysql.credentialsSecret`, the name of a secret with the keys `miko_password`,
`hanami_password`, `ryokan_password`, `omamori_password` and `izakaya_password`.

The passphrase of the generated admin:

```bash
kubectl get secret -n ainari miko-admin -o jsonpath='{.data.passphrase}' | base64 -d
```

## Wireguard

The connections of ryokan and sakura to onsen run through wireguard, if `global.wireguard` is
enabled. The operator creates the keys and configs, so `deploy/k8s/wg_gen.py` is not needed.

- every pod of onsen, ryokan and sakura has its own key and address within `10.10.0.0/16`
- every onsen is a peer of every ryokan and every sakura, so all of them reach every onsen and
  every onsen reaches every ryokan, also with several replicas of each of them
- the peers are reached over the names of their pods, like
  `onsen-1.onsen.<namespace>.svc.cluster.local`, so onsen, ryokan and sakura run as statefulsets
- every pod has the sidecar `wireguard-sync`, which keeps the peers of its interface in sync with
  the secret `wg-<component>-secret`, so a change of the number of replicas reaches the running
  pods without a restart

## Differences to the helm-chart

- all keys and passwords are generated into secrets, which are never changed, instead of being
  part of the values
- the config of omamori is a secret, because it contains the key, with which omamori encrypts the
  stored secrets, which was a fixed value in the helm-chart
- the passphrase of the admin is read from the secret `miko-admin` instead of a plain env-variable
- ryokan is a statefulset and every onsen-pod has its own volume, so both can have several
  replicas together with wireguard
- the unused persistent-volumes `onsen-pv` and `torii-pv` are not created
- objects of disabled components, like the dashboard or izakaya, are removed again

## Development

```bash
make help        # all targets
make manifests   # regenerate the CRD and the RBAC-role after a change of the API or the markers
make test        # unit-tests and tests against an api-server of envtest
make run         # run the operator against the cluster of the current kube-context
```

The code is structured into:

| package               | content                                                              |
|-----------------------|----------------------------------------------------------------------|
| `api/v1alpha1`        | the custom-resource `Ainari`                                         |
| `internal/secrets`    | generates the keys and passwords once and never changes them         |
| `internal/render`     | creates all objects of the stack from the spec, one file per component, the configs are templates in `internal/render/templates` |
| `internal/apply`      | writes the objects with server-side-apply and removes outdated ones  |
| `internal/controller` | the reconciler, which combines the packages above and sets the status |
