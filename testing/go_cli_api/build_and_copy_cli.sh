#!/bin/bash

# build protobuffer for go sdk
#pushd ../../src/sdk/go/ainari_sdk
#protoc --go_out=. --proto_path ../../../libs/protobuf ainari_messages.proto3
#popd

# build cli-binarygolangci-lint
pushd ../../src/cli/ainarictl
go build .
popd
cp ../../src/cli/ainarictl/ainarictl .

