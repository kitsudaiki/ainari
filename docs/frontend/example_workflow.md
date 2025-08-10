# Example-Workflow

This chapter shows an example workflow for the current state of the project by using the CLI-client
and the MNIST-dataset, which is also used for automated testing within the project. Beside this
example-workflow it is also possible to use CSV-files instead of the MNIST-datasets, or interact
directly with the neural network via the python-version of the SDK. See for further information the
[CLI and SDK documentation](/frontend/cli_sdk_docu/)

## Preparation

-   install Backend based on the [Installation-guide](/backend/installation/)

-   Download and unzip the MNIST-dataset for testing

    ```bash
    wget https://storage.googleapis.com/cvdf-datasets/mnist/train-images-idx3-ubyte.gz
    wget https://storage.googleapis.com/cvdf-datasets/mnist/train-labels-idx1-ubyte.gz
    wget https://storage.googleapis.com/cvdf-datasets/mnist/t10k-images-idx3-ubyte.gz
    wget https://storage.googleapis.com/cvdf-datasets/mnist/t10k-labels-idx1-ubyte.gz
    ```

-   unzip files so you have the local files:

    ```bash
    train-images-idx3-ubyte
    train-labels-idx1-ubyte
    t10k-images-idx3-ubyte
    t10k-labels-idx1-ubyte
    ```

-   Download the pre-build binary of the CLI-client from the
    [file-share](https://files.ainari.cloud/)

    !!! info

          Because the rollout still use self-signed certificates, for the following `ainarictl`-commands the `--insecure`-flag has to be used to avoid tls-check. Will be changed in the near future.

-   Export env-variables for connect and login-information:

    ```bash
    export HANAMI_ADDRESS=<ADDRESS_OF_HANAMI_DEPLOYMENT>
    export HANAMI_USER=<USER_ID>
    export HANAMI_PASSPHRASE=<USER_PASSPHRASE>
    ```

    !!! example

          ```bash
          export HANAMI_ADDRESS=https://local-hanami-new
          export HANAMI_USER=asdf
          export HANAMI_PASSPHRASE=asdfasdf
          ```

## Example

-   Upload MNIST-dataset to backend

    ```bash
    # train-data
    ./ainarictl dataset create mnist --insecure -i ./train-images-idx3-ubyte -l ./train-labels-idx1-ubyte train_data

    # test-data
    ./ainarictl dataset create mnist --insecure -i ./t10k-images-idx3-ubyte -l ./t10k-labels-idx1-ubyte test_data
    ```

    !!! example

          ```bash
          ./ainarictl dataset create mnist --insecure -i ./train-images-idx3-ubyte -l ./train-labels-idx1-ubyte train_data
          +-------------------+-----------------------------------------------------------------------------------------------+
          | UUID              | ac404414-9bc3-4be6-85b6-1c76efad7c6e                                                          |
          | NAME              | train_data                                                                                    |
          | VERSION           | v1.alpha                                                                                      |
          | NUMBER OF COLUMNS | 794                                                                                           |
          | NUMBER OF ROWS    | 60000                                                                                         |
          | DESCRIPTION       | {"label":{"column_end":794,"column_start":784},"picture":{"column_end":784,"column_start":0}} |
          | VISIBILITY        | private                                                                                       |
          | OWNER ID          | asdf                                                                                          |
          | PROJECT ID        | admin                                                                                         |
          | CREATED AT        | 2024-08-11 13:16:54                                                                           |
          +-------------------+-----------------------------------------------------------------------------------------------+

          ./ainarictl dataset create mnist --insecure -i ./t10k-images-idx3-ubyte -l ./t10k-labels-idx1-ubyte test_data
          +-------------------+-----------------------------------------------------------------------------------------------+
          | UUID              | a42dce48-d4b6-40f9-a8d8-0c22d48e2a4d                                                          |
          | NAME              | test_data                                                                                     |
          | VERSION           | v1.alpha                                                                                      |
          | NUMBER OF COLUMNS | 794                                                                                           |
          | NUMBER OF ROWS    | 10000                                                                                         |
          | DESCRIPTION       | {"label":{"column_end":794,"column_start":784},"picture":{"column_end":784,"column_start":0}} |
          | VISIBILITY        | private                                                                                       |
          | OWNER ID          | asdf                                                                                          |
          | PROJECT ID        | admin                                                                                         |
          | CREATED AT        | 2024-08-11 13:14:46                                                                           |
          +-------------------+-----------------------------------------------------------------------------------------------+
          ```

-   Can be listed with

    ```bash
    ./ainarictl dataset list --insecure
    ```

    !!! example

          ```bash
          ./ainarictl dataset list --insecure
          +--------------------------------------+------------+------------+----------+------------+---------------------+
          |                 UUID                 |    NAME    | VISIBILITY | OWNER ID | PROJECT ID |     CREATED AT      |
          +--------------------------------------+------------+------------+----------+------------+---------------------+
          | a42dce48-d4b6-40f9-a8d8-0c22d48e2a4d | test_data  | private    | asdf     | admin      | 2024-08-11 13:14:46 |
          | 423ba6da-ca2a-4603-81bb-300326a10176 | train_data | private    | asdf     | admin      | 2024-08-11 13:16:54 |
          +--------------------------------------+------------+------------+----------+------------+---------------------+
          ```

-   Prepare [cluster-template](/frontend/cluster_templates/cluster_template/) by writing the
    following example into a local file names `cluster_template`

    ```
    version: 1
    settings:
        neuron_cooldown: 10000000.0
        refractory_time: 1
        max_connection_distance: 1

    hexagons:
        1,1,1
        2,1,1
        3,1,1

    inputs:
        picture_hexagon: 1,1,1

    outputs:
        label_hexagon: 3,1,1
    ```

    !!! info

          It is planned with version v0.6.0 to make these cluster-templates optional, so they are not required for such small examples anymore.

-   Create cluster from the template

    ```bash
    ./ainarictl cluster create --insecure -t ./cluster_template test_cluster
    ```

    !!! example

          ```
          ./ainarictl cluster create --insecure -t ./cluster_template test_cluster
          +------------+--------------------------------------+
          | UUID       | c495c0dd-2b4e-4a95-b533-fb9eb19c4ce4 |
          | NAME       | test_cluster                         |
          | VISIBILITY | private                              |
          | OWNER ID   | asdf                                 |
          | PROJECT ID | admin                                |
          | CREATED AT | 2024-08-11 13:19:29                  |
          +------------+--------------------------------------+
          ```

-   create task to train the cluster

    ```bash
    ./ainarictl task create train --insecure \
    -i <UUID_OF_THE_TRAIN_DATASET>:picture:picture_hexagon \
    -o <UUID_OF_THE_TRAIN_DATASET>:label:label_hexagon \
    -c <UUID_OF_THE_CLUSTER> \
    train_task
    ```

    !!! info

          The mapping of inputs and outputs is not optimal and will be updated in the near future.

    !!! example

          ```bash
          ./ainarictl task create train --insecure \
          -i 423ba6da-ca2a-4603-81bb-300326a10176:picture:picture_hexagon \
          -o 423ba6da-ca2a-4603-81bb-300326a10176:label:label_hexagon \
          -c c495c0dd-2b4e-4a95-b533-fb9eb19c4ce4 \
          train_task

          +------------------------+--------------------------------------+
          | UUID                   | ae28638c-6856-439e-af3b-71ae509363e9 |
          | STATE                  | queued                               |
          | CURRENT CYCLE          | 0                                    |
          | TOTAL NUMBER OF CYCLES | 60000                                |
          | QUEUE TIMESTAMP        | 2024-08-11 13:26:03                  |
          | START TIMESTAMP        | -                                    |
          | END TIMESTAMP          | -                                    |
          +------------------------+--------------------------------------+
          ```

    The task runs in the background by a task-queue. The status of the task can be checked with

    ```bash
    ./ainarictl task get --insecure -c <CLUSTER_UUID> <TASK_UUID>
    ```

    !!! example

          ```bash
          ./ainarictl task get --insecure -c  c495c0dd-2b4e-4a95-b533-fb9eb19c4ce4 ae28638c-6856-439e-af3b-71ae509363e9
          +------------------------+--------------------------------------+
          | UUID                   | ae28638c-6856-439e-af3b-71ae509363e9 |
          | STATE                  | finished                             |
          | CURRENT CYCLE          | 60000                                |
          | TOTAL NUMBER OF CYCLES | 60000                                |
          | QUEUE TIMESTAMP        | 2024-08-11 13:26:03                  |
          | START TIMESTAMP        | 2024-08-11 13:26:03                  |
          | END TIMESTAMP          | 2024-08-11 13:26:06                  |
          +------------------------+--------------------------------------+
          ```

    !!! info

        With the python-SDK it is also possible to interact directly with the cluster. See [direct-IO](/frontend/cli_sdk_docu/#direct-io)

-   create task to test the cluster

    ```bash
    ./ainarictl task create request --insecure \
    -i <UUID_OF_THE_TEST_DATASET>:picture:picture_hexagon \
    -r label_hexagon:test_output \
    -c <UUID_OF_THE_CLUSTER> \
    test_task
    ```

    It will create a new dataset with name `test_output` where the output of the task is written
    into

    !!! example

          ```bash
          ./ainarictl task create request --insecure \
          -i a42dce48-d4b6-40f9-a8d8-0c22d48e2a4d:picture:picture_hexagon \
          -r label_hexagon:test_output \
          -c c495c0dd-2b4e-4a95-b533-fb9eb19c4ce4 \
          test_task
          
          +------------------------+--------------------------------------+
          | UUID                   | 36f70489-c71c-48d7-9e08-a766507df5de |
          | STATE                  | queued                               |
          | CURRENT CYCLE          | 0                                    |
          | TOTAL NUMBER OF CYCLES | 10000                                |
          | QUEUE TIMESTAMP        | 2024-08-11 13:27:49                  |
          | START TIMESTAMP        | -                                    |
          | END TIMESTAMP          | -                                    |
          +------------------------+--------------------------------------+
          ```

-   check accuracy of the new resulting dataset

    ```bash
    ./ainarictl dataset check --insecure -r <UUID_OF_THE_TEST_DATASET> <UUID_OF_THE_NEW_CREATED_DATASET>
    ```

    !!! info

          This check only works for the MNIST-dataset and is primary for automated testing within the CI-pipeline.

    !!! example

          ```bash
          ./ainarictl dataset list --insecure
          +--------------------------------------+-------------+------------+----------+------------+---------------------+
          |                 UUID                 |    NAME     | VISIBILITY | OWNER ID | PROJECT ID |     CREATED AT      |
          +--------------------------------------+-------------+------------+----------+------------+---------------------+
          | a42dce48-d4b6-40f9-a8d8-0c22d48e2a4d | test_data   | private    | asdf     | admin      | 2024-08-11 13:14:46 |
          | 423ba6da-ca2a-4603-81bb-300326a10176 | train_data  | private    | asdf     | admin      | 2024-08-11 13:16:54 |
          | 36f70489-c71c-48d7-9e08-a766507df5de | test_output | private    | asdf     | admin      | 2024-08-11 13:27:49 |
          +--------------------------------------+-------------+------------+----------+------------+---------------------+

          ./ainarictl dataset check --insecure -r a42dce48-d4b6-40f9-a8d8-0c22d48e2a4d 36f70489-c71c-48d7-9e08-a766507df5de
          +----------+-------------------+
          | ACCURACY | 91.86000061035156 |
          +----------+-------------------+
          ```

          (Running the train-task multiple times still increased the accuracy up to 95-96%)

-   get the part of the resulting dataset

    ```bash
    ./ainarictl dataset content --insecure -c test_output -o 100 -n 10 <UUID_OF_THE_NEW_CREATED_DATASET>
    ```

    This prints the data of the lines 100 - 110 of the segment names `test_output` of the dataset

    !!! example

          ```bash
          ./ainarictl dataset content --insecure -c test_output -o 100 -n 10  36f70489-c71c-48d7-9e08-a766507df5de
          +-----+----------+----------+----------+----------+----------+----------+----------+----------+----------+----------+
          |     |    0     |    1     |    2     |    3     |    4     |    5     |    6     |    7     |    8     |    9     |
          +-----+----------+----------+----------+----------+----------+----------+----------+----------+----------+----------+
          | 100 | 0.002552 | 0.005864 | 0.003836 | 0.001205 | 0.009393 | 0.002166 | 0.755618 | 0.000546 | 0.000532 | 0.006673 |
          | 101 | 0.971624 | 0.001603 | 0.000198 | 0.005888 | 0.000695 | 0.027081 | 0.002200 | 0.003619 | 0.004066 | 0.000716 |
          | 102 | 0.000961 | 0.010222 | 0.000058 | 0.010438 | 0.001177 | 0.973779 | 0.000915 | 0.051081 | 0.006354 | 0.032422 |
          | 103 | 0.009570 | 0.001631 | 0.005261 | 0.003210 | 0.968258 | 0.000693 | 0.001188 | 0.006331 | 0.000562 | 0.029283 |
          | 104 | 0.001646 | 0.003349 | 0.001516 | 0.054619 | 0.005111 | 0.007289 | 0.000956 | 0.002295 | 0.002855 | 0.880102 |
          | 105 | 0.000907 | 0.005781 | 0.000871 | 0.002264 | 0.025513 | 0.002774 | 0.002865 | 0.005225 | 0.001418 | 0.966673 |
          | 106 | 0.007334 | 0.001637 | 0.881242 | 0.046091 | 0.004952 | 0.002028 | 0.008346 | 0.001143 | 0.009504 | 0.000137 |
          | 107 | 0.001559 | 0.902669 | 0.002997 | 0.029666 | 0.000555 | 0.004980 | 0.004056 | 0.000957 | 0.060024 | 0.000453 |
          | 108 | 0.000495 | 0.012944 | 0.000147 | 0.031673 | 0.052620 | 0.023361 | 0.001264 | 0.004183 | 0.007328 | 0.966943 |
          | 109 | 0.000874 | 0.001262 | 0.003177 | 0.001065 | 0.957378 | 0.001983 | 0.002980 | 0.017218 | 0.000286 | 0.004259 |
          +-----+----------+----------+----------+----------+----------+----------+----------+----------+----------+----------+
          ```
