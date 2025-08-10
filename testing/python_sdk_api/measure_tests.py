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
from ainari_sdk import ainari_token
from ainari_sdk import cluster
from ainari_sdk import dataset
from ainari_sdk import task
from ainari_sdk import direct_io
import configparser
import urllib3
import time
import sys
import csv
import json


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



# the test use insecure connections, which is totally ok for the tests
# and neaded for testings endpoints with self-signed certificastes,
# but the warnings are anoying and have to be disabled by this line
urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

matplotlib.use('Qt5Agg')

config = configparser.ConfigParser()
config.read('/etc/ainari/hanami_testing.conf')

address = config["connection"]["address"]
test_user_id = config["connection"]["test_user"]
test_user_pw = config["connection"]["test_passphrase"]

train_inputs_file = "./train.csv"
request_inputs_file = "./test.csv"

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
    "    test_input: 1,1,1; " \
    "outputs: " \
    "    test_output: 3,2,2;"

cluster_name = "test_cluster"
generic_task_name = "test_task"
template_name = "dynamic"
request_dataset_name = "request_test_dataset"
train_dataset_name = "train_test_dataset"

token = ainari_token.request_token(address, test_user_id, test_user_pw, False)

# initial cleanup for the case of leftovers from previous run
dataset.delete_all_datasets(token, address, False)
cluster.delete_all_cluster(token, address, False)

# update dataset
train_dataset_uuid = dataset.upload_csv_files(
    token, address, train_dataset_name, train_inputs_file, False)["uuid"]
request_dataset_uuid = dataset.upload_csv_files(
    token, address, request_dataset_name, request_inputs_file, False)["uuid"]

# define relations between data and cluster
train_inputs = [
    {
        "dataset_uuid": train_dataset_uuid,
        "dataset_column": "test_input",
        "hexagon": "test_input"
    }
]

train_outputs = [
    {
        "dataset_uuid": train_dataset_uuid,
        "dataset_column": "test_output",
        "hexagon": "test_output"
    }
]

request_inputs = [
    {
        "dataset_uuid": train_dataset_uuid,
        "dataset_column": "test_input",
        "hexagon": "test_input"
    }
]

request_outputs = [
    {
        "hexagon": "test_output"
    }
]

replicas = 5
cluster_uuids = [""] * 10
task_uuids = [""] * 10
flattened_list = [0.0] * 1750

# create all cluster
for x in range(replicas):
    cluster_uuids[x] = cluster.create_cluster(
        token, address, cluster_name + str(x), cluster_template, False)["uuid"]

# train
for x in range(replicas):
    print("train replica: ", x)
    print('\n')
    task_uuids[x] = task.create_train_task(
        token, address, generic_task_name, cluster_uuids[x], train_inputs, train_outputs, 200, 20, False)["uuid"]
    finished = False
    while not finished:
        time.sleep(1)
        result = task.get_task(token, address, task_uuids[x], cluster_uuids[x], False)
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
    
# test
for x in range(replicas):
    print("test replica: ", x)
    with open(request_inputs_file, mode='r') as file:
        reader = csv.reader(file)
        
        # skip the header if there is one
        next(reader)
        output_names = ["test_output"]

        # read all rows into a list
        rows = list(reader)
        
        for index, _ in enumerate(rows[:-20]):
            values = list()
            for offset in range(0, 20):
                values.append(float(rows[index + offset][0]))
            inputs = dict()
            inputs["test_input"] = values

            output_values = cluster.request(token, address, cluster_uuids[x], inputs, output_names, False)
            out_val = json.dumps(output_values["outputs"]["test_output"][0], indent=4)
            flattened_list[index] += float(out_val)

# delete everything again
for x in range(replicas):
    cluster.delete_cluster(token, address, cluster_uuids[x], False)
dataset.delete_dataset(token, address, train_dataset_uuid, False)

# update result
for r in range(len(flattened_list)):
    flattened_list[r] /= 5.0

# plot result
plt.rcParams["figure.figsize"] = [10, 5]
plt.rcParams["figure.autolayout"] = True
plt.plot(flattened_list, color="red")
plt.show()
