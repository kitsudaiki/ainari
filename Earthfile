# Copyright 2022 Tobias Anker <tobias.anker@kitsunemimim.moe>

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


cppcheck:
    RUN apt-get update && \
        apt-get install -y cppcheck
    COPY src src
    RUN rm -rf \
          src/libs/protobuf/hanami_messages.proto3.pb.h
    RUN cppcheck --error-exitcode=1 src/libs/cpp


clang-format:
    RUN apt-get update && \
        apt-get install -y clang-format-15
    COPY src src
    RUN rm -rf \
          src/sdk/python \
          src/third-party-libs \
          src/libs/protobuf/hanami_messages.proto3.pb.h
    COPY .clang-format .
    RUN find . -regex '.*\.\(h$\|c$\|hpp$\|cpp$\)' | while read f; do \
              clang-format-15 -style=file:.clang-format --dry-run --Werror $f; \
              if [ $? -ne 0 ]; then \
                  exit 1; \
              fi; done


flake8:
    RUN apt-get update && \
        apt-get install -y python3 python3-pip python3-venv && \
        python3 -m venv flake8_env
    ENV PATH="/code/flake8_env/bin:$PATH"
    RUN pip3 install flake8
    COPY src src
    COPY testing testing
    COPY .flake8 .flake8
    RUN rm -rf src/sdk/python/hanami_sdk/hanami_sdk/hanami_messages/proto3_pb2.py src/sdk/python/hanami_sdk/hanami_env src/sdk/python/hanami_sdk/build
    RUN flake8 testing/python_sdk_api/sdk_api_test.py && \
        flake8 src/sdk/python


ansible-lint:
    RUN apt-get update && \
        apt-get install -y python3 python3-pip python3-venv && \
        python3 -m venv lint_env
    ENV PATH="/code/lint_env/bin:$PATH"
    RUN pip3 install ansible-lint
    COPY deploy deploy
    COPY .ansible-lint .ansible-lint
    RUN ansible-lint deploy/ansible/openhanami


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
                           make \
                           cmake \
                           bison \
                           flex \
                           git \
                           ssh \
                           nlohmann-json3-dev \
                           rustup \
                           # TODO:enable gpu init here again
                           # related issue: https://github.com/kitsudaiki/Hanami/issues/325
                           # nvidia-cuda-toolkit \
                           nano && \
        ln -s /usr/bin/clang++-19 /usr/bin/clang++ && \
        ln -s /usr/bin/clang-19 /usr/bin/clang && \
        rustup install stable --no-self-update
    # copy current code into the docker-container
    COPY . .


compile-cli:
    RUN apt-get update && \
        apt-get install -y wget protobuf-compiler golang-goprotobuf-dev && \
        wget -c https://go.dev/dl/go1.22.5.linux-amd64.tar.gz && \
        tar -C /usr/local/ -xzf go1.22.5.linux-amd64.tar.gz
    COPY src src
    RUN cd ./src/sdk/go/hanami_sdk && \
        protoc --go_out=. --proto_path ../../../libs/protobuf hanami_messages.proto3
    RUN cd src/cli/hanamictl && \
        /usr/local/go/bin/go build .
    SAVE ARTIFACT ./src/cli/hanamictl/hanamictl /tmp/hanamictl


compile-core-lib:
    FROM +prepare-build-dependencies
    RUN cmake -DCMAKE_BUILD_TYPE=Release -Drun_tests=ON  .
    RUN make -j8
    RUN mkdir /tmp/hanami_core && \
        find src -type f -executable -exec cp {} /tmp/hanami_core \;
    SAVE ARTIFACT /tmp/hanami_core /tmp/hanami_core
    SAVE ARTIFACT /tmp/hanami_core AS LOCAL hanami_core

compile-core-lib-debug:
    FROM +prepare-build-dependencies
    RUN cmake -DCMAKE_BUILD_TYPE=Debug -Drun_tests=ON  .
    RUN make -j8
    RUN mkdir /tmp/hanami_core && \
        find src -type f -executable -exec cp {} /tmp/hanami_core \;
    SAVE ARTIFACT /tmp/hanami_core /tmp/hanami_core
    SAVE ARTIFACT /tmp/hanami_core AS LOCAL hanami_core

compile-hanami:
    FROM +prepare-build-dependencies
    RUN apt-get update && \
        apt-get install -y libsqlite3-dev
    RUN cargo build
    RUN mkdir /tmp/hanami
    SAVE ARTIFACT ./target/debug/hanami /tmp/hanami

test-hanami:
    FROM +prepare-build-dependencies
    RUN apt-get update && \
        apt-get install -y libsqlite3-dev
    RUN cargo test


build-image:
    ARG image_name

    RUN apt-get update && \
        apt-get install -y openssl libuuid1 libcrypto++8 libsqlite3-0 libprotobuf23 libboost1.74 && \
        apt-get clean autoclean && \
        apt-get autoremove --yes && \
        chmod +x /usr/bin/hanami

    COPY +compile-code/hanami_core/hanami /usr/bin/

    # run hanami
    ENTRYPOINT ["/usr/bin/hanami"]

    SAVE IMAGE "$image_name"


generate-docs:
    COPY +compile-code/hanami/hanami /tmp/

    RUN apt-get update && \
        apt-get install -y openssl libuuid1 libcrypto++8 libsqlite3-0 libprotobuf23 libboost1.74 libgbm-dev libasound2 xvfb dbus
    RUN chmod +x /tmp/hanami_core
    RUN /tmp/hanami_core --generate_docu

    RUN apt-get update && \
        apt-get install -y python3 \
                           python3-pip \
                           wget \
                           curl && \
        pip3 install mkdocs \
                     mkdocs-material \
                     mkdocs-swagger-ui-tag \
                     # pin mkdocs-drawio-exporter because 0.10.x is broken
                     mkdocs-drawio-exporter==0.9.1 && \
        curl -s https://api.github.com/repos/jgraph/drawio-desktop/releases/latest | grep browser_download_url | grep "amd64"  | grep "deb" | cut -d "\"" -f 4 | wget -i - && \
        apt -f -y install ./drawio-amd64-*.deb

    COPY mkdocs.yml .
    COPY CHANGELOG.md .
    COPY ROADMAP.md .
    COPY LICENSE .
    COPY docs docs
    RUN cp ./db.md docs/backend/
    RUN cp ./config.md docs/backend/
    RUN cp ./open_api_docu.json docs/frontend/

    # the `xvfb-run -a` comes from the following trouble-shooting for a headless execution in github actions:
    # https://github.com/LukeCarrier/mkdocs-drawio-exporter?tab=readme-ov-file#headless-usage
    RUN xvfb-run -a mkdocs build --clean

    SAVE ARTIFACT site AS LOCAL site


build-docs:
    ARG image_name

    RUN apt-get update && \
        apt-get install -y python3

    COPY +generate-docs/site /openhanami_docs

    WORKDIR /openhanami_docs

    RUN useradd -m ubuntu
    RUN chown -R ubuntu:ubuntu .
    USER ubuntu

    CMD python3 -m http.server 8000

    SAVE IMAGE "$image_name"
