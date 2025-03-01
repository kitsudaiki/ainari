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

import matplotlib
import matplotlib.pyplot as plt
from hanami_sdk import hanami_token
from hanami_sdk import cluster
from hanami_sdk import dataset
from hanami_sdk import task
from hanami_sdk import direct_io
import json
import time
import configparser
import urllib3
import asyncio


# the test use insecure connections, which is totally ok for the tests
# and neaded for testings endpoints with self-signed certificastes,
# but the warnings are anoying and have to be disabled by this line
urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

matplotlib.use('Qt5Agg')

config = configparser.ConfigParser()
config.read('/etc/openhanami/hanami_testing.conf')

address = config["connection"]["address"]
test_user_id = config["connection"]["test_user"]
test_user_pw = config["connection"]["test_passphrase"]

train_inputs = "./train.csv"
request_inputs = "./test.csv"

cluster_template = \
    "version: 1\n" \
    "settings:\n" \
    "   neuron_cooldown: 100000000.0\n" \
    "   refractory_time: 1\n" \
    "   max_connection_distance: 1\n" \
    "    \n" \
    "hexagons:\n" \
    "    1,1,1\n" \
    "    2,1,1\n" \
    "    3,1,1\n" \
    "    \n" \
    "inputs:\n" \
    "    test_input: 1,1,1\n" \
    "\n" \
    "outputs:\n" \
    "    test_output: 3,1,1\n" \

cluster_name = "test_cluster"
generic_task_name = "test_task"
template_name = "dynamic"
request_dataset_name = "request_test_dataset"
train_dataset_name = "train_test_dataset"

token = hanami_token.request_token(address, test_user_id, test_user_pw, False)

# initial cleanup for the case of leftovers from previous run
dataset.delete_all_datasets(token, address, False)
cluster.delete_all_cluster(token, address, False)

# update dataset
train_dataset_uuid = dataset.upload_csv_files(
    token, address, train_dataset_name, train_inputs, False)
request_dataset_uuid = dataset.upload_csv_files(
    token, address, request_dataset_name, request_inputs, False)

# define relations between data and cluster
train_inputs = [
    {
        "dataset_uuid": train_dataset_uuid,
        "dataset_column": "test_input",
        "hexagon_name": "test_input"
    }
]

train_outputs = [
    {
        "dataset_uuid": train_dataset_uuid,
        "dataset_column": "test_output",
        "hexagon_name": "test_output"
    }
]

request_inputs = [
    {
        "dataset_uuid": request_dataset_uuid,
        "dataset_column": "test_input",
        "hexagon_name": "test_input"
    }
]

request_results = [
    {
        "dataset_column": "test_output",
        "hexagon_name": "test_output"
    }
]

replicas = 10
cluster_uuids = [""] * 10
task_uuids = [""] * 10
result_outputs = [0.0] * 20
flattened_list = [0.0] * 1750

# create all cluster
for x in range(replicas):
    cluster_uuids[x] = cluster.create_cluster(
        token, address, cluster_name + str(x), cluster_template, False)["uuid"]

# train
for i in range(0, 500):
    for x in range(replicas):
        print("poi: ", i)
        task_uuids[x] = task.create_train_task(
            token, address, generic_task_name, cluster_uuids[x], train_inputs, train_outputs, 20, False)["uuid"]

    for x in range(replicas):
        finished = False
        result = task.get_task(token, address, task_uuids[x], cluster_uuids[x], False)
        finished = result["state"] == "finished"
        while not finished:
            result = task.get_task(token, address, task_uuids[x], cluster_uuids[x], False)
            finished = result["state"] == "finished"
            # print("wait for finish train-task")
            time.sleep(0.01)
        result = task.delete_task(token, address, task_uuids[x], cluster_uuids[x], False)

# test
for x in range(replicas):
    task_uuids[x] = task.create_request_task(
        token, address, generic_task_name, cluster_uuids[x], request_inputs, request_results, 20, False)["uuid"]
    finished = False
    while not finished:
        result = task.get_task(token, address, task_uuids[x], cluster_uuids[x], False)
        finished = result["state"] == "finished"
        print(result)
        # print("wait for finish request-task")
        time.sleep(0.1)
        # result = task.delete_task(token, address, task_uuids[x], cluster_uuids[x])

    data = dataset.download_dataset_content(
        token, address, task_uuids[x], "test_output", 1700, 0, False)["data"]

    # print(data)
    temp_list = [item for sublist in data for item in sublist]
    for r in range(len(temp_list)):
        flattened_list[r] += temp_list[r]

# delete everything again
for x in range(replicas):
    cluster.delete_cluster(token, address, cluster_uuids[x], False)
dataset.delete_dataset(token, address, train_dataset_uuid, False)
dataset.delete_dataset(token, address, request_dataset_uuid, False)

# update result
for r in range(len(flattened_list)):
    flattened_list[r] /= 10.0

# plot result
plt.rcParams["figure.figsize"] = [10, 5]
plt.rcParams["figure.autolayout"] = True
plt.plot(flattened_list, color="red")
plt.show()
