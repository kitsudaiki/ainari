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
from . import ainari_exceptions
from .access_context import AccessContext


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


def set_project_role(context: AccessContext,
                     user_id: str,
                     project_id: str,
                     project_role: str) -> dict:
    """
    Sets the role of a user within a project, to which the user is already assigned.
    project_role is one of "admin", "member" or "observer".
    """
    path = f'/v1alpha/user/{user_id}/set_project_role/admin'
    json_body = {
        "project_id": project_id,
        "project_role": project_role,
    }
    return ainari_request.send_put_request(context,
                                           context.miko_address,
                                           path,
                                           json_body)


def list_user_projects(context: AccessContext) -> dict:
    """
    Returns all projects, to which the user of the access-context is assigned, together with the
    role of the user within each of these projects:
    {"projects": [{"project_id": ..., "project_role": ...}]}
    """
    path = "/v1alpha/user/user_projects"
    return ainari_request.send_get_request(context,
                                           context.miko_address,
                                           path,
                                           "")
