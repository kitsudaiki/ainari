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
from .access_context import AccessContext


def create_project(context: AccessContext,
                   project_id: str,
                   project_name: str) -> dict:
    path = "/v1alpha/project/admin"
    json_body = {
        "id": project_id,
        "name": project_name,
    }
    return ainari_request.send_post_request(context,
                                            context.miko_address,
                                            path,
                                            json_body)


def get_project(context: AccessContext,
                project_id: str) -> dict:
    path = f"/v1alpha/project/{project_id}/admin"
    return ainari_request.send_get_request(context,
                                           context.miko_address,
                                           path,
                                           "")


def list_projects(context: AccessContext) -> dict:
    path = "/v1alpha/project/admin"
    return ainari_request.send_get_request(context,
                                           context.miko_address,
                                           path,
                                           "")


def delete_project(context: AccessContext,
                   project_id: str):
    path = f"/v1alpha/project/{project_id}/admin"
    ainari_request.send_delete_request(context,
                                       context.miko_address,
                                       path,
                                       "")


def list_users_in_project(context: AccessContext) -> dict:
    """
    Returns all users of the project of the access-context, together with the role of each user
    within this project: {"members": [{"user_id": ..., "project_role": ...}]}
    """
    path = "/v1alpha/project/users"
    return ainari_request.send_get_request(context,
                                           context.miko_address,
                                           path,
                                           "")


def list_users_in_project_admin(context: AccessContext,
                                project_id: str) -> dict:
    """
    Returns all users of the given project, together with the role of each user within this
    project: {"members": [{"user_id": ..., "project_role": ...}]}
    """
    path = f"/v1alpha/project/{project_id}/users/admin"
    return ainari_request.send_get_request(context,
                                           context.miko_address,
                                           path,
                                           "")


def add_user_to_project(context: AccessContext,
                        project_id: str,
                        user_id: str,
                        project_role: str) -> dict:
    """
    Adds a user to a project. project_role is one of "admin", "member" or "observer".
    """
    path = f"/v1alpha/project/{project_id}/add_user/admin"
    json_body = {
        "user_id": user_id,
        "project_role": project_role,
    }
    return ainari_request.send_post_request(context,
                                            context.miko_address,
                                            path,
                                            json_body)


def remove_user_from_project(context: AccessContext,
                             project_id: str,
                             user_id: str):
    """
    Removes a user from a project.
    """
    path = f"/v1alpha/project/{project_id}/remove_user/admin"
    json_body = {
        "user_id": user_id,
    }
    ainari_request.send_post_request(context,
                                     context.miko_address,
                                     path,
                                     json_body)


def delete_all_projects(context: AccessContext):
    body = list_projects(context)["projects"]
    for entry in body:
        delete_project(context, entry["id"])
