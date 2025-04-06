#!/bin/bash

# build protobuffer for go sdk
pushd ../../src/sdk/go/hanami_sdk
protoc --go_out=. --proto_path ../../../libs/protobuf hanami_messages.proto3
popd

# build cli-binarygolangci-lint
pushd ../../src/cli/hanamictl
go build .
popd
cp ../../src/cli/hanamictl/hanamictl .

