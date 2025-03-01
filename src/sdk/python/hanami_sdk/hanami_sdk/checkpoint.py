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


def list_checkpoints(token: str,
                     address: str,
                     verify_connection: bool = True) -> dict:
    path = "/v1.0alpha/checkpoint/all"
    return hanami_request.send_get_request(token,
                                           address,
                                           path,
                                           "",
                                           verify=verify_connection)


def delete_checkpoint(token: str,
                      address: str,
                      checkpoint_uuid: str,
                      verify_connection: bool = True):
    path = "/v1.0alpha/checkpoint"
    values = f'uuid={checkpoint_uuid}'
    hanami_request.send_delete_request(token,
                                       address,
                                       path,
                                       values,
                                       verify=verify_connection)


def delete_all_checkpoints(token: str,
                           address: str,
                           verify_connection: bool = True):
    body = list_checkpoints(token, address, False)["body"]
    for entry in body:
        delete_checkpoint(token, address, entry[1], verify_connection)
