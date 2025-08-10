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
from . import ainari_exceptions
# import base64


def request_token(address: str,
                  user_id: str,
                  passphrase: str,
                  verify_connection: bool = True) -> str:
    url = f'{address}/v1alpha/token'
    # passphrase_bytes = passphrase.encode('utf-8')
    # base64_encoded = base64.b64encode(passphrase_bytes)

    body = "token_format=jwt&grant_type=client_credentials" \
           f'&client_id={user_id}&client_secret={passphrase}'

    response = requests.post(url, data=body, verify=verify_connection)
    if response.status_code == 200:
        return response.json()["access_token"]
    if response.status_code == 400:
        raise ainari_exceptions.BadRequestException(response.content)
    if response.status_code == 401:
        raise ainari_exceptions.UnauthorizedException(response.content)
    if response.status_code == 404:
        raise ainari_exceptions.NotFoundException(response.content)
    if response.status_code == 409:
        raise ainari_exceptions.ConflictException(response.content)
    if response.status_code == 500:
        raise ainari_exceptions.InternalServerErrorException()
