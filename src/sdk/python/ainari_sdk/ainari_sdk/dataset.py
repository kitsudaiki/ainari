# Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>
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
from .access_context import AccessContext


def list_datasets(context: AccessContext) -> dict:
    path = "/v1alpha/dataset"
    return ainari_request.send_get_request(context,
                                           context.ryokan_adress,
                                           path,
                                           "")


def get_dataset(context: AccessContext,
                dataset_uuid: str) -> dict:
    path = f"/v1alpha/dataset/{dataset_uuid}"
    return ainari_request.send_get_request(context,
                                           context.ryokan_adress,
                                           path,
                                           "")


def delete_dataset(context: AccessContext,
                   dataset_uuid: str):
    path = f"/v1alpha/dataset/{dataset_uuid}"
    ainari_request.send_delete_request(context,
                                       context.ryokan_adress,
                                       path,
                                       "")


def delete_all_datasets(context: AccessContext):
    body = list_datasets(context)["datasets"]
    for entry in body:
        delete_dataset(context, entry["uuid"])


def check_dataset(context: AccessContext,
                  dataset_uuid: str,
                  dataset_column: str,
                  reference_uuid: str,
                  reference_column: str) -> dict:
    path = f"/v1alpha/dataset/{dataset_uuid}/check"
    json_body = {
        "dataset_column": dataset_column,
        "reference_uuid": reference_uuid,
        "reference_column": reference_column,
    }

    return ainari_request.send_put_request(context,
                                           context.ryokan_adress,
                                           path,
                                           json_body)


def upload_mnist_files(context: AccessContext,
                       name: str,
                       input_file_path: str,
                       label_file_path: str) -> dict:
    path = f"/v1alpha/dataset/mnist/{name}"
    files = [input_file_path, label_file_path]

    return ainari_request.upload_files(context,
                                       context.ryokan_adress,
                                       path,
                                       files)


def upload_csv_files(context: AccessContext,
                     name: str,
                     input_file_path: str) -> dict:
    path = f"/v1alpha/dataset/csv/{name}"
    files = [input_file_path]

    return ainari_request.upload_files(context,
                                       context.ryokan_adress,
                                       path,
                                       files)
