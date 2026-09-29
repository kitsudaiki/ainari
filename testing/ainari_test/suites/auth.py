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
Login, tokens, endpoints and the versions of the components of the control-plane.
"""

import dataclasses

from ainari_sdk import ainari_exceptions
from ainari_sdk import common
from ainari_sdk import login
from ainari_sdk import quota

from ainari_test.checks import check, check_equal, check_keys, expect_error
from ainari_test.framework import Suite

suite = Suite("auth", "login, tokens, endpoints and versions")


@suite.test("validate token")
def validate_token(ctx):
    result = login.validate_token(ctx.api)
    check_keys(result, ["context"], "validate-response")
    check_equal(result["context"]["user_id"], ctx.config.user_id, "user of the token")


@suite.test("renew token")
def renew_token(ctx):
    result = login.renew_token(ctx.api)
    check_keys(result, ["access_token"], "renew-response")
    check(len(result["access_token"]) > 0, "renewed token is empty")

    # the renewed token has to be accepted as well
    renewed = dataclasses.replace(ctx.api, token=result["access_token"])
    check_equal(login.validate_token(renewed)["context"]["user_id"], ctx.config.user_id,
                "user of the renewed token")


@suite.test("login with wrong passphrase is rejected")
def wrong_passphrase(ctx):
    expect_error(ainari_exceptions.UnauthorizedException,
                 login.request_context,
                 ctx.config.miko_address,
                 ctx.config.user_id,
                 ctx.config.passphrase + "-wrong",
                 verify_connection=False)


@suite.test("invalid token is rejected")
def invalid_token(ctx):
    broken = dataclasses.replace(ctx.api, token="invalid-token")
    expect_error(ainari_exceptions.UnauthorizedException, login.validate_token, broken)


@suite.test("list endpoints")
def endpoints(ctx):
    result = login.get_endpoints(ctx.api)
    for component in ("hanami", "ryokan", "torii", "omamori"):
        check_keys(result, [component], "endpoints")
        check_keys(result[component], ["public_address", "internal_address"],
                   f"endpoint of {component}")
        ctx.log(f"{component}: {result[component]['public_address']}")


@suite.test("versions of the components")
def versions(ctx):
    addresses = {
        "miko": ctx.api.miko_address,
        "hanami": ctx.api.hanami_address,
        "ryokan": ctx.api.ryokan_adress,
        "omamori": ctx.api.omamori_address,
    }
    for component, address in addresses.items():
        result = common.get_version(ctx.api, address)
        check_keys(result, ["version", "commit_hash"], f"version of {component}")
        ctx.log(f"{component}: {result['version']} ({result['commit_hash']})")


@suite.test("own quota")
def own_quota(ctx):
    result = quota.get_own_quota(ctx.api)
    check_keys(result, ["max_virtual_machine", "max_image", "max_network", "max_floating_ip"],
               "quota")
    check(result["max_virtual_machine"] >= ctx.config.number_of_virtual_machines,
          f"quota allows only {result['max_virtual_machine']} virtual machines, but the test "
          f"creates {ctx.config.number_of_virtual_machines}")
    check(result["max_floating_ip"] >= ctx.config.number_of_virtual_machines,
          f"quota allows only {result['max_floating_ip']} floating ip-addresses")
    ctx.log(f"quota: {result}")
