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

import requests
import json
import os
from requests_toolbelt import MultipartEncoder

from . import hanami_exceptions


def _handle_response(response) -> str:
    if response.status_code >= 200 and response.status_code < 300:
        return response.content
    if response.status_code == 400:
        raise hanami_exceptions.BadRequestException(response.content)
    if response.status_code == 401:
        raise hanami_exceptions.UnauthorizedException(response.content)
    if response.status_code == 404:
        raise hanami_exceptions.NotFoundException(response.content)
    if response.status_code == 409:
        raise hanami_exceptions.ConflictException(response.content)
    if response.status_code == 500:
        raise hanami_exceptions.InternalServerErrorException()


def send_post_request(token: str,
                      address: str,
                      path: str,
                      body: dict,
                      verify: bool) -> dict:
    body_str = json.dumps(body)
    url = f'{address}{path}'
    bearer_token = "Bearer " + token
    headers = {'Authorization': bearer_token,
               'content-type': 'application/json'}
    response = requests.post(url, data=body_str, headers=headers, verify=verify)
    return json.loads(_handle_response(response))


def send_get_request(token: str,
                     address: str,
                     path: str,
                     values: str,
                     verify: bool) -> dict:
    if values:
        url = f'{address}{path}?{values}'
    else:
        url = f'{address}{path}'

    bearer_token = "Bearer " + token
    headers = {'Authorization': bearer_token}
    response = requests.get(url, headers=headers, verify=verify)
    return json.loads(_handle_response(response))


def send_put_request(token: str,
                     address: str,
                     path: str,
                     body: dict,
                     verify: bool) -> dict:
    body_str = json.dumps(body)
    url = f'{address}{path}'
    bearer_token = "Bearer " + token
    headers = {'Authorization': bearer_token,
               'content-type': 'application/json'}
    response = requests.put(url, data=body_str, headers=headers, verify=verify)
    return json.loads(_handle_response(response))


def send_delete_request(token: str,
                        address: str,
                        path: str,
                        values: str,
                        verify: bool):
    url = f'{address}{path}?{values}'
    bearer_token = "Bearer " + token
    headers = {'Authorization': bearer_token}
    response = requests.delete(url, headers=headers, verify=verify)
    _handle_response(response)


def upload_files(token: str,
                 address: str,
                 path: str,
                 file_paths,
                 verify: bool):
    url = f'{address}{path}'
    bearer_token = "Bearer " + token

    fields = {}
    open_files = []

    for i, file_path in enumerate(file_paths):
        f = open(file_path, 'rb')
        open_files.append(f)  # Keep open until after upload!
        fields[f'file{i}'] = (os.path.basename(file_path), f, 'application/octet-stream')

    encoder = MultipartEncoder(fields=fields)
    headers = {
        'Authorization': bearer_token,
        'Content-Type': encoder.content_type
    }

    try:
        response = requests.post(url, data=encoder, headers=headers, verify=verify)
        return json.loads(_handle_response(response))
    except requests.exceptions.RequestException as e:
        raise e
    finally:
        for f in open_files:
            f.close()
