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
A second user, which must not see the resources of the user of the test.
"""

import uuid

from ainari_sdk import ainari_exceptions
from ainari_sdk import floating_ip
from ainari_sdk import login
from ainari_sdk import network
from ainari_sdk import user
from ainari_sdk import virtual_machine

from ainari_test.checks import check_equal, check_in, check_not_in, exists, expect_error
from ainari_test.framework import Suite

suite = Suite("users", "second user and isolation between users")


@suite.test("create second user", provides=("second_user",))
def create_user(ctx):
    user_id = f"lst-{ctx.test_id}"
    passphrase = str(uuid.uuid4())
    result = user.create_user(ctx.api, user_id, f"local-stack-test {ctx.test_id}", passphrase,
                              False)
    ctx.cleanup.add("user", user_id, user_id,
                    lambda: user.delete_user(ctx.api, user_id),
                    lambda: exists(user.get_user, ctx.api, user_id))
    check_equal(result["id"], user_id, "id of the new user")
    check_in(user_id, [entry["id"] for entry in user.list_users(ctx.api)["users"]],
             "new user in list")

    second = login.request_context(ctx.config.miko_address, user_id, passphrase,
                                   verify_connection=False)
    check_equal(login.validate_token(second)["context"]["user_id"], user_id,
                "user of the token of the second user")
    ctx.state["second_user"] = second
    ctx.state["second_passphrase"] = passphrase


@suite.test("second user can not use admin-endpoints", requires=("second_user",))
def no_admin(ctx):
    expect_error(ainari_exceptions.UnauthorizedException, user.list_users,
                 ctx.state["second_user"])


@suite.test("second user can not see the virtual machines",
            requires=("second_user", "virtual_machines"))
def isolated_virtual_machines(ctx):
    second = ctx.state["second_user"]
    listed = [entry["uuid"]
              for entry in virtual_machine.list_virtual_machines(second)["virtual_machines"]]
    for entry in ctx.state["virtual_machines"]:
        check_not_in(entry["uuid"], listed, "foreign virtual machine in list")
        expect_error(ainari_exceptions.NotFoundException, virtual_machine.get_virtual_machine,
                     second, entry["uuid"])


@suite.test("second user can not see the network", requires=("second_user", "network"))
def isolated_network(ctx):
    second = ctx.state["second_user"]
    listed = [entry["uuid"] for entry in network.list_networks(second)["networks"]]
    check_not_in(ctx.state["network"], listed, "foreign network in list")
    expect_error(ainari_exceptions.NotFoundException, network.get_network, second,
                 ctx.state["network"])


@suite.test("second user can not take the floating ip-addresses",
            requires=("second_user", "floating_ips"))
def isolated_floating_ips(ctx):
    second = ctx.state["second_user"]
    listed = [entry["uuid"] for entry in floating_ip.list_floating_ips(second)["floating_ips"]]
    for entry in ctx.state["virtual_machines"]:
        check_not_in(entry["floating_ip_uuid"], listed, "foreign floating ip in list")
        expect_error(ainari_exceptions.NotFoundException, floating_ip.detach_floating_ip,
                     second, entry["floating_ip_uuid"])


@suite.test("user can not delete himself")
def delete_own_user(ctx):
    own_user_id = ctx.config.user_id
    expect_error(ainari_exceptions.ConflictException, user.delete_user, ctx.api, own_user_id)
    check_equal(user.get_user(ctx.api, own_user_id)["id"], own_user_id,
                "own user still exists after the rejected delete")


@suite.test("delete second user", requires=("second_user",))
def delete_user(ctx):
    user_id = f"lst-{ctx.test_id}"
    user.delete_user(ctx.api, user_id)
    ctx.cleanup.discard(user_id)
    expect_error(ainari_exceptions.NotFoundException, user.get_user, ctx.api, user_id)
    # an already issued token is invalidated immediately and a new login has to fail as well
    expect_error(ainari_exceptions.UnauthorizedException, network.list_networks,
                 ctx.state["second_user"])
    rejected = (ainari_exceptions.UnauthorizedException, ainari_exceptions.NotFoundException)
    expect_error(rejected, login.request_context, ctx.config.miko_address, user_id,
                 ctx.state["second_passphrase"], verify_connection=False)
