# Quick start

Runs the whole stack locally with docker-compose. See the [docker-compose
setup](/developer/local_testing/docker_compose_setup/#running-from-the-tools-container)
for details.

## Requirements

- Linux host with docker
- `/dev/kvm` and `/dev/net/tun`
- kernel with eBPF/XDP support (See [list of minimal supported distributions](/developer/build_guide/#requirements))
- at least 4 GiB free memory for the two test virtual machines

## Deploy

In the root of the repository, build the toolbox image and start it:

```bash
docker build -f dockerfiles/Dockerfile_local_test_tools -t ainari/local-test-tools .

docker run --rm -it --privileged --network host --pid host \
    -e HOST_UID=$(id -u) -e HOST_GID=$(id -g) \
    -v /var/run/docker.sock:/var/run/docker.sock \
    -v "$PWD:$PWD" -w "$PWD" \
    ainari/local-test-tools
```

Within the container, start the stack:

```bash
make up local
```

## Trust the CA of the setup

The virtual machines are managed over the proxies of torii, which are only reachable over https
(`https://<vm-uuid>.ainari.localhost:10042`). Their certificate is signed by the CA
`temporary_files/local/ainari-local-ca.crt`, which `make up local` creates with its first start
and keeps over all runs. Trust it once for the system, so `curl` and the go-cli on the host
accept the proxies:

!!! info

    Run this command on the host, in a second terminal in the root of the repository, and not
    within the container. The certificate-store of the container is not used by the tools on the
    host.

```bash
sudo cp temporary_files/local/ainari-local-ca.crt /usr/local/share/ca-certificates/ainari-local-ca.crt && sudo update-ca-certificates
```

The browser of the dashboard has its own certificate-store, see
[Use the dashboard](#use-the-dashboard). For more details see
[The CAs of the local setups](/developer/local_testing/https_ca/).

## Create virtual machines

Still within the container, create an ssh-key, an image, a network and two virtual machines with
floating ip-addresses:

```bash
python3 testing/local_stack/prepare_resources.py
```

## Access the virtual machines

At the end, the script prints one ssh-command for each virtual machine, which also works from
the host:

```bash
ssh -i temporary_files/local_stack_test/id_ed25519 ubuntu@<FLOATING_IP>
```

for the 2 created example VMs:

```bash
ssh -i temporary_files/local_stack_test/id_ed25519 ubuntu@10.0.0.2
ssh -i temporary_files/local_stack_test/id_ed25519 ubuntu@10.0.0.3
```

## Use the CLI

Within the container, build the CLI and point it at the stack:

```bash
cd src/cli/ainarictl && go build .
source test_auth.sh    # user 'asdf', passphrase 'asdfasdf'
./ainarictl vm list
./ainarictl --help
```

The trust-store of the container doesn't know the CA of the setup, so the commands, which are
done on a virtual machine over its proxy (`vm create`, `vm start`, `vm stop`, `vm reboot` and the
`task`-commands), need `--insecure` within the container:

```bash
./ainarictl --insecure task list <VIRTUAL_MACHINE_UUID>
```

On the host with the trusted CA, they work without it.

## Use the dashboard

!!! info

    The following steps have to be done on the host, NOT within the docker container from above.

Create the config of the dashboard and start it:

```bash
sudo mkdir -p /etc/ainari
echo '{"apiUrl": "http://localhost:11417"}' | sudo tee /etc/ainari/dashboard_config.json

cd src/dashboard
docker compose up --build
```

The dashboard reaches the virtual machines over their proxies, so the CA of the setup (see
[Trust the CA of the setup](#trust-the-ca-of-the-setup)) has to be trusted in the browser as well.
On the host, add it once to the certificate-store of Chrome and Chromium (`certutil` is part of
`libnss3-tools`):

```bash
sudo apt install libnss3-tools
certutil -d sql:$HOME/.pki/nssdb -A -t "C,," -n ainari-local-ca -i temporary_files/local/ainari-local-ca.crt
```

For Firefox and the snap-version of Chromium see
[The CAs of the local setups](/developer/local_testing/https_ca/). Restart the browser completely
afterwards. Without the trusted CA, the dashboard fails with *API did not respond* for everything,
which is done on a virtual machine, like *Show tasks*.

Open [http://localhost:5173](http://localhost:5173) and log in with `asdf` / `asdfasdf`.

## Stop

Stop the dashboard with `Ctrl+C` and the stack within the toolbox container with:

```bash
make down local
```
