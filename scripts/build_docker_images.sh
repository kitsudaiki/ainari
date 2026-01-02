#!/bin/bash

# Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>
#
# Licensed under the Apache License, Version 2.0 (the "License")
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#    http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

docker build -f dockerfiles/Dockerfile_ainari_base -t kitsudaiki/ainari_base:latest .

docker build -f dockerfiles/Dockerfile_hanami    -t kitsudaiki/hanami:local_test .
docker build -f dockerfiles/Dockerfile_miko      -t kitsudaiki/miko:local_test .
docker build -f dockerfiles/Dockerfile_omamori   -t kitsudaiki/omamori:local_test .
docker build -f dockerfiles/Dockerfile_onsen     -t kitsudaiki/onsen:local_test .
docker build -f dockerfiles/Dockerfile_ryokan    -t kitsudaiki/ryokan:local_test .
docker build -f dockerfiles/Dockerfile_sakura    -t kitsudaiki/sakura:local_test .
docker build -f dockerfiles/Dockerfile_torii     -t kitsudaiki/torii:local_test .
docker build -f dockerfiles/Dockerfile_dashboard -t kitsudaiki/ainari_dashboard:local_test .

mkdir -p temporary_files
docker save -o temporary_files/ainari_docker_files.tar \
    kitsudaiki/hanami:local_test \
    kitsudaiki/miko:local_test \
    kitsudaiki/omamori:local_test \
    kitsudaiki/onsen:local_test \
    kitsudaiki/ryokan:local_test \
    kitsudaiki/sakura:local_test \
    kitsudaiki/torii:local_test \
    kitsudaiki/ainari_dashboard:local_test
