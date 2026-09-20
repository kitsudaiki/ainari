# How to build

## Preparation

- Install packages

    - For Ubuntu 22.04 and 24.04 (Debian should work too):

        ```bash
        apt-get install libsqlite3-dev libssl-dev pkg-config
        ```

- Install Rust-compiler (minimum version: `1.85.1`)

- Clone repository with submodules

    ```bash
    git clone https://github.com/kitsudaiki/ainari.git
    ```

## Build hamami plain

- Compile sakura

    ```bash
    cd ainari

    cargo build --release
    ```

- Resulting binary

    ```bash
    ./target/release/sakura
    ```

## Build sakura as docker-image

Run `docker build -f dockerfiles/Dockerfile_<COMPONENT> -t <DOCKER_IMAGE_NAME> .`

!!! example

    ```bash
    docker build -f dockerfiles/Dockerfile_sakura -t sakura:test .
    ```

## Build CLI-client

- Install [go](https://go.dev/doc/install) (the required version is defined in the
    `go.mod`-file of the cli)

- build the cli

    ```bash
    cd ./src/cli/ainarictl
    go build .
    ```

- the resulting binary `ainarictl` is placed beside the sources within the same directory

## Build python-SDK as package

- install packages

    ```bash
    sudo apt-get update
    sudo apt-get install -y python3 python3-pip
    sudo pip3 install wheel
    ```

<!-- -   build protobuf-messages and package

    ```bash
    cd ./src/sdk/python/ainari_sdk/ainari_sdk
    protoc --python_out=. --proto_path ../../../../libs/protobuf  ainari_messages.proto3
    cd ..
    python3 setup.py bdist_wheel --universal
    ``` -->

## Prechecks

There are a bunch of pre-checks at the beginning of the CI-pipeline, which can fail and where it is
useful to be able to run the same tests locally for debugging.

### Flake8-check

- run `pip3 install flake8` and then `flake8 src/sdk/python`

### Secret-scan

- run `git ls-files -z | xargs -0 detect-secrets-hook --baseline .secrets.baseline`

It is possible, that the check fails, even if there are no (new) secrets in the code and fails,
because of some other code-movements. The check compares all to the `.secrets.baseline`-file, where
also line-numbers are marked. To update the file to get the test green again:

- install [detect-secrets](https://github.com/Yelp/detect-secrets)

- update file with `detect-secrets scan > .secrets.baseline`

<!-- ### Ansible-lint

-   run `ansible-lint deploy/ansible/ainari` -->

## Build docs

- The documenation can be build as image like this:

    ```bash
    docker build -f dockerfiles/Dockerfile_docs -t <DOCKER_IMAGE_NAME> .
    ```

    !!! example

        ```bash
        docker build -f dockerfiles/Dockerfile_docs -t ainari_docs:test .
        ```

- The documentation listen on port 8000 within the docker-container. So the port has to be forwarded
    into the container:

    ```bash
    docker run -p 127.0.0.1:8080:8000 ainari_docs:test
    ```

- After this within the browser the addess `127.0.0.1:8080` can be entered to call the documenation
    within the browser.

## Run preview of docs

- Install Mkdocs and plugins

    ```bash
    pip3 install mkdocs mkdocs-material mkdocs-swagger-ui-tag mkdocs-drawio-exporter
    ```

- To build the documentation `Draw.io` also has to be installed on the system

    - Example how to install draw.io

        ```bash
        curl -s https://api.github.com/repos/jgraph/drawio-desktop/releases/latest | grep browser_download_url | grep "amd64"  | grep "deb" | cut -d "\"" -f 4 | wget -i -

        apt -f -y install ./drawio-amd64-*.deb
        ```

- checkout repository and run the local server

    ```bash
    git clone --recurse-submodules https://github.com/kitsudaiki/ainari.git
    cd Ainari

    mkdocs serve
    ```

- Open web-browser with address `http://127.0.0.1:8000/` to see the docs. The `mkdocs serve`-command
    runs in the background and makes live-updates of all changes within the files.
