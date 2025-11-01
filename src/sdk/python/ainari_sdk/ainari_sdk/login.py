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
from . import ainari_request

# import base64
from .access_context import AccessContext


def request_context(address: str,
                    user_id: str,
                    passphrase: str,
                    verify_connection: bool = True) -> AccessContext:
    auth_url = f'{address}/v1alpha/token'
    # passphrase_bytes = passphrase.encode('utf-8')
    # base64_encoded = base64.b64encode(passphrase_bytes)

    body = "token_format=jwt&grant_type=client_credentials" \
           f'&client_id={user_id}&client_secret={passphrase}'

    resp = requests.post(auth_url, data=body, verify=verify_connection)
    token = ""
    if resp.status_code == 200:
        token = resp.json()["access_token"]
    if resp.status_code == 400:
        raise ainari_exceptions.BadRequestException(resp.content)
    if resp.status_code == 401:
        raise ainari_exceptions.UnauthorizedException(resp.content)
    if resp.status_code == 404:
        raise ainari_exceptions.NotFoundException(resp.content)
    if resp.status_code == 409:
        raise ainari_exceptions.ConflictException(resp.content)
    if resp.status_code == 500:
        raise ainari_exceptions.InternalServerErrorException()

    # request where to reach the other endpoints
    path = "/v1alpha/endpoints"
    resp = ainari_request.send_get_request_without_context(token,
                                                           address,
                                                           path,
                                                           "",
                                                           verify=verify_connection)

    # get addresses
    miko_address = address
    hanami_address = f'{resp["hanami"]["public_address"]}:{resp["hanami"]["public_port"]}'
    bento_address = f'{resp["bento"]["public_address"]}:{resp["bento"]["public_port"]}'
    torii_address = f'{resp["torii"]["public_address"]}:{resp["torii"]["public_port"]}'
    omamori_address = f'{resp["omamori"]["public_address"]}:{resp["omamori"]["public_port"]}'

    torii_address_split = torii_address.split(":")
    torii_base_address = torii_address_split[0] + ":" + torii_address_split[1]

    context = AccessContext(token, miko_address, hanami_address,
                            bento_address, omamori_address, torii_address, torii_base_address)

    # print(context)

    return context
