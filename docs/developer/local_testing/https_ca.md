# The CAs of the local setups

Every local setup has its own CA, which `make up local`, `make up kind` or `make up vagrant`
creates once and keeps over all runs, so it only has to be trusted once.

- The kind- and the vagrant-setup serve every endpoint over https with certificates of their CA.
- The docker-compose setup serves its api over plain http, but the proxies of torii, over which
  the clients reach the sakura-hosts, only work over https: torii picks the sakura-host by the
  server name of the TLS-connection and passes the connection through unchanged. So the clients
  get the certificate of the external api of the sakura-host, which is signed by the CA of the
  setup.

Trusted, the browser and the clients accept all endpoints of the setup, also the proxies of torii
(`https://<vm-uuid>.<domain>:10042`), without any exceptions. Without it, the clients have to
skip the verification (`ainarictl --insecure`), and the dashboard fails with
*API did not respond* for everything, which is done on a sakura-host, like *Show tasks*, starting,
stopping or snapshotting a virtual machine. In the kind- and the vagrant-setup, it can't even log
in. Accepting a certificate in the browser doesn't help: every virtual machine has its own proxy
with its own name, and the browser talks to the api on other ports, whose certificates would have
to be accepted separately.

| Setup   | CA                                              | Name                | Valid only for                                                              |
| ------- | ----------------------------------------------- | ------------------- | --------------------------------------------------------------------------- |
| local   | `temporary_files/local/ainari-local-ca.crt`     | `ainari-local-ca`   | names below `localhost`, like the proxies `*.ainari.localhost`              |
| kind    | `temporary_files/kind/ainari-kind-ca.crt`       | `ainari-kind-ca`    | `127.0.0.1`, `localhost`, names below `cluster.local`                       |
| vagrant | `temporary_files/vagrant/ainari-vagrant-ca.crt` | `ainari-vagrant-ca` | `192.168.56.0/24`, names below `cluster.local` and `192-168-56-11.sslip.io` |

The CAs are limited by name-constraints to the addresses of their setup, so they can't be misused
for any other host, even if their key leaks. Deleting `temporary_files/local`,
`temporary_files/kind` or `temporary_files/vagrant` creates a new CA with the next start of the
setup, which then has to be trusted again. The same happens, when a new version of the setup
changes the name-constraints: the setup-script prints a warning in this case.

The commands below are the same for all setups. They are run in the root of the repository with
the name of the setup in `SETUP`:

```bash
SETUP=local      # or: SETUP=kind, SETUP=vagrant
```

The setup-scripts run as root, so the CA belongs to root, but it is readable by everyone.

## Trust-store of the system

The trust-store of the system (debian/ubuntu) is used by curl and the go-cli, so `ainarictl` works
without `--insecure`. The file has to end with `.crt` and has to be in
`/usr/local/share/ca-certificates`, otherwise it is skipped. `update-ca-certificates` reports
`1 added`:

```bash
sudo cp temporary_files/$SETUP/ainari-$SETUP-ca.crt /usr/local/share/ca-certificates/ainari-$SETUP-ca.crt && sudo update-ca-certificates
```

While the setup is running, the api then answers without skipping the verification, for example
`curl https://127.0.0.1:11417/v1alpha/is_ready` (kind) or
`curl https://192.168.56.10:11417/v1alpha/is_ready` (vagrant). The proxies are checked with the
`torii_address` of a virtual machine, for example
`curl https://<vm-uuid>.ainari.localhost:10042/v1alpha/version` (local, kind). To remove the CA
again:

```bash
sudo rm /usr/local/share/ca-certificates/ainari-$SETUP-ca.crt && sudo update-ca-certificates --fresh
```

## Firefox

Firefox and Chromium don't use the trust-store of the system on linux, but their own
nss-databases, so the CA has to be added to them separately, even if it is already in the one of
the system.

An enterprise-policy imports the CAs at every start into every profile of Firefox. It uses the
files of the trust-store of the system, so that step has to be done first. The command overwrites
an existing `/etc/firefox/policies/policies.json`, so check before, that there is none yet.

```bash
sudo mkdir -p /etc/firefox/policies && echo '{"policies":{"Certificates":{"Install":["/usr/local/share/ca-certificates/ainari-'$SETUP'-ca.crt"]}}}' | sudo tee /etc/firefox/policies/policies.json
```

To trust the CAs of several setups, all files have to be in the trust-store of the system and all
paths in the list:

```bash
sudo mkdir -p /etc/firefox/policies && echo '{"policies":{"Certificates":{"Install":["/usr/local/share/ca-certificates/ainari-local-ca.crt","/usr/local/share/ca-certificates/ainari-kind-ca.crt","/usr/local/share/ca-certificates/ainari-vagrant-ca.crt"]}}}' | sudo tee /etc/firefox/policies/policies.json
```

Firefox has to be closed completely and started again afterwards; `about:policies` shows the
policy as active. Without the policy, the CA can be imported by hand: *Settings → Privacy &
Security → Certificates → View Certificates → Authorities → Import*, then choose *Trust this CA to
identify websites*. To remove the policy again: `sudo rm /etc/firefox/policies/policies.json`; the
CAs, which Firefox imported already, stay in its store, until they are deleted under
*Authorities*.

## Chrome and Chromium

Chrome and Chromium use the nss-database of the user in `~/.pki/nssdb` (the snap of Chromium uses
`~/snap/chromium/current/.pki/nssdb` instead). `certutil` is part of `libnss3-tools`:

```bash
sudo apt install libnss3-tools
certutil -d sql:$HOME/.pki/nssdb -A -t "C,," -n ainari-$SETUP-ca -i temporary_files/$SETUP/ainari-$SETUP-ca.crt
```

The browser has to be restarted completely afterwards; `certutil -d sql:$HOME/.pki/nssdb -L`
lists the CA as `ainari-local-ca`, `ainari-kind-ca` or `ainari-vagrant-ca` with the trust `C,,`.
Without `certutil`, the CA can be imported by hand in *Settings → Privacy and security → Security
→ Manage certificates* under the authorities, with *Trust this certificate for identifying
websites*. To remove it again:

```bash
certutil -d sql:$HOME/.pki/nssdb -D -n ainari-$SETUP-ca
```

## Python-sdk

The python-sdk uses the certificates of `certifi` and not the ones of the system, so it needs the
CA explicitly:

```bash
export REQUESTS_CA_BUNDLE=temporary_files/$SETUP/ainari-$SETUP-ca.crt
```

The end-to-end test `testing/ainari_test/vm_lifecycle_test.py` and
`testing/local_stack/prepare_resources.py` skip the verification anyway.
