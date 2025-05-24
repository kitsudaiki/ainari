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

from . import hanami_request
# from .hanami_messages import proto3_pb2


def list_datasets(token: str,
                  address: str,
                  verify_connection: bool = True) -> dict:
    path = "/v1alpha/dataset"
    return hanami_request.send_get_request(token,
                                           address,
                                           path,
                                           "",
                                           verify=verify_connection)


def get_dataset(token: str,
                address: str,
                dataset_uuid: str,
                verify_connection: bool = True) -> dict:
    path = f"/v1alpha/dataset/{dataset_uuid}"
    return hanami_request.send_get_request(token,
                                           address,
                                           path,
                                           "",
                                           verify=verify_connection)


def delete_dataset(token: str,
                   address: str,
                   dataset_uuid: str,
                   verify_connection: bool = True):
    path = f"/v1alpha/dataset/{dataset_uuid}"
    hanami_request.send_delete_request(token,
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


def check_mnist_dataset(token: str,
                        address: str,
                        dataset_uuid: str,
                        reference_dataset_uuid: str,
                        verify_connection: bool = True) -> dict:
    path = "/v1alpha/dataset/check"
    values = f'uuid={dataset_uuid}&reference_uuid={reference_dataset_uuid}'
    return hanami_request.send_get_request(token,
                                           address,
                                           path,
                                           values,
                                           verify=verify_connection)


def download_dataset_content(token: str,
                             address: str,
                             dataset_uuid: str,
                             column_name: str,
                             number_of_rows: int,
                             row_offset: int = 0,
                             verify_connection: bool = True) -> dict:
    path = "/v1alpha/dataset/content"
    values = f'uuid={dataset_uuid}&column_name={column_name}&row_offset={row_offset}' \
        f'&number_of_rows={number_of_rows}'
    return hanami_request.send_get_request(token,
                                           address,
                                           path,
                                           values,
                                           verify=verify_connection)


def upload_mnist_files(token: str,
                       address: str,
                       name: str,
                       input_file_path: str,
                       label_file_path: str,
                       verify_connection: bool = True) -> str:
    path = f"/v1alpha/dataset/mnist/{name}"
    files = [input_file_path, label_file_path]

    return hanami_request.upload_files(token,
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

    return hanami_request.upload_files(token,
                                       address,
                                       path,
                                       files,
                                       verify=verify_connection)
