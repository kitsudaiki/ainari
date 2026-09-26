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

# Starts and stops the local setups, which run the whole stack on this machine.
#
#   make up local      docker-compose setup (scripts/setup_local_stack.sh, runs with sudo)
#   make down local
#   make up kind       the same setup on a kind-cluster (scripts/setup_kind_stack.sh)
#   make down kind
#   make up vagrant    the same setup on a k3s-cluster of four virtual machines
#   make down vagrant  (scripts/setup_vagrant_stack.sh)

# kind is downloaded into temporary_files, if it is not installed
KIND_VERSION ?= v0.30.0
BIN_DIR := temporary_files/bin
KIND_ARCH := $(if $(filter aarch64 arm64,$(shell uname -m)),arm64,amd64)
KIND ?= $(or $(shell command -v kind 2> /dev/null),$(BIN_DIR)/kind)

# ansible is installed into a virtual environment in temporary_files, if it is not installed
ANSIBLE_CORE_VERSION ?= 2.20.9
ANSIBLE_VENV := temporary_files/ansible-venv
ANSIBLE_PLAYBOOK ?= $(or $(shell command -v ansible-playbook 2> /dev/null),$(ANSIBLE_VENV)/bin/ansible-playbook)

# the setup, which 'up' and 'down' act on, is given as second goal
STACK := $(filter local kind vagrant,$(MAKECMDGOALS))

.PHONY: help up down local kind vagrant

help:
	@echo "Usage:"
	@echo "    make up local      start the docker-compose setup (needs sudo)"
	@echo "    make down local    stop the docker-compose setup"
	@echo "    make up kind       start the setup on a kind-cluster (asks for sudo)"
	@echo "    make down kind     delete the kind-cluster"
	@echo "    make up vagrant    start the setup on four virtual machines (asks for sudo)"
	@echo "    make down vagrant  destroy the virtual machines"

up down: $(if $(filter kind,$(STACK)),$(KIND)) $(if $(filter vagrant,$(STACK)),$(ANSIBLE_PLAYBOOK))
	@case "$(STACK)" in \
		local) sudo ./scripts/setup_local_stack.sh $(if $(filter down,$@),--down) ;; \
		kind) KIND="$(abspath $(KIND))" ./scripts/setup_kind_stack.sh $(if $(filter down,$@),--down) ;; \
		vagrant) PATH="$(abspath $(dir $(ANSIBLE_PLAYBOOK))):$$PATH" ./scripts/setup_vagrant_stack.sh $(if $(filter down,$@),--down) ;; \
		*) echo "Usage: make $@ local|kind|vagrant"; exit 1 ;; \
	esac

# 'local', 'kind' and 'vagrant' are only the arguments of 'up' and 'down'
local kind vagrant:
	@$(if $(filter up down,$(MAKECMDGOALS)),:,echo "Usage: make up|down $@"; exit 1)

$(BIN_DIR)/kind:
	mkdir -p $(BIN_DIR)
	curl -fsSLo $@ https://kind.sigs.k8s.io/dl/$(KIND_VERSION)/kind-linux-$(KIND_ARCH)
	chmod +x $@

$(ANSIBLE_VENV)/bin/ansible-playbook:
	python3 -m venv $(ANSIBLE_VENV)
	$(ANSIBLE_VENV)/bin/pip install --quiet ansible-core==$(ANSIBLE_CORE_VERSION)
