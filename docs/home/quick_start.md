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

## Create virtual machines

Still within the container, create an ssh-key, an image, a network and two virtual machines with
floating ip-addresses:

```bash
python3 testing/local_stack/vm_lifecycle_test.py
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

## Use the dashboard

On the host, create the config of the dashboard and start it:

```bash
sudo mkdir -p /etc/ainari
echo '{"apiUrl": "http://localhost:11417"}' | sudo tee /etc/ainari/dashboard_config.json

cd src/dashboard
docker compose up --build
```

Open [http://localhost:5173](http://localhost:5173) and log in with `asdf` / `asdfasdf`.

## Stop

Stop the dashboard with `Ctrl+C` and the stack within the toolbox container with:

```bash
make down local
```
