#!python3

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

def delete_all_cluster():
    result = cluster.list_clusters(token, address, False)
    body = json.loads(result)["body"]

    for entry in body:
        cluster.delete_cluster(token, address, entry[1], False)


def delete_all_datasets():
    result = dataset.list_datasets(token, address, False)
    body = json.loads(result)["body"]

    for entry in body:
        dataset.delete_dataset(token, address, entry[1], False)


async def test_direct_io(token, address, cluster_uuid):
    # check direct-mode
    ws = await cluster.switch_to_direct_mode(token, address, cluster_uuid, False)

    input_block = [0.0] * 20
    output_block = [0] * 20

    print("init")
    for step in range (100):
        for x in range(len(input_block)):
            input_block[x] += 1.0
        await direct_io.send_train_input(ws, "test_input",  input_block,  True,  False, False)
        await direct_io.send_train_input(ws, "test_output", output_block, False, True,  False)

    await ws.close()
    cluster.switch_to_task_mode(token, address, cluster_uuid, False)


config = configparser.ConfigParser()
config.read('/etc/openhanami/hanami_testing.conf')

address = config["connection"]["address"]
test_user_id = config["connection"]["test_user"]
test_user_pw = config["connection"]["test_passphrase"]

train_inputs = "/home/neptune/Schreibtisch/Projekte/OpenHanami/testing/python_sdk_api/train.csv"
request_inputs = "/home/neptune/Schreibtisch/Projekte/OpenHanami/testing/python_sdk_api/test.csv"

cluster_template = \
    "version: 1\n" \
    "settings:\n" \
    "   neuron_cooldown: 100000000.0\n" \
    "   refractory_time: 1\n" \
    "   max_connection_distance: 1\n" \
    "   enable_reduction: false\n" \
    "    \n" \
    "hexagons:\n" \
    "    1,1,1\n" \
    "    2,1,1\n" \
    "    \n" \
    "inputs:\n" \
    "    test_input: 1,1,1\n" \
    "\n" \
    "outputs:\n" \
    "    test_output: 2,1,1\n" \

cluster_name = "test_cluster"
generic_task_name = "test_task"
template_name = "dynamic"
request_dataset_name = "request_test_dataset"
train_dataset_name = "train_test_dataset"

token = hanami_token.request_token(address, test_user_id, test_user_pw, False)

result_outputs = [0.0] * 20
flattened_list = [0.0] * 1750

delete_all_datasets()
delete_all_cluster()

result = cluster.create_cluster(token, address, cluster_name, cluster_template, False)
cluster_uuid = json.loads(result)["uuid"]
asyncio.run(test_direct_io(token, address, cluster_uuid))

train_dataset_uuid = dataset.upload_csv_files(token, address, train_dataset_name, train_inputs, False)
request_dataset_uuid = dataset.upload_csv_files(
    token, address, request_dataset_name, request_inputs, False)

for x in range(20):
    inputs = [
        {
            "dataset_uuid": train_dataset_uuid,
            "dataset_column": "test_input",
            "hexagon_name": "test_input"
        }
    ]

    outputs = [
        {
            "dataset_uuid": train_dataset_uuid,
            "dataset_column": "test_output",
            "hexagon_name": "test_output"
        }
    ]


    # train
    for i in range(0, 100):
        print("poi: ", i)
        result = task.create_train_task(token, address, generic_task_name, cluster_uuid, inputs, outputs, 20, False)
        task_uuid = json.loads(result)["uuid"]
        finished = False
        while not finished:
            result = task.get_task(token, address, task_uuid, cluster_uuid, False)
            finished = json.loads(result)["state"] == "finished"
            # print("wait for finish train-task")
            time.sleep(0.01)
        result = task.delete_task(token, address, task_uuid, cluster_uuid, False)


    inputs = [
        {
            "dataset_uuid": request_dataset_uuid,
            "dataset_column": "test_input",
            "hexagon_name": "test_input"
        }
    ]

    results = [
        {
            "dataset_column": "test_output",
            "hexagon_name": "test_output"
        }
    ]

    # test
    result = task.create_request_task(token, address, generic_task_name, cluster_uuid, inputs, results, 20, False)
    task_uuid = json.loads(result)["uuid"]

    finished = False
    while not finished:
        result = task.get_task(token, address, task_uuid, cluster_uuid, False)
        finished = json.loads(result)["state"] == "finished"
        print(result)
        # print("wait for finish request-task")
        time.sleep(0.1)
        # result = task.delete_task(token, address, task_uuid, cluster_uuid)

result = dataset.download_dataset_content(
        token, address, task_uuid, "test_output", 1700, 0, False)

data = json.loads(result)["data"]
#print(data)
temp_list = [item for sublist in data for item in sublist]
for r in range(len(temp_list)):
    flattened_list[r] += temp_list[r]

# result = cluster.get_cluster(token, address, cluster_uuid, False)
# print(json.dumps(json.loads(result), indent=4))

# Output the flattened list
# print(flattened_list)
# print(json.dumps(json.loads(result), indent=4))

dataset.delete_dataset(token, address, train_dataset_uuid, False)
dataset.delete_dataset(token, address, request_dataset_uuid, False)
cluster.delete_cluster(token, address, cluster_uuid, False)

for r in range(len(flattened_list)):
    flattened_list[r] /= 20.0

# Open the file in write mode
with open("out.txt", 'w') as file:
    # Write each value from the array to a new line
    for value in flattened_list:
        file.write(str(value) + '\n')


plt.rcParams["figure.figsize"] = [10, 5]
plt.rcParams["figure.autolayout"] = True

plt.plot(flattened_list, color="red")

plt.show()
