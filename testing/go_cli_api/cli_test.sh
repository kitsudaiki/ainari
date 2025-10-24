#!/bin/bash
# export MIKO_ADDRESS=http://127.0.0.1:11417
# export SAKURA_USER=asdf
# export SAKURA_PASSPHRASE=asdfasdf

# export TRAIN_INPUTS=/home/neptune/Schreibtisch/Projects/mnist/train-images-idx3-ubyte
# export TRAIN_LABELS=/home/neptune/Schreibtisch/Projects/mnist/train-labels-idx1-ubyte
# export REQUEST_INPUTS=/home/neptune/Schreibtisch/Projects/mnist/t10k-images-idx3-ubyte
# export REQUEST_LABELS=/home/neptune/Schreibtisch/Projects/mnist/t10k-labels-idx1-ubyte

# build protobuffer for go sdk
# pushd ../../src/sdk/go/ainari_sdk
# protoc --go_out=. --proto_path ../../../libs/protobuf ainari_messages.proto3
# popd

# build cli-binarygolangci-lint
pushd ../../src/cli/ainarictl
go build .
popd
cp ../../src/cli/ainarictl/ainarictl .

EXECUTABLE="./ainarictl --insecure"

# cleanup before running tests
$EXECUTABLE project delete cli_test_project
$EXECUTABLE user delete cli_test_user

# ########################
echo ""
echo "########################### project tests ##########################"
echo ""
$EXECUTABLE project create -n "cli test project" cli_test_project
$EXECUTABLE project get cli_test_project
$EXECUTABLE project list
$EXECUTABLE project delete cli_test_project

# ########################
echo ""
echo "########################### user tests ##########################"
echo ""
$EXECUTABLE user create -n "cli test user" -p "asdfasdf" cli_test_user
$EXECUTABLE user get cli_test_user
$EXECUTABLE user list
$EXECUTABLE user delete cli_test_user

# ########################
echo ""
echo "########################### dataset tests ##########################"
echo ""
DATASET_UUID=$($EXECUTABLE dataset create mnist -j -i $TRAIN_INPUTS -l $TRAIN_LABELS cli_test_dataset | jq -r '.uuid')
$EXECUTABLE dataset get $DATASET_UUID
$EXECUTABLE dataset list
$EXECUTABLE dataset delete $DATASET_UUID

# ########################
echo ""
echo "########################### cluster tests ##########################"
echo ""
CLUSTER_UUID=$($EXECUTABLE cluster create -j -t ./cluster_template cli_test_cluster | jq -r '.uuid')
$EXECUTABLE cluster get $CLUSTER_UUID
$EXECUTABLE cluster list
$EXECUTABLE cluster delete $CLUSTER_UUID

########################
echo ""
echo "########################### workfloat tests ##########################"
# $EXECUTABLE host list 

train_DATASET_UUID=$($EXECUTABLE dataset create mnist -j -i $TRAIN_INPUTS -l $TRAIN_LABELS cli_test_dataset_train | jq -r '.uuid')
echo "Train-Dataset-UUID: $train_DATASET_UUID"

request_DATASET_UUID=$($EXECUTABLE dataset create mnist -j -i $REQUEST_INPUTS -l $REQUEST_LABELS cli_test_dataset_req | jq -r '.uuid')
echo "Request-Dataset-UUID: $request_DATASET_UUID"

CLUSTER_UUID=$($EXECUTABLE cluster create -j -t ./cluster_template cli_test_cluster | jq -r '.uuid')
echo "Cluster-UUID: $CLUSTER_UUID"


# train test
echo "$EXECUTABLE task create train -j -i $train_DATASET_UUID:picture:picture -o $train_DATASET_UUID:label:label $CLUSTER_UUID cli_train_test_task"
task_uuid=$($EXECUTABLE task create train -j -i $train_DATASET_UUID:picture:picture -o $train_DATASET_UUID:label:label $CLUSTER_UUID cli_train_test_task | jq -r '.uuid')
echo "Train-Task-UUID: $task_uuid"

while true; do
    $EXECUTABLE task get $CLUSTER_UUID $task_uuid
    state=$($EXECUTABLE task get -j $CLUSTER_UUID $task_uuid | jq -r '.state')
    if [[ "$state" == *"Finished"* ]]; then
        echo "Process finished. Exiting loop."
        break
    fi
    sleep 1
done
$EXECUTABLE task get $CLUSTER_UUID $task_uuid


# save and restore test
# $EXECUTABLE task create checkpoint_create $CLUSTER_UUID cli_test_checkpoint
#checkpoint_uuid=$($EXECUTABLE task create checkpoint_create $CLUSTER_UUID cli_test_checkpoint  | jq -r '.uuid')
#sleep 2
#echo "Checkpoint-UUID: $checkpoint_uuid"
#$EXECUTABLE checkpoint list

#$EXECUTABLE cluster delete $CLUSTER_UUID
#CLUSTER_UUID=$($EXECUTABLE cluster create -j -t ./cluster_template cli_test_cluster | jq -r '.uuid')
#echo "new Cluster-UUID: $CLUSTER_UUID"
#$EXECUTABLE task create checkpoint_restore $CLUSTER_UUID restore_cluster
#sleep 2

# request test
echo "$EXECUTABLE task create request -j -i $request_DATASET_UUID:picture:picture -r label:cli_test_output $CLUSTER_UUID cli_request_test_task"
req_task_uuid=$($EXECUTABLE task create request -j -i $request_DATASET_UUID:picture:picture -r label:cli_test_output $CLUSTER_UUID cli_request_test_task | jq -r '.uuid')
echo "Request-Task-UUID: $req_task_uuid"

$EXECUTABLE task list $CLUSTER_UUID 
echo "$taskUuid"
$EXECUTABLE task delete $CLUSTER_UUID $task_uuid

while true; do
    $EXECUTABLE task get $CLUSTER_UUID $req_task_uuid
    state=$($EXECUTABLE task get -j $CLUSTER_UUID $req_task_uuid | jq -r '.state')
    if [[ "$state" == *"Finished"* ]]; then
        echo "Process finished. Exiting loop."
        break
    fi
    sleep 1
done
$EXECUTABLE task get $CLUSTER_UUID $req_task_uuid

$EXECUTABLE dataset list

# $EXECUTABLE dataset check -r $request_DATASET_UUID $req_task_uuid

# content=$($EXECUTABLE dataset content -j -c cli_test_output -o 100 -n 10 $result_uuid | jq -r 'length')
# if [[ "$content" != 10 ]]; then
#     echo "content as length of $content instead of 10"
# fi

# # clear all test-resources
# $EXECUTABLE checkpoint delete $checkpoint_uuid
$EXECUTABLE cluster delete $CLUSTER_UUID
$EXECUTABLE dataset delete $train_DATASET_UUID
$EXECUTABLE dataset delete $request_DATASET_UUID
$EXECUTABLE dataset delete $req_task_uuid
