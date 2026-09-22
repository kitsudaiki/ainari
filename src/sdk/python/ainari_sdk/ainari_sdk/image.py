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


def list_images(context: AccessContext) -> dict:
    path = "/v1alpha/image"
    return ainari_request.send_get_request(context,
                                           context.ryokan_adress,
                                           path,
                                           "")


def get_image_count(context: AccessContext) -> dict:
    path = "/v1alpha/image/count"
    return ainari_request.send_get_request(context,
                                           context.ryokan_adress,
                                           path,
                                           "")


def get_image(context: AccessContext,
              image_uuid: str) -> dict:
    path = f"/v1alpha/image/{image_uuid}"
    return ainari_request.send_get_request(context,
                                           context.ryokan_adress,
                                           path,
                                           "")


def delete_image(context: AccessContext,
                 image_uuid: str):
    path = f"/v1alpha/image/{image_uuid}"
    ainari_request.send_delete_request(context,
                                       context.ryokan_adress,
                                       path,
                                       "")


def delete_all_images(context: AccessContext):
    body = list_images(context)["images"]
    for entry in body:
        delete_image(context, entry["uuid"])


def check_image(context: AccessContext,
                image_uuid: str,
                image_column: str,
                reference_uuid: str,
                reference_column: str) -> dict:
    path = f"/v1alpha/image/{image_uuid}/check"
    json_body = {
        "image_column": image_column,
        "reference_uuid": reference_uuid,
        "reference_column": reference_column,
    }

    return ainari_request.send_put_request(context,
                                           context.ryokan_adress,
                                           path,
                                           json_body)


def upload_disk_file(context: AccessContext,
                     name: str,
                     input_file_path: str) -> dict:
    """
    Uploads a disk-image, which is used as boot-disk of a virtual machine. The file is stored as
    it is, so it has to be an image, which cloud-hypervisor can boot, like a qcow2-cloud-image.
    """
    path = f"/v1alpha/image/disk/{name}"
    files = [input_file_path]

    return ainari_request.upload_files(context,
                                       context.ryokan_adress,
                                       path,
                                       files)
