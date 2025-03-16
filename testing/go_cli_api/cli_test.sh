#!/bin/bash
# export HANAMI_ADDRESS=http://127.0.0.1:11418
# export HANAMI_USER=asdf
# export HANAMI_PASSPHRASE=asdfasdf

# export train_inputs=/home/neptune/Schreibtisch/mnist/train-images.idx3-ubyte
# export train_labels=/home/neptune/Schreibtisch/mnist/train-labels.idx1-ubyte
# export request_inputs=/home/neptune/Schreibtisch/mnist/t10k-images.idx3-ubyte
# export request_labels=/home/neptune/Schreibtisch/mnist/t10k-labels.idx1-ubyte


# build protobuffer for go sdk
# pushd ../../src/sdk/go/hanami_sdk
# protoc --go_out=. --proto_path ../../../libraries/hanami_messages/protobuffers hanami_messages.proto3
# popd

# build cli-binarygolangci-lint
# pushd ../../src/cli/hanamictl
# go build .
# popd
# cp ../../src/cli/hanamictl/hanamictl .

# cleanup before running tests
./hanamictl project delete --insecure cli_test_project
./hanamictl user delete --insecure cli_test_user

########################
echo ""
echo "project tests"
./hanamictl project create --insecure -n "cli test project" cli_test_project
./hanamictl project get --insecure cli_test_project
./hanamictl project list --insecure
./hanamictl project delete --insecure cli_test_project

########################
echo ""
echo "user tests"
./hanamictl user create --insecure -n "cli test user" -p "asdfasdf" cli_test_user
./hanamictl user get --insecure cli_test_user
./hanamictl user list --insecure
./hanamictl user delete --insecure cli_test_user

########################
echo ""
echo "dataset tests"
dataset_uuid=$(./hanamictl dataset create mnist --insecure -j -i $train_inputs -l $train_labels cli_test_dataset | jq -r '.uuid')
./hanamictl dataset get --insecure $dataset_uuid
./hanamictl dataset list --insecure
./hanamictl dataset delete --insecure $dataset_uuid

########################
echo ""
echo "cluster tests"
cluster_uuid=$(./hanamictl cluster create --insecure -j -t ./cluster_template cli_test_cluster | jq -r '.uuid')
./hanamictl cluster get --insecure $cluster_uuid
./hanamictl cluster list --insecure
./hanamictl cluster delete --insecure $cluster_uuid


########################
echo ""
echo "workfloat tests"
./hanamictl host list  --insecure

train_dataset_uuid=$(./hanamictl dataset create mnist --insecure -j -i $train_inputs -l $train_labels cli_test_dataset_train | jq -r '.uuid')
echo "Train-Dataset-UUID: $train_dataset_uuid"

request_dataset_uuid=$(./hanamictl dataset create mnist --insecure -j -i $request_inputs -l $request_labels cli_test_dataset_req | jq -r '.uuid')
echo "Request-Dataset-UUID: $request_dataset_uuid"

cluster_uuid=$(./hanamictl cluster create --insecure -j -t ./cluster_template cli_test_cluster | jq -r '.uuid')
echo "Cluster-UUID: $cluster_uuid"


# train test
echo "./hanamictl task create train --insecure -j -i $train_dataset_uuid:picture:picture_hex -o $train_dataset_uuid:label:label_hex -c $cluster_uuid cli_train_test_task"
task_uuid=$(./hanamictl task create train --insecure -j -i $train_dataset_uuid:picture:picture_hex -o $train_dataset_uuid:label:label_hex -c $cluster_uuid cli_train_test_task | jq -r '.uuid')
echo "Train-Task-UUID: $task_uuid"

while true; do
    ./hanamictl task get --insecure -c $cluster_uuid $task_uuid
    state=$(./hanamictl task get --insecure -j -c $cluster_uuid $task_uuid | jq -r '.state')
    if [[ "$state" == *"finished"* ]]; then
        echo "Process finished. Exiting loop."
        break
    fi
    sleep 1
done
./hanamictl task get --insecure -c $cluster_uuid $task_uuid


# save and restore test
./hanamictl cluster save --insecure -n cli_test_checkpoint $cluster_uuid
checkpoint_uuid=$(./hanamictl cluster save --insecure -j -n cli_test_checkpoint $cluster_uuid  | jq -r '.uuid')
sleep 1
echo "Checkpoint-UUID: $checkpoint_uuid"
./hanamictl checkpoint list --insecure

./hanamictl cluster delete --insecure $cluster_uuid
cluster_uuid=$(./hanamictl cluster create --insecure -j -t ./cluster_template cli_test_cluster | jq -r '.uuid')
echo "new Cluster-UUID: $cluster_uuid"
./hanamictl cluster restore --insecure -c $checkpoint_uuid $cluster_uuid

# request test
echo "./hanamictl task create request --insecure -j -i $request_dataset_uuid:picture:picture_hex -r label_hex:cli_test_output -c $cluster_uuid cli_request_test_task"
req_task_uuid=$(./hanamictl task create request --insecure -j -i $request_dataset_uuid:picture:picture_hex -r label_hex:cli_test_output -c $cluster_uuid cli_request_test_task | jq -r '.uuid')
echo "Request-Task-UUID: $req_task_uuid"

./hanamictl task list --insecure -c $cluster_uuid 
echo "$taskUuid"
./hanamictl task delete --insecure -c $cluster_uuid $task_uuid

while true; do
    ./hanamictl task get --insecure -c $cluster_uuid $req_task_uuid
    state=$(./hanamictl task get --insecure -j -c $cluster_uuid $req_task_uuid | jq -r '.state')
    if [[ "$state" == *"finished"* ]]; then
        echo "Process finished. Exiting loop."
        break
    fi
    sleep 1
done
./hanamictl task get --insecure -c $cluster_uuid $req_task_uuid

./hanamictl dataset list --insecure

# echo "debug $(./hanamictl dataset list --insecure -j)"
result_uuid=$(./hanamictl dataset list --insecure -j | jq -r '.body[] | select(.[5] == "cli_request_test_task") | .[1]')
echo "Result-Dataset-UUID: $result_uuid"

./hanamictl dataset get --insecure $result_uuid

./hanamictl dataset check --insecure -r $request_dataset_uuid $result_uuid

content=$(./hanamictl dataset content --insecure -j -c cli_test_output -o 100 -n 10 $result_uuid | jq -r 'length')
if [[ "$content" != 10 ]]; then
    echo "content as length of $content instead of 10"
fi

# clear all test-resources
./hanamictl checkpoint delete --insecure $checkpoint_uuid
./hanamictl cluster delete --insecure $cluster_uuid
./hanamictl dataset delete --insecure $train_dataset_uuid
./hanamictl dataset delete --insecure $request_dataset_uuid
./hanamictl dataset delete --insecure $result_uuid
