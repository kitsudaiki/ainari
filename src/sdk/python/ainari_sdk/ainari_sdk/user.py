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

from . import ainari_request
from . import ainari_exceptions

# import base64


def create_user(token: str,
                address: str,
                user_id: str,
                user_name: str,
                passphrase: str,
                is_admin: bool,
                verify_connection: bool = True) -> dict:
    path = "/v1alpha/user"
    json_body = {
        "id": user_id,
        "name": user_name,
        "passphrase": passphrase,
        "is_admin": is_admin,
    }
    return ainari_request.send_post_request(token,
                                            address,
                                            path,
                                            json_body,
                                            verify=verify_connection)


def get_user(token: str,
             address: str,
             user_id: str,
             verify_connection: bool = True) -> dict:
    path = f'/v1alpha/user/{user_id}'
    return ainari_request.send_get_request(token,
                                           address,
                                           path,
                                           "",
                                           verify=verify_connection)


def list_users(token: str,
               address: str,
               verify_connection: bool = True) -> dict:
    path = "/v1alpha/user"
    return ainari_request.send_get_request(token,
                                           address,
                                           path,
                                           "",
                                           verify=verify_connection)


def delete_user(token: str,
                address: str,
                user_id: str,
                verify_connection: bool = True):
    path = f'/v1alpha/user/{user_id}'
    ainari_request.send_delete_request(token,
                                       address,
                                       path,
                                       "",
                                       verify=verify_connection)


def delete_all_user(token: str,
                    address: str,
                    verify_connection: bool = True):
    body = list_users(token, address, False)["users"]
    for entry in body:
        try:
            delete_user(token, address, entry["id"], verify_connection)
        except ainari_exceptions.ConflictException:
            # when a user tries to delete himself, then an exception
            # is raised, which is catched here.
            pass


def add_roject_to_user(token: str,
                       address: str,
                       user_id: str,
                       project_id: str,
                       role: str,
                       is_project_admin: bool,
                       verify_connection: bool = True) -> dict:
    path = "/v1alpha/user/project"
    json_body = {
        "id": user_id,
        "project_id": project_id,
        "role": role,
        "is_project_admin": is_project_admin,
    }
    return ainari_request.send_post_request(token,
                                            address,
                                            path,
                                            json_body,
                                            verify=verify_connection)


def remove_project_fromUser(token: str,
                            address: str,
                            user_id: str,
                            project_id: str,
                            verify_connection: bool = True):
    path = "/v1alpha/user/project"
    values = f'project_id={project_id}&id={user_id}'
    ainari_request.send_delete_request(token,
                                       address,
                                       path,
                                       values,
                                       verify=verify_connection)


def list_projects_of_user(token: str,
                          address: str,
                          verify_connection: bool = True) -> dict:
    path = "/v1alpha/user/project"
    return ainari_request.send_get_request(token,
                                           address,
                                           path,
                                           "",
                                           verify=verify_connection)


def switch_project(token: str,
                   address: str,
                   project_id: str,
                   verify_connection: bool = True) -> dict:
    path = "/v1alpha/user/project"
    json_body = {
        "project_id": project_id,
    }
    return ainari_request.send_post_request(token,
                                            address,
                                            path,
                                            json_body,
                                            verify=verify_connection)
