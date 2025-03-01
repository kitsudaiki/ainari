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


def list_audit_logs(token: str,
                    address: str,
                    user_id: str,
                    page: int,
                    verify_connection: bool = True) -> dict:
    path = "/v1.0alpha/audit_log"
    values = f'user_id={user_id}&page={page}'
    return hanami_request.send_get_request(token,
                                           address,
                                           path,
                                           values,
                                           verify=verify_connection)


def list_error_logs(token: str,
                    address: str,
                    user_id: str,
                    page: int,
                    verify_connection: bool = True) -> dict:
    path = "/v1.0alpha/error_log"
    if user_id:
        values = f'user_id={user_id}&page={page}'
    else:
        values = f'page={page}'
    return hanami_request.send_get_request(token,
                                           address,
                                           path,
                                           values,
                                           verify=verify_connection)
