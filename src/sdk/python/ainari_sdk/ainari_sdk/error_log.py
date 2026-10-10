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

from urllib.parse import urlencode

from . import ainari_request
from .access_context import AccessContext


def get_error_log(context: AccessContext,
                  entry_uuid: str) -> dict:
    """
    Returns a single entry of the error-log of omikuji. Only admins are allowed to read it.
    """
    path = f"/v1alpha/error_log/{entry_uuid}/admin"
    return ainari_request.send_get_request(context,
                                           context.omikuji_address,
                                           path,
                                           "")


def list_error_logs(context: AccessContext,
                    project_id: str = None,
                    user_id: str = None,
                    host_name: str = None,
                    limit: int = None) -> dict:
    """
    Returns the entries of the error-log of omikuji, sorted by their creation-time with the newest
    first. Only admins are allowed to read it. Every filter is optional, without a limit omikuji
    returns the newest 100 entries.
    """
    filters = {
        "project_id": project_id,
        "user_id": user_id,
        "host_name": host_name,
        "limit": limit,
    }
    values = urlencode({key: value for key, value in filters.items() if value is not None})
    path = "/v1alpha/error_log/admin"
    return ainari_request.send_get_request(context,
                                           context.omikuji_address,
                                           path,
                                           values)
