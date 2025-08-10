# Copyright 2022 Tobias Anker
#
# Licensed under the Apache License, Version 2.0 (the "License");
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

from . import ainari_request
# from .ainari_messages import proto3_pb2


def list_datasets(token: str,
                  address: str,
                  verify_connection: bool = True) -> dict:
    path = "/v1alpha/dataset"
    return ainari_request.send_get_request(token,
                                           address,
                                           path,
                                           "",
                                           verify=verify_connection)


def get_dataset(token: str,
                address: str,
                dataset_uuid: str,
                verify_connection: bool = True) -> dict:
    path = f"/v1alpha/dataset/{dataset_uuid}"
    return ainari_request.send_get_request(token,
                                           address,
                                           path,
                                           "",
                                           verify=verify_connection)


def delete_dataset(token: str,
                   address: str,
                   dataset_uuid: str,
                   verify_connection: bool = True):
    path = f"/v1alpha/dataset/{dataset_uuid}"
    ainari_request.send_delete_request(token,
                                       address,
                                       path,
                                       "",
                                       verify=verify_connection)


def delete_all_datasets(token: str,
                        address: str,
                        verify_connection: bool = True):
    body = list_datasets(token, address, False)["datasets"]
    for entry in body:
        delete_dataset(token, address, entry["uuid"], verify_connection)


def check_dataset(token: str,
                  address: str,
                  dataset_uuid: str,
                  dataset_column: str,
                  reference_uuid: str,
                  reference_column: str,
                  verify_connection: bool = True) -> dict:
    path = f"/v1alpha/dataset/{dataset_uuid}/check"
    json_body = {
        "dataset_column": dataset_column,
        "reference_uuid": reference_uuid,
        "reference_column": reference_column,
    }

    return ainari_request.send_put_request(token,
                                           address,
                                           path,
                                           json_body,
                                           verify=verify_connection)


def upload_mnist_files(token: str,
                       address: str,
                       name: str,
                       input_file_path: str,
                       label_file_path: str,
                       verify_connection: bool = True) -> str:
    path = f"/v1alpha/dataset/mnist/{name}"
    files = [input_file_path, label_file_path]

    return ainari_request.upload_files(token,
                                       address,
                                       path,
                                       files,
                                       verify=verify_connection)


def upload_csv_files(token: str,
                     address: str,
                     name: str,
                     input_file_path: str,
                     verify_connection: bool = True) -> str:
    path = f"/v1alpha/dataset/csv/{name}"
    files = [input_file_path]

    return ainari_request.upload_files(token,
                                       address,
                                       path,
                                       files,
                                       verify=verify_connection)
