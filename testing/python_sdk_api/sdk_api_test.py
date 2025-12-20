#!python3

# Copyright 2022 Tobias Anker
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

from ainari_sdk import login
from ainari_sdk import checkpoint
from ainari_sdk import cluster
from ainari_sdk import dataset
from ainari_sdk import host
from ainari_sdk import proxy
from ainari_sdk import project
from ainari_sdk import task
from ainari_sdk import secret
from ainari_sdk import user
from ainari_sdk import quota
from ainari_sdk import common
from ainari_sdk import ainari_exceptions
import test_values
import json
import time
import configparser
import urllib3
import asyncio
import sys


# the test use insecure connections, which is totally ok for the tests
# and neaded for testings endpoints with self-signed certificastes,
# but the warnings are anoying and have to be disabled by this line
urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

config = configparser.ConfigParser()
config.read('/etc/ainari/hanami_testing.conf')

miko_address = config["connection"]["miko_address"]

test_user_id = config["connection"]["test_user"]
test_user_pw = config["connection"]["test_passphrase"]

train_inputs = config["test_data"]["train_inputs"]
train_labels = config["test_data"]["train_labels"]
request_inputs = config["test_data"]["request_inputs"]
request_labels = config["test_data"]["request_labels"]

cluster_template = \
    "version: 1 " \
    "settings: " \
    "    neuron_cooldown: 1000000000.0; " \
    "    refractory_time: 1; " \
    "    max_connection_distance: 1; " \
    "hexagons:  " \
    "    1,1,1; " \
    "    2,2,2; " \
    "    3,2,2; " \
    "axons: " \
    "    1,1,1 -> 2,2,2;  " \
    "inputs: " \
    "    picture_hex: 1,1,1; " \
    "outputs: " \
    "    label_hex: 3,2,2;"

user_id = "tsugumi"
user_name = "Tsugumi"
passphrase = "asdfasdf"
is_admin = True
role = "tester"
projet_id = "test_project"
project_name = "Test Project"

cluster_name = "test_cluster"
checkpoint_name = "test_checkpoint"
generic_task_name = "test_task"
template_name = "dynamic"
request_dataset_name = "request_test_dataset"
train_dataset_name = "train_test_dataset"
secret_name = "test_secret"
secret_payload = "this is a dummy secret-payload for testing"


def progress_bar(epoch, total_epochs, cycle, total_cycles, prefix_epoch='', suffix_epoch='', prefix_cycle='', suffix_cycle='', length=50, fill='█'):
    percent1 = "{0:.1f}".format(100 * (epoch / float(total_epochs)))
    filled_length1 = int(length * epoch // total_epochs)
    bar1 = fill * filled_length1 + '-' * (length - filled_length1)

    percent2 = "{0:.1f}".format(100 * (cycle / float(total_cycles)))
    filled_length2 = int(length * cycle // total_cycles)
    bar2 = fill * filled_length2 + '-' * (length - filled_length2)

    sys.stdout.write('\033[F')  # move cursor up one line
    sys.stdout.write('\r%s |%s| %s%% %s\n' % (prefix_epoch, bar1, percent1, suffix_epoch))
    sys.stdout.write('\r%s |%s| %s%% %s' % (prefix_cycle, bar2, percent2, suffix_cycle))
    sys.stdout.flush()


def test_project():
    print("test project")

    project.create_project(context, projet_id, project_name)
    try:
        project.create_project(context, projet_id, project_name)
    except ainari_exceptions.ConflictException:
        pass
    project.list_projects(context)
    project.get_project(context, projet_id)
    try:
        project.get_project(context, "fail_project")
    except ainari_exceptions.NotFoundException:
        pass
    project.delete_project(context, projet_id)
    try:
        project.delete_project(context, projet_id)
    except ainari_exceptions.NotFoundException:
        pass


def test_user():
    print("test user")

    user.create_user(context, user_id, user_name, passphrase, is_admin)
    try:
        user.create_user(context, user_id, user_name, passphrase, is_admin)
    except ainari_exceptions.ConflictException:
        pass
    user.list_users(context)
    user.get_user(context, user_id)
    try:
        user.get_user(context, "fail_user")
    except ainari_exceptions.NotFoundException:
        pass
    user.delete_user(context, user_id)
    try:
        user.delete_user(context, user_id)
    except ainari_exceptions.NotFoundException:
        pass


def test_dataset():
    print("test dataset")

    result = dataset.upload_mnist_files(
        context, train_dataset_name, train_inputs, train_labels)
    mnist_dataset_uuid = result["uuid"]

    dataset.list_datasets(context)
    mnist_dataset = dataset.get_dataset(context, mnist_dataset_uuid)
    assert mnist_dataset["number_of_rows"] == 60000
    assert len(mnist_dataset["column_names"]) == 2

    result = dataset.upload_csv_files(
        context, "csv_test", "./csv_test.csv")
    csv_dataset_uuid = result["uuid"]

    csv_dataset = dataset.get_dataset(context, csv_dataset_uuid)
    assert csv_dataset["number_of_rows"] == 3
    assert len(csv_dataset["column_names"]) == 3

    try:
        dataset.get_dataset(context, " 569003fd-bf24-410b-8678-28f141877ac9")
    except ainari_exceptions.NotFoundException:
        pass
    dataset.delete_dataset(context, mnist_dataset_uuid)
    dataset.delete_dataset(context, csv_dataset_uuid)
    try:
        dataset.delete_dataset(context, mnist_dataset_uuid)
    except ainari_exceptions.NotFoundException:
        pass


def test_cluster():
    print("test cluster")

    cluster_uuid = cluster.create_cluster(
        context, cluster_name, cluster_template)["uuid"]
    cluster.list_clusters(context)
    cluster.get_cluster(context, cluster_uuid)
    try:
        cluster.get_cluster(context, "569003fd-bf24-410b-8678-28f141877ac9")
    except ainari_exceptions.NotFoundException:
        pass
    cluster.delete_cluster(context, cluster_uuid)
    try:
        cluster.delete_cluster(context, cluster_uuid)
    except ainari_exceptions.NotFoundException:
        pass


def test_secret():
    print("test secret")

    secret_uuid = secret.create_secret(
        context, secret_name, secret_payload)["uuid"]
    secret.list_secrets(context)
    secret.get_secret(context, secret_uuid)
    try:
        secret.get_secret(context, "569003fd-bf24-410b-8678-28f141877ac9")
    except ainari_exceptions.NotFoundException:
        pass
    req_secret_payload = secret.get_secret_payload(context, secret_uuid)["secret_payload"]
    assert req_secret_payload == secret_payload

    secret.delete_secret(context, secret_uuid)
    try:
        secret.delete_secret(context, secret_uuid)
    except ainari_exceptions.NotFoundException:
        pass


def test_quota():
    print("test quota")

    user.create_user(context, user_id, user_name, passphrase, is_admin)

    quota.list_quotas(context)
    user_quota = quota.get_quota(context, user_id)
    assert user_quota["max_cluster"] == 10
    assert user_quota["max_dataset"] == 10
    assert user_quota["max_checkpoint"] == 10
    assert user_quota["max_secret"] == 10
    assert user_quota["max_taskqueue"] == 10

    try:
        quota.get_quota(context, "fail-user")
    except ainari_exceptions.NotFoundException:
        pass

    # set and check quota
    user_quota = quota.set_quota(context, user_id, 11, 12, 13, 14, 0)
    user_quota = quota.get_quota(context, user_id)
    assert user_quota["max_cluster"] == 11
    assert user_quota["max_dataset"] == 12
    assert user_quota["max_checkpoint"] == 13
    assert user_quota["max_secret"] == 14
    assert user_quota["max_taskqueue"] == 10

    user.delete_user(context, user_id)


def _creat_and_resore_checkpoint(cluster_uuid, torii_port):
    # save and reload checkpoint
    checkpoint_uuid = task.create_checkpoint_save_task(
        context, torii_port, cluster_uuid, checkpoint_name)["uuid"]
    time.sleep(2)
    result = checkpoint.list_checkpoints(context)
    # print(json.dumps(result, indent=4))

    cluster.delete_cluster(context, cluster_uuid)
    time.sleep(2)
    new_cluster = cluster.create_cluster(
        context, cluster_name, cluster_template)
    cluster_uuid = new_cluster["uuid"]
    torii_port = new_cluster["torii_port"]

    task.create_checkpoint_restore_task(
        context, torii_port, cluster_uuid, "restore", checkpoint_uuid)
    time.sleep(2)
    checkpoint.delete_checkpoint(context, checkpoint_uuid)
    try:
        checkpoint.delete_checkpoint(context, checkpoint_uuid)
    except ainari_exceptions.NotFoundException:
        pass

    return cluster_uuid, torii_port


def _train(cluster_uuid, torii_port, train_dataset_uuid):
    inputs = [
        {
            "dataset_uuid": train_dataset_uuid,
            "dataset_column": "picture",
            "hexagon": "picture_hex"
        }
    ]

    outputs = [
        {
            "dataset_uuid": train_dataset_uuid,
            "dataset_column": "label",
            "hexagon": "label_hex"
        }
    ]

    task_uuid = task.create_train_task(
        context, torii_port, generic_task_name, cluster_uuid, inputs, outputs, 1, 1)["uuid"]

    finished = False
    while not finished:
        time.sleep(1)
        result = task.get_task(context, torii_port, task_uuid, cluster_uuid)
        # print(json.dumps(result, indent=4))

        finished = result["state"] == "Finished" or result["state"] == "Error"
        progress_bar(result["current_epoch"],
                     result["total_number_of_epochs"],
                     result["current_cycle"],
                     result["total_number_of_cycles"],
                     prefix_epoch='Epoch:',
                     suffix_epoch='Complete',
                     prefix_cycle='Cycle:',
                     suffix_cycle='Complete',
                     length=50)

    result = task.get_task(context, torii_port, task_uuid, cluster_uuid)
    # print(json.dumps(result, indent=4))

    print("\n")
    result = cluster.get_cluster(context, cluster_uuid)
    task.delete_task(context, torii_port, task_uuid, cluster_uuid)


def _test(cluster_uuid, torii_port, request_dataset_uuid):
    # run testing
    inputs = [
        {
            "dataset_uuid": request_dataset_uuid,
            "dataset_column": "picture",
            "hexagon": "picture_hex"
        }
    ]

    results = [
        {
            "hexagon": "label_hex"
        }
    ]

    task_uuid = task.create_request_task(
        context, torii_port, generic_task_name, cluster_uuid, inputs, results, 1)["uuid"]

    finished = False
    while not finished:
        time.sleep(1)
        result = task.get_task(context, torii_port, task_uuid, cluster_uuid)
        # print(json.dumps(result, indent=4))

        finished = result["state"] == "Finished" or result["state"] == "Error"
        progress_bar(result["current_epoch"],
                     result["total_number_of_epochs"],
                     result["current_cycle"],
                     result["total_number_of_cycles"],
                     prefix_epoch='Epoch:',
                     suffix_epoch='Complete',
                     prefix_cycle='Cycle:',
                     suffix_cycle='Complete',
                     length=50)

    print("\n")
    result = task.list_tasks(context, torii_port, cluster_uuid)
    # print(json.dumps(result, indent=4))
    task.delete_task(context, torii_port, task_uuid, cluster_uuid)
    time.sleep(1)

    accuracy = dataset.check_dataset(
        context, task_uuid, "label_hex", request_dataset_uuid, "label")["accuracy"]
    print("=======================================")
    print("test-result: " + str(accuracy))
    print("=======================================")
    assert accuracy > 0.85

    # # download part of the resulting dataset
    # data = dataset.download_dataset_content(
    #     context, result_dataset_uuid, "test_output", 10, 100)["data"]
    # assert len(data[0]) == 10


def test_workflow():
    print("test workflow")

    # init
    cluster_resp = cluster.create_cluster(context, cluster_name, cluster_template)
    cluster_uuid = cluster_resp["uuid"]
    torii_port = cluster_resp["torii_port"]

    train_dataset_uuid = ""
    request_dataset_uuid = ""
    try:
        train_dataset_uuid = dataset.upload_mnist_files(
            context, train_dataset_name, train_inputs, train_labels)["uuid"]
        time.sleep(1)
        request_dataset_uuid = dataset.upload_mnist_files(
            context, request_dataset_name, request_inputs, request_labels)["uuid"]
        time.sleep(1)
    except:
        # HINT (kitsudaiki): within the github-CI, the upload sometimes failes. Not sure why.
        #                    Maybe because of the limited resources. So it will be given a second
        #                    chance to make it right.
        train_dataset_uuid = dataset.upload_mnist_files(
            context, train_dataset_name, train_inputs, train_labels)["uuid"]
        time.sleep(1)
        request_dataset_uuid = dataset.upload_mnist_files(
            context, request_dataset_name, request_inputs, request_labels)["uuid"]
        time.sleep(1)

    # hosts_json = hosts.list_hosts(context)["body"]
    # if len(hosts_json) > 1:
    #     print("test move cluster to gpu")
    #     target_host_uuid = hosts_json[1][0]
    #     cluster.switch_host(context, cluster_uuid, target_host_uuid)

    _train(cluster_uuid, torii_port, train_dataset_uuid)

    _test(cluster_uuid, torii_port, request_dataset_uuid)
    _test(cluster_uuid, torii_port, request_dataset_uuid)

    cluster_uuid, torii_port = _creat_and_resore_checkpoint(cluster_uuid, torii_port)

    _test(cluster_uuid, torii_port, request_dataset_uuid)

    inputs = dict()
    inputs["picture_hex"] = test_values.get_direct_io_test_intput()
    outputs = dict()
    outputs["label_hex"] = test_values.get_direct_io_test_output()

    for i in range(0, 100):
        cluster.train(context, torii_port, cluster_uuid, inputs, outputs)

    output_names = ["label_hex"]
    output_values = cluster.request(context, torii_port, cluster_uuid, inputs, output_names)
    print("output: %s" % json.dumps(output_values, indent=4))
    assert list(output_values["outputs"]["label_hex"]).index(
        max(output_values["outputs"]["label_hex"])) == 5

    # cleanup
    dataset.delete_dataset(context, train_dataset_uuid)
    dataset.delete_dataset(context, request_dataset_uuid)
    cluster.delete_cluster(context, cluster_uuid)


context = login.request_context(miko_address, test_user_id, test_user_pw, False)
context.verify_connection = False
print(context)
dataset.delete_all_datasets(context)
checkpoint.delete_all_checkpoints(context)
cluster.delete_all_cluster(context)
project.delete_all_projects(context)
user.delete_all_user(context)

version = common.get_version(context, context.hanami_address)
print(f"hanami-version: {version}")
version = common.get_version(context, context.miko_address)
print(f"miko-version: {version}")
version = common.get_version(context, context.ryokan_adress)
print(f"ryokan-version: {version}")
version = common.get_version(context, context.torii_address)
print(f"torii-version: {version}")

test_project()
test_user()
test_dataset()
test_cluster()
test_secret()
test_quota()
test_workflow()
