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
from .access_context import AccessContext

# import base64


def create_user(context: AccessContext,
                user_id: str,
                user_name: str,
                passphrase: str,
                is_admin: bool) -> dict:
    path = "/v1alpha/user/admin"
    json_body = {
        "id": user_id,
        "name": user_name,
        "passphrase": passphrase,
        "is_admin": "true" if is_admin else "false",
    }
    return ainari_request.send_post_request(context,
                                            context.miko_address,
                                            path,
                                            json_body)


def get_user(context: AccessContext,
             user_id: str) -> dict:
    path = f'/v1alpha/user/{user_id}/admin'
    return ainari_request.send_get_request(context,
                                           context.miko_address,
                                           path,
                                           "")


def list_users(context: AccessContext) -> dict:
    path = "/v1alpha/user/admin"
    return ainari_request.send_get_request(context,
                                           context.miko_address,
                                           path,
                                           "")


def delete_user(context: AccessContext,
                user_id: str):
    path = f'/v1alpha/user/{user_id}/admin'
    ainari_request.send_delete_request(context,
                                       context.miko_address,
                                       path,
                                       "")


def delete_all_user(context: AccessContext):
    body = list_users(context)["users"]
    for entry in body:
        try:
            delete_user(context, entry["id"])
        except ainari_exceptions.ConflictException:
            # when a user tries to delete himself, then an exception
            # is raised, which is catched here.
            pass


def add_roject_to_user(context: AccessContext,
                       user_id: str,
                       project_id: str,
                       role: str,
                       is_project_admin: bool) -> dict:
    path = "/v1alpha/user/project/admin"
    json_body = {
        "id": user_id,
        "project_id": project_id,
        "role": role,
        "is_project_admin": is_project_admin,
    }
    return ainari_request.send_post_request(context,
                                            context.miko_address,
                                            path,
                                            json_body)


def remove_project_fromUser(context: AccessContext,
                            user_id: str,
                            project_id: str):
    path = "/v1alpha/user/project/admin"
    values = f'project_id={project_id}&id={user_id}'
    ainari_request.send_delete_request(context,
                                       context.miko_address,
                                       path,
                                       values)


def list_projects_of_user(context: AccessContext) -> dict:
    path = "/v1alpha/user/project/admin"
    return ainari_request.send_get_request(context,
                                           context.miko_address,
                                           path,
                                           "")


def switch_project(context: AccessContext,
                   project_id: str) -> dict:
    path = "/v1alpha/user/project/admin"
    json_body = {
        "project_id": project_id,
    }
    return ainari_request.send_post_request(context,
                                            context.miko_address,
                                            path,
                                            json_body)
