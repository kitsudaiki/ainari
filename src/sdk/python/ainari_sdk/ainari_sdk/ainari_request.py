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
import json
import os
from requests_toolbelt import MultipartEncoder

from . import ainari_exceptions
from .access_context import AccessContext


def _handle_response(response) -> str:
    """
    Handles the HTTP response from a request.

    Args:
        response: The response object from the requests library.

    Returns:
        str: The content of the response as a string if successful.

    Raises:
        ainari_exceptions.BadRequestException: If the response status code is 400.
        ainari_exceptions.UnauthorizedException: If the response status code is 401.
        ainari_exceptions.ForbiddenException: If the response status code is 403.
        ainari_exceptions.NotFoundException: If the response status code is 404.
        ainari_exceptions.ConflictException: If the response status code is 409.
        ainari_exceptions.LockedException: If the response status code is 423.
        ainari_exceptions.InternalServerErrorException: If the response status code is 500.
    """
    if response.status_code >= 200 and response.status_code < 300:
        if len(response.content) == 0:
            return "{}"
        else:
            return response.content
    if response.status_code == 400:
        raise ainari_exceptions.BadRequestException(response.content)
    if response.status_code == 401:
        raise ainari_exceptions.UnauthorizedException(response.content)
    if response.status_code == 403:
        raise ainari_exceptions.ForbiddenException(response.content)
    if response.status_code == 404:
        raise ainari_exceptions.NotFoundException(response.content)
    if response.status_code == 409:
        raise ainari_exceptions.ConflictException(response.content)
    if response.status_code == 423:
        raise ainari_exceptions.LockedException(response.content)
    if response.status_code == 500:
        raise ainari_exceptions.InternalServerErrorException()


def send_post_request(context: AccessContext,
                      address: str,
                      path: str,
                      body: dict) -> dict:
    """
    Sends a POST request to the specified address and path with the given body.

    Args:
        context: The AccessContext object containing authentication and connection settings.
        address: The base URL of the API.
        path: The path to append to the base URL.
        body: The dictionary to be sent as the request body.

    Returns:
        dict: The parsed JSON response from the server.
    """
    body_str = json.dumps(body)
    url = f'{address}{path}'
    bearer_token = "Bearer " + context.token
    headers = {'Authorization': bearer_token,
               'content-type': 'application/json'}
    response = requests.post(url, data=body_str, headers=headers, verify=context.verify_connection)
    return json.loads(_handle_response(response))


def send_get_request(context: AccessContext,
                     address: str,
                     path: str,
                     values: str) -> dict:
    """
    Sends a GET request to the specified address and path with optional query parameters.

    Args:
        context: The AccessContext object containing authentication and connection settings.
        address: The base URL of the API.
        path: The path to append to the base URL.
        values: The query parameters to append to the URL.

    Returns:
        dict: The parsed JSON response from the server.
    """
    if values:
        url = f'{address}{path}?{values}'
    else:
        url = f'{address}{path}'

    bearer_token = "Bearer " + context.token
    headers = {'Authorization': bearer_token}
    response = requests.get(url, headers=headers, verify=context.verify_connection)
    return json.loads(_handle_response(response))


def send_get_request_without_context(token: str,
                                     address: str,
                                     path: str,
                                     values: str,
                                     verify: bool) -> dict:
    """
    Sends a GET request to the specified address and path with optional query parameters,
    without using an AccessContext object.

    Args:
        token: The authentication token to use for the request.
        address: The base URL of the API.
        path: The path to append to the base URL.
        values: The query parameters to append to the URL.
        verify: Whether to verify the SSL certificate.

    Returns:
        dict: The parsed JSON response from the server.
    """
    if values:
        url = f'{address}{path}?{values}'
    else:
        url = f'{address}{path}'

    bearer_token = "Bearer " + token
    headers = {'Authorization': bearer_token}
    response = requests.get(url, headers=headers, verify=verify)
    return json.loads(_handle_response(response))


def send_put_request(context: AccessContext,
                     address: str,
                     path: str,
                     body: dict) -> dict:
    """
    Sends a PUT request to the specified address and path with the given body.

    Args:
        context: The AccessContext object containing authentication and connection settings.
        address: The base URL of the API.
        path: The path to append to the base URL.
        body: The dictionary to be sent as the request body.

    Returns:
        dict: The parsed JSON response from the server.
    """
    body_str = json.dumps(body)
    url = f'{address}{path}'
    bearer_token = "Bearer " + context.token
    headers = {'Authorization': bearer_token,
               'content-type': 'application/json'}
    response = requests.put(url, data=body_str, headers=headers, verify=context.verify_connection)
    return json.loads(_handle_response(response))


def send_delete_request(context: AccessContext,
                        address: str,
                        path: str,
                        values: str):
    """
    Sends a DELETE request to the specified address and path with optional query parameters.

    Args:
        context: The AccessContext object containing authentication and connection settings.
        address: The base URL of the API.
        path: The path to append to the base URL.
        values: The query parameters to append to the URL.
    """
    url = f'{address}{path}?{values}'
    bearer_token = "Bearer " + context.token
    headers = {'Authorization': bearer_token}
    response = requests.delete(url, headers=headers, verify=context.verify_connection)
    _handle_response(response)


def send_delete_request_with_body(context: AccessContext,
                                  address: str,
                                  path: str,
                                  body: dict) -> dict:
    """
    Sends a DELETE request to the specified address and path with the given body. It is used by
    the endpoints, which describe what has to be removed in the body.

    Args:
        context: The AccessContext object containing authentication and connection settings.
        address: The base URL of the API.
        path: The path to append to the base URL.
        body: The dictionary to be sent as the request body.

    Returns:
        dict: The parsed JSON response from the server.
    """
    body_str = json.dumps(body)
    url = f'{address}{path}'
    bearer_token = "Bearer " + context.token
    headers = {'Authorization': bearer_token,
               'content-type': 'application/json'}
    response = requests.delete(url, data=body_str, headers=headers,
                               verify=context.verify_connection)
    return json.loads(_handle_response(response))


def upload_file(context: AccessContext,
                address: str,
                path: str,
                form_fields: dict,
                file_path: str):
    """
    Uploads a file together with additional text-fields as multipart-form.

    Args:
        context: The AccessContext object containing authentication and connection settings.
        address: The base URL of the API.
        path: The path of the endpoint.
        form_fields: Text-fields of the form. Fields with value `None` are not sent.
        file_path: Path of the file, which is uploaded as field `file`.

    Returns:
        dict: The parsed JSON response from the server.

    Raises:
        requests.exceptions.RequestException: If there is an error during the upload.
    """
    url = f'{address}{path}'
    bearer_token = "Bearer " + context.token

    fields = {key: str(value) for key, value in form_fields.items() if value is not None}

    with open(file_path, 'rb') as f:
        fields['file'] = (os.path.basename(file_path), f, 'application/octet-stream')
        encoder = MultipartEncoder(fields=fields)
        headers = {
            'Authorization': bearer_token,
            'Content-Type': encoder.content_type
        }

        # the file is streamed by the encoder, so it has to stay open until the upload is done
        response = requests.post(url, data=encoder, headers=headers,
                                 verify=context.verify_connection)
        return json.loads(_handle_response(response))
