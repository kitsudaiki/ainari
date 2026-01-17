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

import requests
from . import ainari_exceptions
from . import ainari_request

# import base64
from .access_context import AccessContext


def request_context(address: str,
                    user_id: str,
                    passphrase: str,
                    verify_connection: bool = True) -> AccessContext:
    """
    Authenticates with the API and retrieves the necessary context for making subsequent requests.

    Args:
        address: The base URL of the API.
        user_id: The client ID for authentication.
        passphrase: The client secret for authentication.
        verify_connection: Whether to verify the SSL certificate (default: True).

    Returns:
        AccessContext: An object containing the authentication token and endpoint addresses.

    Raises:
        ainari_exceptions.BadRequestException: If the response status code is 400.
        ainari_exceptions.UnauthorizedException: If the response status code is 401.
        ainari_exceptions.NotFoundException: If the response status code is 404.
        ainari_exceptions.ConflictException: If the response status code is 409.
        ainari_exceptions.InternalServerErrorException: If the response status code is 500.
    """
    auth_url = f'{address}/v1alpha/token'
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

    # Retrieve endpoint addresses
    path = "/v1alpha/endpoints"
    resp = ainari_request.send_get_request_without_context(token,
                                                           address,
                                                           path,
                                                           "",
                                                           verify=verify_connection)

    # Extract endpoint addresses from the response
    miko_address = address
    hanami_address = resp["hanami"]["public_address"]
    ryokan_address = resp["ryokan"]["public_address"]
    torii_address = resp["torii"]["public_address"]
    omamori_address = resp["omamori"]["public_address"]

    # Process torii address to extract base address
    torii_address_split = torii_address.split(":")
    torii_base_address = torii_address_split[0] + ":" + torii_address_split[1]

    # Create and return the context object
    context = AccessContext(token, miko_address, hanami_address,
                            ryokan_address, omamori_address, torii_address, torii_base_address)
    return context
