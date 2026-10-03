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
from ainari_sdk import vm_type

from ainari_test.checks import check_equal, check_in, check_not_in, exists, expect_error
from ainari_test.framework import Suite

suite = Suite("users", "second user, isolation between users and passphrase-changes")


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


@suite.test("create temporary passphrase user", provides=("passphrase_user",))
def create_passphrase_user(ctx):
    # the passphrase-tests use their own temporary user, so the passphrase of the user, which runs
    # the tests, is never touched, also not by a broken permission-check
    user_id = f"lst-pw-{ctx.test_id}"
    passphrase = str(uuid.uuid4())
    user.create_user(ctx.api, user_id, f"local-stack-test passphrase {ctx.test_id}", passphrase,
                     False)
    ctx.cleanup.add("user", user_id, user_id,
                    lambda: user.delete_user(ctx.api, user_id),
                    lambda: exists(user.get_user, ctx.api, user_id))
    ctx.state["passphrase_user"] = login.request_context(ctx.config.miko_address, user_id,
                                                         passphrase, verify_connection=False)
    ctx.state["passphrase_user_passphrase"] = passphrase


@suite.test("user changes own passphrase", requires=("passphrase_user",))
def change_own_passphrase(ctx):
    user_id = f"lst-pw-{ctx.test_id}"
    pw_user = ctx.state["passphrase_user"]
    old_passphrase = ctx.state["passphrase_user_passphrase"]
    new_passphrase = str(uuid.uuid4())

    expect_error(ainari_exceptions.UnauthorizedException, user.change_passphrase, pw_user,
                 old_passphrase + "-wrong", new_passphrase)
    expect_error(ainari_exceptions.BadRequestException, user.change_passphrase, pw_user,
                 old_passphrase, "short")

    user.change_passphrase(pw_user, old_passphrase, new_passphrase)
    ctx.state["passphrase_user_passphrase"] = new_passphrase

    # the existing token stays valid, but only the new passphrase is accepted for a new login
    check_equal(login.validate_token(pw_user)["context"]["user_id"], user_id,
                "user of the token after the passphrase-change")
    expect_error(ainari_exceptions.UnauthorizedException, login.request_context,
                 ctx.config.miko_address, user_id, old_passphrase, verify_connection=False)
    renewed = login.request_context(ctx.config.miko_address, user_id, new_passphrase,
                                    verify_connection=False)
    check_equal(login.validate_token(renewed)["context"]["user_id"], user_id,
                "user of the token with the new passphrase")


@suite.test("admin changes passphrase of user", requires=("passphrase_user",))
def change_passphrase_admin(ctx):
    user_id = f"lst-pw-{ctx.test_id}"
    pw_user = ctx.state["passphrase_user"]
    old_passphrase = ctx.state["passphrase_user_passphrase"]
    new_passphrase = str(uuid.uuid4())

    # only admins can use the admin-endpoint. The temporary user is the target here as well, so a
    # broken check can not change the passphrase of any other user.
    expect_error(ainari_exceptions.UnauthorizedException, user.change_passphrase_admin, pw_user,
                 user_id, new_passphrase)
    expect_error(ainari_exceptions.NotFoundException, user.change_passphrase_admin, ctx.api,
                 f"unknown-{ctx.test_id}", new_passphrase)

    user.change_passphrase_admin(ctx.api, user_id, new_passphrase)
    ctx.state["passphrase_user_passphrase"] = new_passphrase

    expect_error(ainari_exceptions.UnauthorizedException, login.request_context,
                 ctx.config.miko_address, user_id, old_passphrase, verify_connection=False)
    renewed = login.request_context(ctx.config.miko_address, user_id, new_passphrase,
                                    verify_connection=False)
    check_equal(login.validate_token(renewed)["context"]["user_id"], user_id,
                "user of the token with the passphrase set by the admin")


@suite.test("delete temporary passphrase user", requires=("passphrase_user",))
def delete_passphrase_user(ctx):
    user_id = f"lst-pw-{ctx.test_id}"
    user.delete_user(ctx.api, user_id)
    ctx.cleanup.discard(user_id)
    expect_error(ainari_exceptions.NotFoundException, user.get_user, ctx.api, user_id)


@suite.test("second user can use, but not change the vm-type",
            requires=("second_user", "vm_type"))
def shared_vm_type(ctx):
    # vm-types are global, so every user sees them, but only admins can change them
    second = ctx.state["second_user"]
    vm_type_uuid = ctx.state["vm_type"]
    listed = [entry["uuid"] for entry in vm_type.list_vm_types(second)["vm_types"]]
    check_in(vm_type_uuid, listed, "vm-type in list of the second user")
    check_equal(vm_type.get_vm_type(second, vm_type_uuid)["uuid"], vm_type_uuid,
                "vm-type of the second user")

    expect_error(ainari_exceptions.UnauthorizedException, vm_type.create_vm_type, second,
                 ctx.name("foreign-type"), 1, 512)
    expect_error(ainari_exceptions.UnauthorizedException, vm_type.update_vm_type, second,
                 vm_type_uuid, number_of_cores=1)
    expect_error(ainari_exceptions.UnauthorizedException, vm_type.delete_vm_type, second,
                 vm_type_uuid)


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
