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


class NotFoundException(Exception):
    def __init__(self, message):
        self.message = message
        super().__init__(message)


class UnauthorizedException(Exception):
    def __init__(self, message):
        self.message = message
        super().__init__(message)


class BadRequestException(Exception):
    def __init__(self, message):
        self.message = message
        super().__init__(message)


class ConflictException(Exception):
    def __init__(self, message):
        self.message = message
        super().__init__(message)


class InternalServerErrorException(Exception):
    def __init__(self):
        super().__init__("Server internal error appeared.")
