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

"""
Assertions of the tests. They raise a CheckFailed, which the runner reports as failure of the
test, while every other exception is reported as error.
"""

from ainari_sdk import ainari_exceptions


class CheckFailed(AssertionError):
    pass


class SkipTest(Exception):
    """
    Raised by a test, which can not run in the current setup, like with a single virtual machine.
    """


def check(condition: bool, message: str):
    if not condition:
        raise CheckFailed(message)


def check_equal(actual, expected, what: str):
    if actual != expected:
        raise CheckFailed(f"{what}: expected '{expected}', got '{actual}'")


def check_in(item, container, what: str):
    if item not in container:
        raise CheckFailed(f"{what}: '{item}' not found in {container}")


def check_not_in(item, container, what: str):
    if item in container:
        raise CheckFailed(f"{what}: '{item}' unexpectedly found")


def check_keys(data: dict, keys, what: str):
    missing = [key for key in keys if key not in data]
    if missing:
        raise CheckFailed(f"{what}: missing fields {missing} in {data}")


def expect_error(exception_type, function, *args, **kwargs):
    """
    Runs the function and checks, that it fails with the given exception of the sdk. A tuple of
    exceptions accepts every one of them.
    """
    expected = exception_type if isinstance(exception_type, tuple) else (exception_type,)
    expected_names = " or ".join(item.__name__ for item in expected)
    try:
        result = function(*args, **kwargs)
    except exception_type as error:
        return error
    except Exception as error:  # noqa: BLE001
        raise CheckFailed(f"expected {expected_names}, "
                          f"got {type(error).__name__}: {error}") from error
    raise CheckFailed(f"expected {expected_names}, but the call succeeded: {result}")


def exists(getter, *args) -> bool:
    """
    Returns False, if the getter reports the resource as not found.
    """
    try:
        getter(*args)
    except ainari_exceptions.NotFoundException:
        return False
    return True
