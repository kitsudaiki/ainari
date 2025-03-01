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


def create_project(token: str,
                   address: str,
                   project_id: str,
                   project_name: str,
                   verify_connection: bool = True) -> dict:
    path = "/v1.0alpha/project"
    json_body = {
        "id": project_id,
        "name": project_name,
    }
    return hanami_request.send_post_request(token,
                                            address,
                                            path,
                                            json_body,
                                            verify=verify_connection)


def get_project(token: str,
                address: str,
                project_id: str,
                verify_connection: bool = True) -> dict:
    path = "/v1.0alpha/project"
    values = f'id={project_id}'
    return hanami_request.send_get_request(token,
                                           address,
                                           path,
                                           values,
                                           verify=verify_connection)


def list_projects(token: str,
                  address: str,
                  verify_connection: bool = True) -> dict:
    path = "/v1.0alpha/project/all"
    return hanami_request.send_get_request(token,
                                           address,
                                           path,
                                           "",
                                           verify=verify_connection)


def delete_project(token: str,
                   address: str,
                   project_id: str,
                   verify_connection: bool = True):
    path = "/v1.0alpha/project"
    values = f'id={project_id}'
    hanami_request.send_delete_request(token,
                                       address,
                                       path,
                                       values,
                                       verify=verify_connection)
