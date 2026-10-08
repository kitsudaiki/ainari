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

IMG ?= kitsudaiki/ainari_operator:develop

# kubernetes-version of the api-server of the tests
ENVTEST_K8S_VERSION ?= 1.35.0

CONTROLLER_TOOLS_VERSION ?= v0.20.1
ENVTEST_VERSION ?= release-0.23

LOCALBIN ?= $(CURDIR)/bin
CONTROLLER_GEN ?= $(LOCALBIN)/controller-gen
SETUP_ENVTEST ?= $(LOCALBIN)/setup-envtest
KUSTOMIZE ?= kubectl kustomize

.PHONY: all
all: build

##@ Development

.PHONY: generate
generate: $(CONTROLLER_GEN) ## Generate the deepcopy-functions of the API.
	$(CONTROLLER_GEN) object:headerFile=hack/boilerplate.go.txt paths=./api/...

.PHONY: manifests
manifests: $(CONTROLLER_GEN) ## Generate the CRD and the RBAC-role.
	$(CONTROLLER_GEN) crd rbac:roleName=ainari-operator paths=./... \
		output:crd:artifacts:config=config/crd/bases output:rbac:artifacts:config=config/rbac

.PHONY: fmt
fmt: ## Format the code.
	go fmt ./...

.PHONY: vet
vet: ## Check the code with go vet.
	go vet ./...

.PHONY: test
test: generate manifests fmt vet $(SETUP_ENVTEST) ## Run all tests, including the ones with an api-server.
	KUBEBUILDER_ASSETS="$$($(SETUP_ENVTEST) use $(ENVTEST_K8S_VERSION) --bin-dir $(LOCALBIN) -p path)" \
		go test ./... -coverprofile cover.out

##@ Build

.PHONY: build
build: generate fmt vet ## Build the binary of the operator.
	go build -o $(LOCALBIN)/operator ./cmd

.PHONY: run
run: generate fmt vet ## Run the operator against the cluster of the current kube-context.
	go run ./cmd

.PHONY: docker-build
docker-build: ## Build the image of the operator.
	docker build -t $(IMG) .

.PHONY: docker-push
docker-push: ## Push the image of the operator.
	docker push $(IMG)

##@ Deployment

.PHONY: install
install: manifests ## Install the CRD into the cluster of the current kube-context.
	$(KUSTOMIZE) config/crd | kubectl apply --server-side -f -

.PHONY: uninstall
uninstall: manifests ## Remove the CRD and so all Ainari-resources from the cluster.
	$(KUSTOMIZE) config/crd | kubectl delete --ignore-not-found -f -

.PHONY: deploy
deploy: manifests ## Deploy the CRD and the operator with the image IMG.
	$(KUSTOMIZE) config/default | sed 's|kitsudaiki/ainari_operator:develop|$(IMG)|' | kubectl apply --server-side -f -

.PHONY: undeploy
undeploy: ## Remove the operator and the CRD.
	$(KUSTOMIZE) config/default | kubectl delete --ignore-not-found -f -

##@ Tools

$(LOCALBIN):
	mkdir -p $(LOCALBIN)

$(CONTROLLER_GEN): | $(LOCALBIN)
	GOBIN=$(LOCALBIN) go install sigs.k8s.io/controller-tools/cmd/controller-gen@$(CONTROLLER_TOOLS_VERSION)

$(SETUP_ENVTEST): | $(LOCALBIN)
	GOBIN=$(LOCALBIN) go install sigs.k8s.io/controller-runtime/tools/setup-envtest@$(ENVTEST_VERSION)

.PHONY: help
help: ## Show this help.
	@awk 'BEGIN {FS = ":.*##"} /^[a-zA-Z_-]+:.*?##/ { printf "  %-15s %s\n", $$1, $$2 } /^##@/ { printf "\n%s\n", substr($$0, 5) }' $(MAKEFILE_LIST)
