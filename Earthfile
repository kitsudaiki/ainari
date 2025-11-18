# Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at

#     http://www.apache.org/licenses/LICENSE-2.0

# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

VERSION 0.8
FROM ubuntu:24.04

# configure apt to be noninteractive
ENV DEBIAN_FRONTEND noninteractive
ENV DEBCONF_NONINTERACTIVE_SEEN true

WORKDIR /code

local-setup:
    LOCALLY
    RUN git config --local core.hooksPath .githooks


flake8:
    RUN apt-get update && \
        apt-get install -y python3 python3-pip python3-venv && \
        python3 -m venv flake8_env
    ENV PATH="/code/flake8_env/bin:$PATH"
    RUN pip3 install flake8
    COPY src src
    COPY testing testing
    COPY .flake8 .flake8
    RUN rm -rf src/sdk/python/ainari_sdk/ainari_sdk/ainari_messages/proto3_pb2.py src/sdk/python/ainari_sdk/ainari_env src/sdk/python/ainari_sdk/build
    RUN flake8 src/sdk/python


ansible-lint:
    RUN apt-get update && \
        apt-get install -y python3 python3-pip python3-venv && \
        python3 -m venv lint_env
    ENV PATH="/code/lint_env/bin:$PATH"
    RUN pip3 install ansible-lint
    COPY deploy deploy
    COPY .ansible-lint .ansible-lint
    RUN ansible-lint deploy/ansible/ainari


secret-scan:
    RUN apt-get update && \
        apt-get install -y python3 python3-pip git python3-venv && \
        python3 -m venv check_env
    ENV PATH="/code/check_env/bin:$PATH"
    RUN pip3 install detect-secrets
    COPY . .
    RUN git ls-files -z | xargs -0 detect-secrets-hook --baseline .secrets.baseline


prepare-build-dependencies:
    # install dependencies
    RUN apt-get update && \
        apt-get install -y clang-19 \
                           g++ \
                           git \
                           ssh \
                           pkg-config \
                           openssl \
                           libssl-dev \
                           rustup \
                           # TODO:enable gpu init here again
                           # related issue: https://github.com/kitsudaiki/Sakura/issues/325
                           # nvidia-cuda-toolkit \
                           nano && \
        rustup install stable --no-self-update
    # copy current code into the docker-container
    COPY . .


compile-cli:
    RUN apt-get update && \
        apt-get install -y wget protobuf-compiler golang-goprotobuf-dev && \
        wget -c https://go.dev/dl/go1.22.5.linux-amd64.tar.gz && \
        tar -C /usr/local/ -xzf go1.22.5.linux-amd64.tar.gz
    COPY src src
    # RUN cd ./src/sdk/go/ainari_sdk && \
    #     protoc --go_out=. --proto_path ../../../libs/protobuf ainari_messages.proto3
    RUN cd src/cli/ainarictl && \
        /usr/local/go/bin/go build .
    SAVE ARTIFACT ./src/cli/ainarictl/ainarictl /tmp/ainarictl

compile-ainari:
    FROM +prepare-build-dependencies
    RUN apt-get update && \
        apt-get install -y protobuf-compiler  libsqlite3-dev
    RUN cargo build
    RUN mkdir /tmp/ainari/
    RUN cp ./target/debug/sakura /tmp/ainari/
    RUN cp ./target/debug/miko /tmp/ainari/
    RUN cp ./target/debug/ryokan /tmp/ainari/
    RUN cp ./target/debug/hanami /tmp/ainari/
    RUN cp ./target/debug/torii /tmp/ainari/
    RUN cp ./target/debug/omamori /tmp/ainari/
    RUN cp ./target/debug/onsen /tmp/ainari/
    SAVE ARTIFACT /tmp/ainari /tmp/ainari
    SAVE ARTIFACT /tmp/ainari AS LOCAL ainari

test-hanami:
    FROM +prepare-build-dependencies
    COPY example_configs/ainari /etc/ainari
    RUN apt-get update && \
        apt-get install -y protobuf-compiler libsqlite3-dev libssl-dev
    # only one test-thread to avoid conflicts between tests, which access the same singleton
    RUN cargo test -- --test-threads=1


generate-docs:
    ENV AINARI_ADMIN_ID asdf
    ENV AINARI_ADMIN_NAME asdf
    ENV AINARI_ADMIN_PASSPHRASE asdfasdf

    COPY +compile-ainari/ainari/sakura /tmp/sakura
    COPY +compile-ainari/ainari/miko /tmp/miko
    COPY +compile-ainari/ainari/ryokan /tmp/ryokan
    COPY +compile-ainari/ainari/hanami /tmp/hanami
    COPY +compile-ainari/ainari/torii /tmp/torii
    COPY +compile-ainari/ainari/omamori /tmp/omamori
    COPY +compile-ainari/ainari/onsen /tmp/onsen
    COPY example_configs/ainari /etc/ainari

    RUN apt-get update && \
        apt-get install -y protobuf-compiler openssl libsqlite3-0 libgbm-dev xvfb dbus

    RUN apt-get update && \
        apt-get install -y python3 \
                           python3-pip \
                           python3-venv \
                           wget \
                           curl && \
        python3 -m venv ainari_env && \
        . ainari_env/bin/activate && \
        pip3 install hapless \
                     mkdocs \
                     mkdocs-material \
                     mkdocs-swagger-ui-tag \
                     mkdocs-drawio-exporter && \
        wget https://github.com/jgraph/drawio-desktop/releases/download/v28.2.5/drawio-amd64-28.2.5.deb && \
        apt -f -y install ./drawio-amd64-*.deb

    RUN chmod +x /tmp/miko
    RUN . ainari_env/bin/activate && \
        hap run /tmp/miko && \
        sleep 5 && \
        curl 127.0.0.1:11417/openapi.json > ./open_api_docu_miko.json
    RUN chmod +x /tmp/ryokan
    RUN . ainari_env/bin/activate && \
        hap run /tmp/ryokan && \
        sleep 5 && \
        curl 127.0.0.1:11416/openapi.json > ./open_api_docu_ryokan.json
    RUN chmod +x /tmp/hanami
    RUN . ainari_env/bin/activate && \
        hap run /tmp/hanami && \
        sleep 5 && \
        curl 127.0.0.1:11418/openapi.json > ./open_api_docu_hanami.json
    RUN chmod +x /tmp/torii
    RUN . ainari_env/bin/activate && \
        hap run /tmp/torii && \
        sleep 5 && \
        curl 127.0.0.1:11419/openapi.json > ./open_api_docu_torii.json
    RUN chmod +x /tmp/omamori
    RUN . ainari_env/bin/activate && \
        hap run /tmp/omamori && \
        sleep 5 && \
        curl 127.0.0.1:11421/openapi.json > ./open_api_docu_omamori.json
    # RUN chmod +x /tmp/sakura
    # RUN . ainari_env/bin/activate && \
    #     hap run /tmp/sakura && \
    #     sleep 5 && \
    #     curl 127.0.0.1:11420/openapi.json > ./open_api_docu_sakura.json

    COPY mkdocs.yml .
    COPY CHANGELOG.md .
    COPY ROADMAP.md .
    COPY LICENSE .
    COPY docs docs
    # RUN cp ./open_api_docu_sakura.json docs/frontend/
    RUN cp ./open_api_docu_miko.json docs/frontend/
    RUN cp ./open_api_docu_ryokan.json docs/frontend/
    RUN cp ./open_api_docu_hanami.json docs/frontend/
    RUN cp ./open_api_docu_torii.json docs/frontend/
    RUN cp ./open_api_docu_omamori.json docs/frontend/

    # the `xvfb-run -a` comes from the following trouble-shooting for a headless execution in github actions:
    # https://github.com/LukeCarrier/mkdocs-drawio-exporter?tab=readme-ov-file#headless-usage
    RUN . ainari_env/bin/activate && xvfb-run -a mkdocs build --clean

    SAVE ARTIFACT site AS LOCAL site


build-docs:
    ARG image_name

    RUN apt-get update && \
        apt-get install -y python3

    COPY +generate-docs/site /ainari_docs

    WORKDIR /ainari_docs

    RUN chown -R ubuntu:ubuntu .
    USER ubuntu

    CMD python3 -m http.server 8000

    SAVE IMAGE "$image_name"
