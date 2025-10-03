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

from ainari_sdk import ainari_token
from ainari_sdk import checkpoint
from ainari_sdk import cluster
from ainari_sdk import dataset
# from ainari_sdk import direct_io
from ainari_sdk import hosts
from ainari_sdk import project
from ainari_sdk import task
from ainari_sdk import user
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
hanami_address = config["connection"]["hanami_address"]

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

    project.create_project(token, miko_address, projet_id, project_name, False)
    try:
        project.create_project(token, miko_address, projet_id, project_name, False)
    except ainari_exceptions.ConflictException:
        pass
    project.list_projects(token, miko_address, False)
    project.get_project(token, miko_address, projet_id, False)
    try:
        project.get_project(token, miko_address, "fail_project", False)
    except ainari_exceptions.NotFoundException:
        pass
    project.delete_project(token, miko_address, projet_id, False)
    try:
        project.delete_project(token, miko_address, projet_id, False)
    except ainari_exceptions.NotFoundException:
        pass


def test_user():
    print("test user")

    user.create_user(token, miko_address, user_id, user_name, passphrase, is_admin, False)
    try:
        user.create_user(token, miko_address, user_id, user_name, passphrase, is_admin, False)
    except ainari_exceptions.ConflictException:
        pass
    user.list_users(token, miko_address, False)
    user.get_user(token, miko_address, user_id, False)
    try:
        user.get_user(token, miko_address, "fail_user", False)
    except ainari_exceptions.NotFoundException:
        pass
    user.delete_user(token, miko_address, user_id, False)
    try:
        user.delete_user(token, miko_address, user_id, False)
    except ainari_exceptions.NotFoundException:
        pass


def test_dataset():
    print("test dataset")

    result = dataset.upload_mnist_files(
        token, hanami_address, train_dataset_name, train_inputs, train_labels, False)
    mnist_dataset_uuid = result["uuid"]

    dataset.list_datasets(token, hanami_address, False)
    mnist_dataset = dataset.get_dataset(token, hanami_address, mnist_dataset_uuid, False)
    assert mnist_dataset["number_of_rows"] == 60000
    assert mnist_dataset["number_of_columns"] == 2

    result = dataset.upload_csv_files(
        token, hanami_address, "csv_test", "./csv_test.csv", False)
    csv_dataset_uuid = result["uuid"]

    csv_dataset = dataset.get_dataset(token, hanami_address, csv_dataset_uuid, False)
    assert csv_dataset["number_of_rows"] == 3
    assert csv_dataset["number_of_columns"] == 3

    try:
        dataset.get_dataset(token, hanami_address, " 569003fd-bf24-410b-8678-28f141877ac9", False)
    except ainari_exceptions.NotFoundException:
        pass
    dataset.delete_dataset(token, hanami_address, mnist_dataset_uuid, False)
    dataset.delete_dataset(token, hanami_address, csv_dataset_uuid, False)
    try:
        dataset.delete_dataset(token, hanami_address, mnist_dataset_uuid, False)
    except ainari_exceptions.NotFoundException:
        pass


def test_cluster():
    print("test cluster")

    cluster_uuid = cluster.create_cluster(
        token, hanami_address, cluster_name, cluster_template, False)["uuid"]
    cluster.list_clusters(token, hanami_address, False)
    cluster.get_cluster(token, hanami_address, cluster_uuid, False)
    try:
        cluster.get_cluster(token, hanami_address, "569003fd-bf24-410b-8678-28f141877ac9", False)
    except ainari_exceptions.NotFoundException:
        pass
    cluster.delete_cluster(token, hanami_address, cluster_uuid, False)
    try:
        cluster.delete_cluster(token, hanami_address, cluster_uuid, False)
    except ainari_exceptions.NotFoundException:
        pass


async def test_direct_io(token, hanami_address, cluster_uuid):
    # check direct-mode
    ws = await cluster.switch_to_direct_mode(token, hanami_address, cluster_uuid, False)
    for i in range(0, 1):
        await direct_io.send_train_input(ws,
                                         "picture_hex",
                                         test_values.get_direct_io_test_intput(),
                                         True,
                                         False,
                                         False)
        await direct_io.send_train_input(ws,
                                         "label_hex",
                                         test_values.get_direct_io_test_output(),
                                         False,
                                         True,
                                         False)
    output_values = await direct_io.send_request_input(ws,
                                                       "picture_hex",
                                                       test_values.get_direct_io_test_intput(),
                                                       True,
                                                       False)
    # print(output_values)
    await ws.close()

    cluster.switch_to_task_mode(token, hanami_address, cluster_uuid, False)
    print(output_values)
    assert list(output_values).index(max(output_values)) == 5


def _creat_and_resore_checkpoint(cluster_uuid):
    # save and reload checkpoint
    checkpoint_uuid = task.create_checkpoint_save_task(
        token, hanami_address, cluster_uuid, checkpoint_name, False)["uuid"]
    time.sleep(2)
    result = checkpoint.list_checkpoints(token, hanami_address, False)
    # print(json.dumps(result, indent=4))

    cluster.delete_cluster(token, hanami_address, cluster_uuid, False)
    cluster_uuid = cluster.create_cluster(
        token, hanami_address, cluster_name, cluster_template, False)["uuid"]

    task.create_checkpoint_restore_task(
        token, hanami_address, cluster_uuid, "restore", checkpoint_uuid, False)
    time.sleep(2)
    checkpoint.delete_checkpoint(token, hanami_address, checkpoint_uuid, False)
    try:
        checkpoint.delete_checkpoint(token, hanami_address, checkpoint_uuid, False)
    except ainari_exceptions.NotFoundException:
        pass

    return cluster_uuid


def _train(cluster_uuid, train_dataset_uuid):
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
        token, hanami_address, generic_task_name, cluster_uuid, inputs, outputs, 1, 1, False)["uuid"]

    finished = False
    while not finished:
        time.sleep(1)
        result = task.get_task(token, hanami_address, task_uuid, cluster_uuid, False)
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

    result = task.get_task(token, hanami_address, task_uuid, cluster_uuid, False)
    # print(json.dumps(result, indent=4))

    print("\n")
    result = cluster.get_cluster(token, hanami_address, cluster_uuid, False)
    task.delete_task(token, hanami_address, task_uuid, cluster_uuid, False)


def _test(cluster_uuid, request_dataset_uuid):
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
        token, hanami_address, generic_task_name, cluster_uuid, inputs, results, 1, False)["uuid"]

    finished = False
    while not finished:
        time.sleep(1)
        result = task.get_task(token, hanami_address, task_uuid, cluster_uuid, False)
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
    result = task.list_tasks(token, hanami_address, cluster_uuid, False)
    # print(json.dumps(result, indent=4))
    task.delete_task(token, hanami_address, task_uuid, cluster_uuid, False)
    time.sleep(1)

    accuracy = dataset.check_dataset(
        token, hanami_address, task_uuid, "label_hex", request_dataset_uuid, "label", False)["accuracy"]
    print("=======================================")
    print("test-result: " + str(accuracy))
    print("=======================================")
    assert accuracy > 0.85

    # # download part of the resulting dataset
    # data = dataset.download_dataset_content(
    #     token, hanami_address, result_dataset_uuid, "test_output", 10, 100, False)["data"]
    # assert len(data[0]) == 10


def test_workflow():
    print("test workflow")

    # init
    cluster_uuid = cluster.create_cluster(
        token, hanami_address, cluster_name, cluster_template, False)["uuid"]
    train_dataset_uuid = ""
    request_dataset_uuid = ""
    try:
        train_dataset_uuid = dataset.upload_mnist_files(
            token, hanami_address, train_dataset_name, train_inputs, train_labels, False)["uuid"]
        time.sleep(1)
        request_dataset_uuid = dataset.upload_mnist_files(
            token, hanami_address, request_dataset_name, request_inputs, request_labels, False)["uuid"]
        time.sleep(1)
    except:
        # HINT (kitsudaiki): within the github-CI, the upload sometimes failes. Not sure why.
        #                    Maybe because of the limited resources. So it will be given a second
        #                    chance to make it right.
        train_dataset_uuid = dataset.upload_mnist_files(
            token, hanami_address, train_dataset_name, train_inputs, train_labels, False)["uuid"]
        time.sleep(1)
        request_dataset_uuid = dataset.upload_mnist_files(
            token, hanami_address, request_dataset_name, request_inputs, request_labels, False)["uuid"]
        time.sleep(1)

    # hosts_json = hosts.list_hosts(token, hanami_address, False)["body"]
    # if len(hosts_json) > 1:
    #     print("test move cluster to gpu")
    #     target_host_uuid = hosts_json[1][0]
    #     cluster.switch_host(token, hanami_address, cluster_uuid, target_host_uuid, False)

    _train(cluster_uuid, train_dataset_uuid)

    _test(cluster_uuid, request_dataset_uuid)
    _test(cluster_uuid, request_dataset_uuid)

    cluster_uuid = _creat_and_resore_checkpoint(cluster_uuid)

    _test(cluster_uuid, request_dataset_uuid)

    # asyncio.run(test_direct_io(token, hanami_address, cluster_uuid))

    inputs = dict()
    inputs["picture_hex"] = test_values.get_direct_io_test_intput()
    outputs = dict()
    outputs["label_hex"] = test_values.get_direct_io_test_output()

    for i in range(0, 100):
        cluster.train(token, hanami_address, cluster_uuid, inputs, outputs, False)

    output_names = ["label_hex"]
    output_values = cluster.request(token, hanami_address, cluster_uuid, inputs, output_names, False)
    print("output: %s" % json.dumps(output_values, indent=4))
    assert list(output_values["outputs"]["label_hex"]).index(
        max(output_values["outputs"]["label_hex"])) == 5

    # cleanup
    dataset.delete_dataset(token, hanami_address, train_dataset_uuid, False)
    dataset.delete_dataset(token, hanami_address, request_dataset_uuid, False)
    cluster.delete_cluster(token, hanami_address, cluster_uuid, False)


token = ainari_token.request_token(miko_address, test_user_id, test_user_pw, False)
print(token)
dataset.delete_all_datasets(token, hanami_address, False)
checkpoint.delete_all_checkpoints(token, hanami_address, False)
cluster.delete_all_cluster(token, hanami_address, False)
project.delete_all_projects(token, miko_address, False)
user.delete_all_user(token, miko_address, False)

version = common.get_version(token, hanami_address, False)
print(f"hanami-version: {version}")
version = common.get_version(token, miko_address, False)
print(f"miko-version: {version}")

test_project()
test_user()
test_dataset()
test_cluster()
test_workflow()
