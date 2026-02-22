# CLI-SDK-Docu

The CLI and the SDK-library provides functions to interact with the API of the backend.

!!! Warning

    This documentation is **NOT** up-to-date at the moment. I'm very sorry, but because of a very
    limited amount of time I have for this project and because there coming some big changes in 0.10.0
    again and because it seems that no one use the project at the moment anyway, I haven't updated the
    documenation of the SDK and CLI here for version 0.8.0. For the CLI you can use the --help output.

## Installation / Compile

=== "CLI"

    ```bash
    # go into the cli-source-directory
    cd src/cli/ainarictl/

    # build protobuf-messages
    # pushd ../sdk/go/ainari_sdk
    # protoc --go_out=. --proto_path ../../../libs/protobuf ainari_messages.proto3
    # popd

    # build cli-tool
    go build .
    ```

=== "Python-SDK"

    ```bash
    # clone repository
    git clone https://github.com/kitsudaiki/ainari.git

    # create python-env (optional)
    python3 -m venv ainari_sdk_env
    source ainari_sdk_env/bin/activate

    # install sdk
    cd ainari/src/sdk/python/ainari_sdk
    pip3 install -U .
    ```

## Exceptions

Each of the used HTTP-error codes results in a different exception. For the available error-code /
exceptions of each of the endpoints, look into the
[REST-API documenation](https://docs.ainari.cloud/api/rest_api_documentation/)

=== "Python-SDK"

    ```python
    from ainari_sdk import ainari_exceptions

    try:
        (command)
    except ainari_exceptions.NotFoundException as e:
        print(e)
    except ainari_exceptions.UnauthorizedException as e:
        print(e)
    except ainari_exceptions.BadRequestException as e:
        print(e)
    except ainari_exceptions.ConflictException as e:
        print(e)
    except ainari_exceptions.InternalServerErrorException as e:
        print("internal error")
    ```

    !!! info

        The `InternalServerErrorException` doesn't contain a message. If this exception appears, you have
        have to look into the logs on the server.

## For insecure connections

In case the server use self-signed certificates for its https-connection, the ssl verification can
be disabled. Each functions has a paramater `verify_connection`, wich is per default `True`. This
validation can be disabled by adding `,verify_connection=False` to the end of a function-call.

## Request Token

For each of the following actions, the user must request an access-token at the beginning. This
token is a jwt-token with basic information of the user. The token is only valid for a certain
amount of time until it expires, based on the configuration of the server.

=== "CLI"

    In case of the cli, the address and login credentials only have to be set via environment variables

    ```bash
    export AINARI_ADDRESS=http://127.0.0.1:11417
    export AINARI_USER=asdf
    export AINARI_PASSPHRASE=asdfasdf
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import ainari_token

    address = "http://127.0.0.1:11417"
    test_user = "asdf"
    test_passphrase = "asdfasdf"

    context = login.request_context(address, test_user, test_passphrase)

    ```

## Project

Non-admin user need to be assigned to a project for logical separation.

!!! info

    These endpoints have a hard-coded requirement, that only admins are allowed to manage projects.

### Create Project

Create new empty project.

=== "CLI"

    ```bash
    ainarictl project create -n <NAME> <PROJECT_ID>
    ```

    example:

    ```bash
    ainarictl project create -n "cli test project" cli_test_project

    +------------+---------------------+
    | ID         | cli_test_project    |
    | NAME       | cli test project    |
    | CREATOR ID | asdf                |
    | CREATED AT | 2024-07-12 20:52:21 |
    +------------+---------------------+
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import project

    address = "http://127.0.0.1:11417"
    project_id = "test_project"
    project_name = "Test Project"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = project.create_project(context, projet_id, project_name)

    # example-content of result:
    #
    # {
    #     "creator_id": "asdf",
    #     "id": "test_project",
    #     "name": "Test Project"
    # }
    ```

### Get Project

Get information about a project.

=== "CLI"

    ```bash
    ainarictl project get <PROJECT_ID>
    ```

    example:

    ```bash
    ainarictl project get cli_test_project

    +------------+---------------------+
    | ID         | cli_test_project    |
    | NAME       | cli test project    |
    | CREATOR ID | asdf                |
    | CREATED AT | 2024-07-12 20:52:21 |
    +------------+---------------------+
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import project

    address = "http://127.0.0.1:11417"
    project_id = "test_project"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = project.get_project(context, projet_id)

    # example-content of result:
    #
    # {
    #     "creator_id": "asdf",
    #     "id": "test_project",
    #     "name": "Test Project"
    # }
    ```

### List Project

List all projects.

=== "CLI"

    ```bash
    ainarictl project list
    ```

    example:

    ```bash
    ainarictl project list

    +------------------+------------------+------------+---------------------+
    |        ID        |       NAME       | CREATOR ID |     CREATED AT      |
    +------------------+------------------+------------+---------------------+
    | cli_test_project | cli test project | asdf       | 2024-07-12 20:52:21 |
    +------------------+------------------+------------+---------------------+
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import project

    address = "http://127.0.0.1:11417"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = project.list_projects(token, address)

    # example-content of result:
    #
    # {
    #     "body": [
    #         [
    #             "test_project",
    #             "Test Project",
    #             "asdf"
    #         ]
    #     ],
    #     "header": [
    #         "id",
    #         "name",
    #         "creator_id"
    #     ]
    # }
    ```

### Delete Project

Delete a project.

!!! warning

    At the moment there is no check, if there still exist resources within this project.

=== "CLI"

    ```bash
    ainarictl project delete <PROJECT_ID>
    ```

    example:

    ```bash
    ainarictl project delete cli_test_project

    successfully deleted project 'cli_test_project'
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import project

    address = "http://127.0.0.1:11417"
    project_id = "test_project"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    project.delete_project(context, projet_id)
    ```

### Delete all projects

Delete all projects.

=== "CLI"

    (not implemented yet)

=== "Python-SDK"

    ```python
    from ainari_sdk import project

    address = "http://127.0.0.1:11417"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    project.delete_all_projects(token, address)
    ```

## User

!!! info

    These endpoints have a hard-coded requirement, that only admins are allowed to manage user.

### Create User

Create a new user.

If the `is_admin` is set to true, the user becomes a global admin.

=== "CLI"

    ```bash
    ./ainarictl user create -n <NAME> <USER_ID>

    (the cli will request the passphrase for the new user after enter this command)
    ```

    example:

    ```bash
    ./ainarictl user create -n "cli test user" -p "asdfasdfasdf" cli_test_user
    Enter Passphrase:
    Enter Passphrase again:

    +------------+---------------------+
    | ID         | cli_test_user       |
    | NAME       | cli test user       |
    | IS ADMIN   | false               |
    | PROJECTS   | []                  |
    | CREATOR ID | asdf                |
    | CREATED AT | 2024-07-12 20:52:21 |
    +------------+---------------------+
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import user

    address = "http://127.0.0.1:11417"
    new_user = "new_user"
    new_id = "new_user"
    new_pw = "asdfasdf"
    is_admin = True

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = user.create_user(context, new_id, new_user, new_pw, is_admin)

    # example-content of result:
    #
    # {
    #     "creator_id": "asdf",
    #     "id": "new_user",
    #     "is_admin": true,
    #     "name": "new_user",
    #     "projects": []
    # }
    ```

### Get User

Get information about a specific user.

=== "CLI"

    ```bash
    ainarictl user get <USER_ID>
    ```

    example:

    ```bash
    ainarictl user get cli_test_user

    +------------+---------------------+
    | ID         | cli_test_user       |
    | NAME       | cli test user       |
    | IS ADMIN   | false               |
    | PROJECTS   | []                  |
    | CREATOR ID | asdf                |
    | CREATED AT | 2024-07-12 20:52:21 |
    +------------+---------------------+
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import user

    address = "http://127.0.0.1:11417"
    user_id = "new_user"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = user.get_user(context, user_id)

    # example-content of result:
    #
    # {
    #     "creator_id": "asdf",
    #     "id": "tsugumi",
    #     "is_admin": true,
    #     "name": "Tsugumi",
    #     "projects": []
    # }
    ```

### List User

List all user.

=== "CLI"

    ```bash
    ainarictl user list
    ```

    example:

    ```bash
    ainarictl user list

    |      ID       |     NAME      | IS ADMIN | PROJECTS | CREATOR ID  |     CREATED AT      |
    +---------------+---------------+----------+----------+-------------+---------------------+
    | asdf          | asdf          | true     | []       | AINARI_INIT | 2024-06-26 16:57:35 |
    | cli_test_user | cli test user | false    | []       | asdf        | 2024-07-12 20:52:21 |
    +---------------+---------------+----------+----------+-------------+---------------------+
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import user

    address = "http://127.0.0.1:11417"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = user.list_users(token, address)

    # example-content of result:
    #
    # {
    #     "header": [
    #         "id",
    #         "name",
    #         "creator_id",
    #         "projects",
    #         "is_admin"
    #     ],
    #     "body": [
    #         [
    #             "asdf",
    #             "asdf",
    #             "MISAKI",
    #             [],
    #             true
    #         ],
    #         [
    #             "new_user",
    #             "new_user",
    #             "asdf",
    #             [],
    #             true
    #         ]
    #     ]
    # }
    ```

### Delete User

Delete a user from the backend.

!!! info

    A user can not be deleted by himself.

=== "CLI"

    ```bash
    ainarictl user delete cli_test_user
    ```

    example:

    ```bash
    ainarictl user delete cli_test_user

    successfully deleted project 'cli_test_project'
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import user

    address = "http://127.0.0.1:11417"
    user_id = "new_user"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    user.delete_user(context, user_id)
    ```

### Delete all users

Delete all users, except the one, who executed this action.

=== "CLI"

    (not implemented yet)

=== "Python-SDK"

    ```python
    from ainari_sdk import user

    address = "http://127.0.0.1:11417"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    user.delete_all_user(token, address)
    ```

### Add project to user

Assigne a project to a normal user.

The `role` is uses be the policy-file of the Ainari-instance restrict access to specific
API-endpoints. Per default there exist `admin` and `member` as roles.

If `is_project_admin` is set to true, the user can access all resources of all users within the
project.

=== "CLI"

    ```bash
    ```

    example:

    ```bash
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import user

    address = "http://127.0.0.1:11417"
    user_id = "new_user"
    project_id = "test_project"
    role = "member"
    is_project_admin = True

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = add_roject_to_user(token,
                                address,
                                user_id,
                                project_id,
                                role,
                                is_project_admin)
    ```

### Remove project from user

Unassign a project from a user.

=== "CLI"

    ```bash

    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import user

    address = "http://127.0.0.1:11417"
    user_id = "new_user"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    remove_project_fromUser(context, user_id, project_id)
    ```

### List projects of current user

List projects only of the current user, which are enabled by the current token.

=== "CLI"

    ```bash

    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import user

    address = "http://127.0.0.1:11417"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = list_projects_of_user(token, address)
    ```

### Switch project-scrope of current user

Switch to another project with the current user.

=== "CLI"

    ```bash

    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import user

    address = "http://127.0.0.1:11417"
    project_id = "test_project"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = switch_project(context, project_id)
    ```

## Dataset

Datasets are a bunch of train- or test-data, which can be uploaded to the server.

### Upload MNIST-Dataset

These are files of the official mnist-dataset, which can be uploaded and which are primary used for
testing currently. Each dataset of this type requires the file-path to the local input- and
label-file of the same dataset.

!!! warning

    Because of a lack of validation at the moment, it is easy to break the backend with unexpected
    input.

=== "CLI"

    ```bash
    ainarictl dataset create mnist -i <PATH_TO_INPUT_FILE> -l <PATH_TO_LABEL_FILE> <NAME>
    ```

    example:

    ```bash
    ainarictl dataset create mnist -i /tmp/train-images-idx3-ubyte -l /tmp/train-labels-idx1-ubyte cli_test_dataset

    +-------------------+-----------------------------------------------------------------------------------------------+
    | UUID              | 146bacb3-b5bf-485b-a2e8-d1812b57eb63                                                          |
    | NAME              | cli_test_dataset                                                                              |
    | VERSION           | v1.0alpha                                                                                     |
    | NUMBER OF COLUMNS | 794                                                                                           |
    | NUMBER OF ROWS    | 60000                                                                                         |
    | DESCRIPTION       | {"label":{"column_end":794,"column_start":784},"picture":{"column_end":784,"column_start":0}} |
    | VISIBILITY        | private                                                                                       |
    | OWNER ID          | asdf                                                                                          |
    | PROJECT ID        | admin                                                                                         |
    | CREATED AT        | <nil>                                                                                         |
    +-------------------+-----------------------------------------------------------------------------------------------+
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import dataset

    address = "http://127.0.0.1:11417"
    train_dataset_name = "train_test_dataset"
    train_inputs = "/tmp/mnist/train-images.idx3-ubyte"
    train_labels = "/tmp/mnist/train-labels.idx1-ubyte"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    dataset_uuid = dataset.upload_mnist_files(context, train_dataset_name, train_inputs, train_labels)

    # example-content of dataset_uuid:
    #
    # 6f2bbcd2-7081-4b08-ae1d-16e6cd6f54c4
    ```

### Upload CSV-Dataset

!!! warning

    Because of a lack of validation at the moment, it is easy to break the backend with unexpected
    input.

=== "CLI"

    ```bash
    ainarictl dataset create csv -i <PATH_TO_INPUT_FILE> <NAME>
    ```

    example:

    ```bash
    ainarictl dataset create csv -i /tmp/test.csv test_csv

    +-------------------+--------------------------------------------------------------------------------------------------+
    | UUID              | 0923f01b-90ff-4323-9c18-fcfb655985d4                                                             |
    | NAME              | test_csv                                                                                         |
    | VERSION           | v1.0alpha                                                                                        |
    | NUMBER OF COLUMNS | 2                                                                                                |
    | NUMBER OF ROWS    | 1723                                                                                             |
    | DESCRIPTION       | {"test_input":{"column_end":1,"column_start":0},"test_output":{"column_end":2,"column_start":1}} |
    | TASK UUID         | 6f2bbcd2-7081-4b08-ae1d-16e6cd6f54c4                                                             |
    | VISIBILITY        | private                                                                                          |
    | OWNER ID          | asdf                                                                                             |
    | PROJECT ID        | admin                                                                                            |
    | CREATED AT        | 2025-03-15 22:02:47                                                                              |
    +-------------------+--------------------------------------------------------------------------------------------------+
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import dataset

    address = "http://127.0.0.1:11417"
    train_dataset_name = "train_test_dataset"
    train_inputs = "/tmp/csv/test-file.csv"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    dataset_uuid = dataset.upload_csv_files(context, train_dataset_name, train_inputs)

    # example-content of dataset_uuid:
    #
    # 6f2bbcd2-7081-4b08-ae1d-16e6cd6f54c4
    ```

### Get Dataset

Get information about a specific dataset.

=== "CLI"

    ```bash
    ainarictl dataset get <DATASET_UUID>
    ```

    example:

    ```bash
    ainarictl dataset get 146bacb3-b5bf-485b-a2e8-d1812b57eb63

    +-------------------+-----------------------------------------------------------------------------------------------+
    | UUID              | 91c3799d-556b-4562-ae6d-96d631a46a42                                                          |
    | NAME              | cli_test_dataset_req                                                                          |
    | VERSION           | v1.0alpha                                                                                     |
    | NUMBER OF COLUMNS | 794                                                                                           |
    | NUMBER OF ROWS    | 10000                                                                                         |
    | DESCRIPTION       | {"label":{"column_end":794,"column_start":784},"picture":{"column_end":784,"column_start":0}} |
    | TASK UUID         | 6f2bbcd2-7081-4b08-ae1d-16e6cd6f54c4                                                          |
    | VISIBILITY        | private                                                                                       |
    | OWNER ID          | asdf                                                                                          |
    | PROJECT ID        | admin                                                                                         |
    | CREATED AT        | 2025-03-15 22:02:47                                                                           |
    +-------------------+-----------------------------------------------------------------------------------------------+
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import dataset

    address = "http://127.0.0.1:11417"
    dataset_uuid = "6f2bbcd2-7081-4b08-ae1d-16e6cd6f54c4"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = dataset.get_dataset(context, dataset_uuid)

    # example-content of result:
    #
    # {
    #     "inputs": 784,
    #     "lines": 60000,
    #     "location": "/etc/ainari/datasets/6f2bbcd2-7081-4b08-ae1d-16e6cd6f54c4_mnist_asdf",
    #     "name": "train_test_dataset",
    #     "outputs": 10,
    #     "owner_id": "asdf",
    #     "project_id": "admin",
    #     "type": "mnist",
    #     "task_uuid": "6f2bbcd2-7081-4b08-ae1d-16e6cd6f54c4",
    #     "uuid": "6f2bbcd2-7081-4b08-ae1d-16e6cd6f54c4",
    #     "visibility": "private"
    # }
    ```

### List Datasets

List all visible datasets.

=== "CLI"

    ```bash
    ainarictl dataset list
    ```

    example:

    ```bash
    ainarictl dataset list

    +--------------------------------------+------------------------+--------------------------------------+------------+----------+------------+---------------------+
    |                 UUID                 |          NAME          |              TASK UUID               | VISIBILITY | OWNER ID | PROJECT ID |     CREATED AT      |
    +--------------------------------------+------------------------+--------------------------------------+------------+----------+------------+---------------------+
    | 8126302c-6d51-43d5-8f34-2c0a574b9ed7 | cli_test_dataset_train |                                      | private    | asdf     | admin      | 2025-03-15 22:02:45 |
    | 91c3799d-556b-4562-ae6d-96d631a46a42 | cli_test_dataset_req   |                                      | private    | asdf     | admin      | 2025-03-15 22:02:47 |
    | 91c3799d-556b-4562-ae6d-96d631a46a42 | cli_request_test_task  | 6f2bbcd2-7081-4b08-ae1d-16e6cd6f54c4 | private    | asdf     | admin      | 2025-03-15 22:02:47 |
    +--------------------------------------+------------------------+--------------------------------------+------------+----------+------------+---------------------+
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import dataset

    address = "http://127.0.0.1:11417"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = dataset.list_datasets(token, address)

    # example-content of result:
    #
    # {
    #     "body": [
    #         [
    #             "2025-03-15 21:18:52",
    #             "329553e0-c4c4-4139-95ee-3ace764739a5",
    #             "admin",
    #             "asdf",
    #             "private",
    #             "cli_test_dataset_train",
    #             ""
    #         ],
    #         [
    #             "2025-03-15 21:18:54",
    #             "887d1b59-8a3e-4814-b148-8405c1d240e0",
    #             "admin",
    #             "asdf",
    #             "private",
    #             "cli_test_dataset_req",
    #             ""
    #         ],
    #         [
    #             "2025-03-15 21:20:17",
    #             "38f464b8-d627-4ab5-944c-94391dd1962d",
    #             "admin",
    #             "asdf",
    #             "private",
    #             "cli_request_test_task",
    #             "38f464b8-d627-4ab5-944c-94391dd1962d"
    #         ]
    #     ],
    #     "header": [
    #         "created_at",
    #         "uuid",
    #         "project_id",
    #         "owner_id",
    #         "visibility",
    #         "name",
    #         "task_uuid"
    #     ]
    # }
    ```

### Delete Dataset

Delete a dataset.

=== "CLI"

    ```bash
    ainarictl dataset delete <DATASET_UUID>
    ```

    example:

    ```bash
    ainarictl dataset delete 146bacb3-b5bf-485b-a2e8-d1812b57eb63

    successfully deleted dataset '146bacb3-b5bf-485b-a2e8-d1812b57eb63'
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import dataset

    address = "http://127.0.0.1:11417"
    dataset_uuid = "6f2bbcd2-7081-4b08-ae1d-16e6cd6f54c4"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    dataset.delete_dataset(context, dataset_uuid)
    ```

### Delete all Datasets

Delete all datasets.

=== "CLI"

    (not implemented yet)

=== "Python-SDK"

    ```python
    from ainari_sdk import dataset

    address = "http://127.0.0.1:11417"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    dataset.delete_all_datasets(token, address)
    ```

### Check MNIST Dataset Result

Checks a resulting dataset from an MNIST-test against a reference-dataset to compare how much of the
output of the network was correct. The output gives the percentage of the correct output-values. It
is primary used for automatic testing.

=== "CLI"

    ```bash
    ainarictl dataset check -r <REFERENCE_DATASET_UUID> <COMPARE_DATASET_UUID>
    ```

    example:

    ```bash
    ainarictl dataset check -r 6f2bbcd2-7081-4b08-ae1d-16e6cd6f54c4 d40c0c06-bd28-49a4-b872-6a70c4750bb9

    +----------+-------------------+
    | ACCURACY | 91.22999572753906 |
    +----------+-------------------+
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import request_result

    address = "http://127.0.0.1:11417"
    reference_dataset_uuid = "c7f7e274-5d7d-4696-8591-18441cb1b685"
    dataset_uuid = "d40c0c06-bd28-49a4-b872-6a70c4750bb9"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = request_result.check_mnist_dataset(token,
                                                address,
                                                dataset_uuid,
                                                reference_dataset_uuid)

    # example-content of result:
    #
    # {
    #     "accuracy": 93.40999603271484
    # }
    ```

### Download Dataset content

At the moment it is not possible to download complete datasets via websocket, like it is done for
the upload. For now there is only an endpoint to request a slice of a dataset.

=== "CLI"

    ```bash
    ainarictl dataset content -c <COLUMN_NAME>  -n <NUMBER_OF_ROWS> -o <ROW_OFFSET>  <DATASET_UUID>
    ```

    example:

    ```bash
    ainarictl dataset content -c test_output -o 100 -n 10 718566ed-b8a7-4d69-8cf5-d3eb7d75e30b

    +-----+----------+----------+----------+----------+----------+----------+----------+----------+----------+----------+
    |     |    0     |    1     |    2     |    3     |    4     |    5     |    6     |    7     |    8     |    9     |
    +-----+----------+----------+----------+----------+----------+----------+----------+----------+----------+----------+
    | 100 | 0.016387 | 0.002124 | 0.010096 | 0.000188 | 0.017916 | 0.001169 | 0.964437 | 0.003989 | 0.002181 | 0.228054 |
    | 101 | 0.916684 | 0.002899 | 0.003278 | 0.000233 | 0.000781 | 0.026245 | 0.001160 | 0.004049 | 0.021715 | 0.001850 |
    | 102 | 0.001839 | 0.000525 | 0.000984 | 0.017098 | 0.001073 | 0.912213 | 0.000645 | 0.027830 | 0.022612 | 0.016936 |
    | 103 | 0.001453 | 0.000520 | 0.001891 | 0.000291 | 0.978680 | 0.002586 | 0.001325 | 0.008534 | 0.000475 | 0.003352 |
    | 104 | 0.002475 | 0.000776 | 0.002657 | 0.085600 | 0.004949 | 0.008605 | 0.001795 | 0.008018 | 0.000428 | 0.842937 |
    | 105 | 0.002862 | 0.002042 | 0.000669 | 0.000687 | 0.038985 | 0.004651 | 0.001625 | 0.014875 | 0.005904 | 0.969930 |
    | 106 | 0.005426 | 0.003454 | 0.940081 | 0.012480 | 0.002740 | 0.002005 | 0.007840 | 0.002082 | 0.001886 | 0.000216 |
    | 107 | 0.005302 | 0.903132 | 0.005140 | 0.005941 | 0.000177 | 0.009913 | 0.002384 | 0.002448 | 0.130046 | 0.003401 |
    | 108 | 0.000278 | 0.000692 | 0.000237 | 0.010781 | 0.005375 | 0.037098 | 0.007750 | 0.012575 | 0.004456 | 0.901723 |
    | 109 | 0.002321 | 0.000556 | 0.001553 | 0.000226 | 0.858400 | 0.002418 | 0.006818 | 0.017290 | 0.001165 | 0.016600 |
    +-----+----------+----------+----------+----------+----------+----------+----------+----------+----------+----------+
    ```

    (first column of the table is the row-counter starting by the defined row-offset)

=== "Python-SDK"

    ```python
    from ainari_sdk import request_result

    address = "http://127.0.0.1:11417"
    dataset_uuid = "d40c0c06-bd28-49a4-b872-6a70c4750bb9"
    column_name = "test_output"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = dataset.download_dataset_content(token,
                                              address,
                                              dataset_uuid,
                                              column_name,
                                              2,    # number of rows
                                              100)  # offset

    # example-content of result:
    #
    # {
    #     "data": [
    #         [
    #             0.0035933551844209433,
    #             0.0016254606889560819,
    #             0.024600498378276825,
    #             0.002956731477752328,
    #             0.002234742045402527,
    #             0.0011109848273918033,
    #             0.8660337328910828,
    #             0.00447552977129817,
    #             0.04139076545834541,
    #             0.0272488035261631
    #         ],
    #         [
    #             0.8367037773132324,
    #             0.002998552517965436,
    #             0.0006984848878346384,
    #             0.008744454011321068,
    #             0.00039861834375187755,
    #             0.013055858202278614,
    #             0.00038920185761526227,
    #             0.003807016182690859,
    #             0.005977323278784752,
    #             0.004582217428833246
    #         ]
    #     ]
    # }

    ```

## Model

Model containing the neural network.

### Create Model

To initialize a new model, a model-templated is used, which describes the basic structure of the
network (see documentation of the
[model-templates](https://docs.ainari.cloud/api/model_template/))

=== "CLI"

    ```bash
    ainarictl model create -t <PATH_TO_TEMPLATE> <NAME>
    ```

    example:

    ```bash
    ainarictl model create -t ./model_template cli_test_model

    +------------+--------------------------------------+
    | UUID       | 12959485-51a7-45bc-84dd-aad1c9bfd510 |
    | NAME       | cli_test_model                     |
    | VISIBILITY | private                              |
    | OWNER ID   | asdf                                 |
    | PROJECT ID | admin                                |
    | CREATED AT | 2024-07-13 21:45:56                  |
    +------------+--------------------------------------+
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import model

    address = "http://127.0.0.1:11417"
    model_name = "test_model"
    model_template = \
        "version: 1\n" \
        "settings:\n" \
        "    neuron_cooldown: 10000000.0\n" \
        "    refractory_time: 1\n" \
        "    max_connection_distance: 1\n" \
        "hexagons:\n" \
        "    1,1,1\n" \
        "    2,1,1\n" \
        "    3,1,1\n" \
        "    \n" \
        "inputs:\n" \
        "    picture: 1,1,1\n" \
        "\n" \
        "outputs:\n" \
        "    label: 3,1,1\n" \

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = model.create_model(context, model_name, model_template)

    # example-content of result:
    #
    # {
    #     "name": "test_model",
    #     "owner_id": "asdf",
    #     "project_id": "admin",
    #     "uuid": "d94f2b53-f404-4215-9a33-63c4a03e3202",
    #     "visibility": "private"
    # }
    ```

### Get Model

Get information of a specific model.

!!! info It is basically the same output like coming from the create command and contains only the

data stored in the database. Information about the model itself, like number of neurons, amount of
used memory and so on are still missing in this output currently.

=== "CLI"

    ```bash
    ainarictl model get <CLUSTER_UUID>
    ```

    example:

    ```bash
    ainarictl model get 12959485-51a7-45bc-84dd-aad1c9bfd510

    +--------------------+--------------------------------------+
    | UUID               | 12959485-51a7-45bc-84dd-aad1c9bfd510 |
    | NAME               | cli_test_model                     |
    | VISIBILITY         | private                              |
    | OWNER ID           | asdf                                 |
    | NUMBER OF BLOCKS   | 3                                    |
    | NUMBER OF SECTIONS | 973                                  |
    | PROJECT ID         | admin                                |
    | CREATED AT         | 2024-07-13 21:45:56                  |
    +--------------------+--------------------------------------+
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import model

    address = "http://127.0.0.1:11417"
    model_uuid = "d94f2b53-f404-4215-9a33-63c4a03e3202"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = model.get_model(context, model_uuid)

    # example-content of result:
    #
    # {
    #     "name": "test_model",
    #     "owner_id": "asdf",
    #     "project_id": "admin",
    #     "number_of_blocks": 3,
    #     "number_of_sections": 973,
    #     "uuid": "d94f2b53-f404-4215-9a33-63c4a03e3202",
    #     "visibility": "private"
    # }
    ```

### List Model

List all visible model.

=== "CLI"

    ```bash
    ainarictl model list
    ```

    example:

    ```bash
    ainarictl model list

    +--------------------------------------+------------------+------------+----------+------------+---------------------+
    |                 UUID                 |       NAME       | VISIBILITY | OWNER ID | PROJECT ID |     CREATED AT      |
    +--------------------------------------+------------------+------------+----------+------------+---------------------+
    | 12959485-51a7-45bc-84dd-aad1c9bfd510 | cli_test_model | private    | asdf     | admin      | 2024-07-13 21:45:56 |
    +--------------------------------------+------------------+------------+----------+------------+---------------------+
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import model

    address = "http://127.0.0.1:11417"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = model.list_models(token, address)

    # example-content of result:
    #
    # {
    #     "body": [
    #         [
    #             "d94f2b53-f404-4215-9a33-63c4a03e3202",
    #             "admin",
    #             "asdf",
    #             "private",
    #             "test_model"
    #         ]
    #     ],
    #     "header": [
    #         "uuid",
    #         "project_id",
    #         "owner_id",
    #         "visibility",
    #         "name"
    #     ]
    # }
    ```

### Delete Model

Delete a model from a backend.

=== "CLI"

    ```bash
    ainarictl model delete <CLUSTER_UUID>
    ```

    example:

    ```bash
    ainarictl model delete 12959485-51a7-45bc-84dd-aad1c9bfd510

    successfully deleted model '12959485-51a7-45bc-84dd-aad1c9bfd510'
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import model

    address = "http://127.0.0.1:11417"
    model_uuid = "d94f2b53-f404-4215-9a33-63c4a03e3202"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    model.delete_model(context, model_uuid)
    ```

### Delete all Model

Delete all model.

=== "CLI"

    (not implemented yet)

=== "Python-SDK"

    ```python
    from ainari_sdk import model

    address = "http://127.0.0.1:11417"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    model.delete_all_model(token, address)
    ```

### Create Checkpoint of Model

Save the state of the model by creating a checkpoint, which is stored on the server.

=== "CLI"

    ```bash
    ainarictl model save -n <NAME> <CLUSTER_UUID>
    ```

    example:

    ```bash
    ainarictl model save -n cli_test_checkpoint d28d72f0-f95f-42bd-b14d-b7e12d3b9d82

    +------------+--------------------------------------+
    | UUID       | d28d72f0-f95f-42bd-b14d-b7e12d3b9d82 |
    | NAME       | cli_test_checkpoint                  |
    +------------+--------------------------------------+
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import model

    address = "http://127.0.0.1:11417"
    checkpoint_name = "test_checkpoint"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = model.save_model(context, checkpoint_name, model_uuid)

    # example-content of result:
    #
    # {
    #     "name": "test_checkpoint",
    #     "uuid": "d7130869-520f-4743-8f90-4f17f3382321"
    # }
    ```

### Restore Checkpoint of Model

Reset a model to the state, which is stored in a specific checkpoint.

=== "CLI"

    ```bash
    ainarictl model restore -c <CHECKPOINT_UUID> <CLUSTER_UUID>
    ```

    example:

    ```bash
    ainarictl model restore -c d28d72f0-f95f-42bd-b14d-b7e12d3b9d82 cc6120c7-cc31-4f17-baee-c6c606f00512

    +------------+--------------------------------------+
    | UUID       | 6e7a911e-5f81-4ffb-9de0-2b6717b1be52 |
    +------------+--------------------------------------+
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import model

    address = "http://127.0.0.1:11417"
    checkpoint_uuid = "cc6120c7-cc31-4f17-baee-c6c606f00512"
    model_uuid = "d94f2b53-f404-4215-9a33-63c4a03e3202"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = model.restore_model(context, checkpoint_uuid, model_uuid)

    # example-content of result:
    #
    # {
    #     "uuid": "e27e4834-64df-4db2-883e-9bfb44bc6753"
    # }
    ```

### Switch Host

Each CPU and GPU is handled as its logical host. Model and be moved between them. To list
avaialble hosts there is the
[list-hosts endpoint](https://docs.ainari.cloud/api/sdk_library/#list-hosts).

!!! warning

    GPU-support is not available at the moment and multi CPU is also still not supported, so this
    function is basically not avaiable in the current state

=== "CLI"

    (Not implemented by the CLI)

=== "Python-SDK"

    ```python
    from ainari_sdk import model

    address = "http://127.0.0.1:11417"
    host_uuid = "cc6120c7-cc31-4f17-baee-c6c606f00512"
    model_uuid = "d94f2b53-f404-4215-9a33-63c4a03e3202"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = model.switch_host(context, model_uuid, host_uuid)

    # example-content of result:
    #
    # {
    #     "name": "test_model",
    #     "owner_id": "asdf",
    #     "project_id": "admin",
    #     "uuid": "d94f2b53-f404-4215-9a33-63c4a03e3202",
    #     "visibility": "private"
    # }
    ```

## Task

Tasks are asynchronous actions, which are placed within a queue of the model, which should be
affected by the task. They are processed one after another.

### Create Train-Task

Create a new task to train the model with the data of a dataset, which was uploaded before.

=== "CLI"

    ```bash
    ainarictl task create train -j \
    -i <INPUT_DATASET_UUID>:<INPUT_DATASET_COLUMN_NAME>:<INPUT_HEXAGON_NAME> \
    -o <LABEL_DATASET_UUID>:<LABEL_DATASET_COLUMN_NAME>:<LABEL_HEXAGON_NAME> \
    -c <CLUSTER_UUID> \
    <TASK_NAME>
    ```

    example:

    ```bash
    ainarictl task create train -j \
    -i b03d1682-8f5b-48cb-bff5-08b67e8de6fe:picture:picture_hexagon \
    -o b833ddbe-55db-49d5-97b7-771293505493:label:label_hexagon \
    -c 9f86921d-9a7c-44a2-836c-1683928d9354 \
    cli_train_test_task

    +------------------------+--------------------------------------+
    | UUID                   | 2e28e3bb-af45-4fbc-8ce3-c0c9a8e704bc |
    | STATE                  | active                               |
    | CURRENT CYCLE          | 171                                  |
    | TOTAL NUMBER OF CYCLES | 60000                                |
    | QUEUE TIMESTAMP        | 2024-07-13 22:21:13                  |
    | START TIMESTAMP        | 2024-07-13 22:21:13                  |
    | END TIMESTAMP          | -                                    |
    +------------------------+--------------------------------------+
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import task

    address = "http://127.0.0.1:11417"
    task_name = "test_task"
    model_uuid = "9f86921d-9a7c-44a2-836c-1683928d9354"
    inputs = [
        {
            "dataset_uuid": "6f2bbcd2-7081-4b08-ae1d-16e6cd6f54c4",
            "dataset_column": "picture",
            "hexagon_name": "picture_hex"
        }
    ]

    outputs = [
        {
            "dataset_uuid": "6f2bbcd2-7081-4b08-ae1d-16e6cd6f54c4",
            "dataset_column": "label",
            "hexagon_name": "label_hex"
        }
    ]


    # inputs and outputs are maps with key-value-pairs,
    # where the key is the name of the hexagon of the matching
    # field within the dataset and the value is the UUID of
    # the used dataset for the input- and output-data

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = task.create_train_task(token,
                                    address,
                                    task_name,
                                    model_uuid,
                                    inputs,
                                    outputs)
    task_uuid = result["uuid"]

    # optional you can wait until the task is finished
    task.wait_for_task_finished(context, task_uuid, model_uuid, 1.0)
    ```

### Create Request-Task

Create a new task to request information from a trained model. As input the data of a dataset are
used, which had to be uplaoded first.

=== "CLI"

    ```bash
    ainarictl task create request -j \
    -i <INPUT_DATASET_UUID>:<INPUT_DATASET_COLUMN_NAME>:<INPUT_HEXAGON_NAME> \
    -r <OUTPUT_HEXAGON_NAME>:<OUTPUT_DATASET_COLUMN_NAME> \
    -c <CLUSTER_UUID> \
    <TASK_NAME>
    ```

    example:

    ```bash
    ainarictl task create train -j \
    -i b03d1682-8f5b-48cb-bff5-08b67e8de6fe:picture:picture_hexagon \
    -r label_hexagon:output_data \
    -c 9f86921d-9a7c-44a2-836c-1683928d9354 \
    cli_train_test_task

    +------------------------+--------------------------------------+
    | UUID                   | ec964017-ee19-4775-8fff-4f3fb3640361 |
    | STATE                  | finished                             |
    | CURRENT CYCLE          | 10000                                |
    | TOTAL NUMBER OF CYCLES | 10000                                |
    | QUEUE TIMESTAMP        | 2024-07-13 22:21:23                  |
    | START TIMESTAMP        | 2024-07-13 22:21:23                  |
    | END TIMESTAMP          | 2024-07-13 22:21:23                  |
    +------------------------+--------------------------------------+
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import task

    address = "http://127.0.0.1:11417"
    task_name = "test_task"
    model_uuid = "9f86921d-9a7c-44a2-836c-1683928d9354"
    inputs = [
        {
            "dataset_uuid": "6f2bbcd2-7081-4b08-ae1d-16e6cd6f54c4",
            "dataset_column": "picture",
            "hexagon_name": "picture_hex"
        }
    ]

    results = [
        {
            "dataset_column": "test_output",
            "hexagon_name": "label_hex"
        }
    ]


    # inputs and results are maps with key-value-pairs,
    # where the key is the name of the hexagon of the matching
    # field within the dataset and the value is the UUID of
    # the used dataset for the input- and output-data

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = task.create_request_task(token,
                                      address,
                                      task_name,
                                      model_uuid,
                                      inputs,
                                      results)
    task_uuid = result["uuid"]

    # optional you can wait until the task is finished
    task.wait_for_task_finished(context, task_uuid, model_uuid, 1.0)
    ```

### Get Task

=== "CLI"

    ```bash
    ainarictl task get -c <CLUSTER_UUID> <TASK_UUID>
    ```

    example:

    ```bash
    ainarictl task get -c 9f86921d-9a7c-44a2-836c-1683928d93542e28e3bb-af45-4fbc-8ce3-c0c9a8e704bc

    +------------------------+--------------------------------------+
    | UUID                   | 2e28e3bb-af45-4fbc-8ce3-c0c9a8e704bc |
    | STATE                  | finished                             |
    | CURRENT CYCLE          | 60000                                |
    | TOTAL NUMBER OF CYCLES | 60000                                |
    | QUEUE TIMESTAMP        | 2024-07-13 22:21:13                  |
    | START TIMESTAMP        | 2024-07-13 22:21:13                  |
    | END TIMESTAMP          | 2024-07-13 22:21:21                  |
    +------------------------+--------------------------------------+
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import task

    address = "http://127.0.0.1:11417"
    model_uuid = "9f86921d-9a7c-44a2-836c-1683928d9354"
    task_uuid = "c7f7e274-5d7d-4696-8591-18441cb1b685"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = task.get_task(context, task_uuid, model_uuid)

    # example-content of result:
    #
    # {
    #     "end_timestamp": "-",
    #     "percentage_finished": 0.6904833316802979,
    #     "queue_timestamp": "2024-01-08 21:19:16",
    #     "start_timestamp": "2024-01-08 21:19:16",
    #     "state": "active"
    # }
    ```

### List Task

List all tasks for a model, together with their progress.

=== "CLI"

    ```bash
    ainarictl task list -c <CLUSTER_UUID>
    ```

    example:

    ```bash
    ./ainarictl task list -c 49d50999-c47f-48cb-906b-211218f897e4

    +--------------------------------------+----------+
    |                 UUID                 |  STATE   |
    +--------------------------------------+----------+
    | 97bdb8e7-c23f-41dc-af92-5cb77e2843a4 | active   |
    | efb1eb3b-a3fd-4dca-9cfa-84d728dc69eb | finished |
    +--------------------------------------+----------+
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import task

    address = "http://127.0.0.1:11417"
    model_uuid = "9f86921d-9a7c-44a2-836c-1683928d9354"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = task.list_tasks(context, model_uuid)

    # example-content of result:
    #
    # {
    #     "body": [
    #         [
    #             "ef2ee9e9-d724-49bb-b656-2d5e3484b9f3",
    #             "finished",
    #             "1.000000",
    #             "2024-01-08 21:19:23",
    #             "2024-01-08 21:19:23",
    #             "2024-01-08 21:19:23"
    #         ]
    #     ],
    #     "header": [
    #         "uuid",
    #         "state",
    #         "percentage",
    #         "queued",
    #         "start",
    #         "end"
    #     ]
    # }
    ```

### Delete Task

Delete a task from a model. In this task was a request and produced a request-result, this result
will not be deleted.

=== "CLI"

    ```bash
    ainarictl task delete -c <CLUSTER_UUID> <TASK_UUID>
    ```

    example:

    ```bash
    ainarictl task delete -c 49d50999-c47f-48cb-906b-211218f897e4 ddbf5bc1-3487-4755-8651-a96842ccec12

    successfully deleted task 'ddbf5bc1-3487-4755-8651-a96842ccec12'
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import task

    address = "http://127.0.0.1:11417"
    model_uuid = "9f86921d-9a7c-44a2-836c-1683928d9354"
    task_uuid = "c7f7e274-5d7d-4696-8591-18441cb1b685"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    task.delete_task(context, task_uuid, model_uuid)
    ```

### Wait until Task is finished

Delete a task from a model. In this task was a request and produced a request-result, this result
will not be deleted.

=== "CLI"

    (not implemented yet)

=== "Python-SDK"

    ```python
    from ainari_sdk import task

    address = "http://127.0.0.1:11417"
    model_uuid = "9f86921d-9a7c-44a2-836c-1683928d9354"
    task_uuid = "c7f7e274-5d7d-4696-8591-18441cb1b685"
    time_interval = 0.1  # time-interval to check if finished, in seconds

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    task.wait_for_task_finished(context, task_uuid, model_uuid, time_interval)
    ```

## Checkpoint

Checkpoints are a copy of the current state of a model. It can be used as backup to restore an
older state a model.

!!! info

    It is possible to apply a checkpoint to any model, but at the moment it is not possible to
    directly create a new model out of a checkpoint.

### List Checkpoints

List all visible checkpoints.

=== "CLI"

    ```bash
    ainarictl checkpoint list
    ```

    example:

    ```bash
    ainarictl checkpoint list

    +--------------------------------------+---------------------+------------+----------+------------+---------------------+
    |                 UUID                 |        NAME         | VISIBILITY | OWNER ID | PROJECT ID |     CREATED AT      |
    +--------------------------------------+---------------------+------------+----------+------------+---------------------+
    | 9303816f-e575-410b-a75d-8444ff3ac303 | cli_test_checkpoint | private    | asdf     | admin      | 2024-07-12 20:46:50 |
    +--------------------------------------+---------------------+------------+----------+------------+---------------------+
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import checkpoint

    address = "http://127.0.0.1:11417"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = checkpoint.list_checkpoints(token, address)

    # example-content of result:
    #
    # {
    #     "body": [
    #         [
    #             "cc6120c7-cc31-4f17-baee-c6c606f00512",
    #             "admin",
    #             "asdf",
    #             "private",
    #             "test_checkpoint"
    #         ]
    #     ],
    #     "header": [
    #         "uuid",
    #         "project_id",
    #         "owner_id",
    #         "visibility",
    #         "name"
    #     ]
    # }
    ```

### Delete Checkpoint

Delete a checkpoint from the backend.

=== "CLI"

    ```bash
    ainarictl checkpoint delete <CHECKPOINT_UUID>
    ```

    example:

    ```bash
    ainarictl checkpoint delete 84eaae8e-aeae-4db4-840e-ca38f4461ec7

    successfully deleted checkpoint '84eaae8e-aeae-4db4-840e-ca38f4461ec7'
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import checkpoint

    address = "http://127.0.0.1:11417"
    checkpoint_uuid = "cc6120c7-cc31-4f17-baee-c6c606f00512"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    checkpoint.delete_checkpoint(context, checkpoint_uuid)

    ```

### Delete all Checkpoints

Delete all checkpoint.

=== "CLI"

    (not implemented yet)

=== "Python-SDK"

    ```python
    from ainari_sdk import checkpoint

    address = "http://127.0.0.1:11417"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    checkpoint.delete_all_checkpoints(token, address)
    ```

## Hosts

### List Hosts

Each CPU and GPU is handled as its own logical host to have more control over the exact location of
the data. These logical hosts can be listed with this endpoint.

=== "CLI"

    ```bash
    ainarictl host list
    ```

    example:

    ```bash
    ainarictl host list

    +--------------------------------------+------+
    |                 UUID                 | TYPE |
    +--------------------------------------+------+
    | e82a8848-e23c-4c60-9017-d98414cf3c0d | cpu  |
    +--------------------------------------+------+
    ```

=== "Python-SDK"

    ```python
    from ainari_sdk import hosts

    address = "http://127.0.0.1:11417"

    # request a token for a user, who has admin-permissions
    # see: https://docs.ainari.cloud/api/sdk_library/#request-token

    result = hosts.list_hosts(token, address)

    # example-content of result:
    #
    # {
    #     "body": [
    #         [
    #             "cc6120c7-cc31-4f17-baee-c6c606f00512",
    #             "cpu",
    #         ]
    #     ],
    #     "header": [
    #         "uuid",
    #         "type"
    #     ]
    # }
    ```
