# Workflow with websockets

Additionally to the REST-API there is a websocket connection available. This is used to upload files
and for the direct interaction with cluster in the backend.

As reference-implementation you can look into the python-version of the
[hanami_sdk](https://github.com/kitsudaiki/OpenHanami/tree/develop/src/sdk/python/hanami_sdk/hanami_sdk).

## File-upload of Datasets

![File-upload](Dataset_upload_workflow.drawio)

Steps:

1. send POST-request with json-body

    `{"name": "<NAME>", "input_data_size": <SIZE>, "label_data_size": <SIZE>}`

    to `<ADDRESS>/v1/mnist/dataset` in init an upload of a MNIST-dataset or

    `{"name": "<NAME>", "input_data_size": <SIZE>}`

    to `<ADDRESS/v1/csv/dataset` to upload a CSV-file. The `<SIZE>` is the size of the file in
    number of bytes to allocate the memory in the backend.

    The response is a json with the content:

    `{"uuid": "<DATASET_UUID>", "uuid_input_file": <INPUT_FILE_UUID>, "uuid_label_file": <LABEL_FILE_UUID>}`

    These are the UUIDs of the allocated resources in the backend to access them. In case of a
    CSV-file, there is no uuid_label_file.

2. create a websocket-connection to `ws://<ADDRESS>` and send as first message

    `{"token": "<TOKEN>", "uuid": "<FILE_UUID>", "target": "file_upload"}`

    to request a connection of the websocket to the requested temp-file in the backend. If the
    connection is accepted, it returns a json with success = 1. In case of an MNIST-dataset, where 2
    files have to be uploaded, there are 2 websockets necessary, one for each file.

3. The data-transfer over the websocket after this is done with
   [protobuf-messages](https://github.com/kitsudaiki/OpenHanami/blob/develop/src/libs/protobuf/hanami_messages.proto3)
   with the `FileUpload_Message`-message:

    - **position**: Byte-position within the temporary file
    - **data**: transfered bytes

    In the SDK the files are transfered in 128 KiB chunks.

    The response is the `FileUploadResponse_Message`-message.

4. After all is uploade, it can be checked, if the backend has registered all packages, with a
   GET-request against `<ADDRESS>/v1/dataset/progress?uuid=<UUID>` and returns a json like
   `{"complete": true }`, which says it is fully uploaded or not.

5. After everything was uploaded and the progress says, it is complete, a PUT-request to
   `<ADDRESS>/v1/mnist/dataset` or `<ADDRESS>/v1/csv/dataset` has to be send with a file like this:

    `{"uuid": "<DATASET_UUID>", "uuid_input_file": "<FILE_UUID>", "uuid_label_file": "<FILE_UUID>"}`

    This merge the temporary files within the backend into one file.

## Direct interaction with Cluster

![DirectIO](DirectIO_workflow.drawio)

Steps:

1. send PUT-request with json-body `{"uuid": "<CLUSTER_UUID>", "new_state": "DIRECT"}` to
   `<ADDRESS>/v1/cluster/set_mode` to swtich the cluster from the task-mode to the direct-mode

2. create a websocket-connection to `ws://<ADDRESS>` and send as first message
   `{"token": "<TOKEN>", "uuid": "<CLUSTER_UUID>", "target": "cluster"}` to request a connection of
   the websocket to the requested cluster. If the connection is accepted, it returns a json with
   success = 1.

3. The data-transfer over the websocket after this is done with
   [protobuf-messages](https://github.com/kitsudaiki/OpenHanami/blob/develop/src/libs/protobuf/hanami_messages.proto3)
   with the `ClusterIO_Message`-message:

    - **hexagonName**: name of the input-hexagon, where the data have to be applied
    - **isLast**: set to true, to say the backend, that this was the last message, so the backend
      starts to process the cluster
    - **processType**: `REQUEST_TYPE` or `TRAIN_TYPE` based of the type of the action
    - **numberOfValues**: number of input-values in the message
    - **values**: list of float-values to apply to the hexagon

    The response is the same type, which is coming from the backend in case of a request-task.

4. after all is done, switch back to task-mode.
