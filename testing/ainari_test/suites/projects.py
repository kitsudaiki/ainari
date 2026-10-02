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
Projects and the roles of the users within them: the default-project of each user, the
assignment of users to further projects, tokens for a specific project and the read-only access
of observers.
"""

import uuid

from ainari_sdk import ainari_exceptions
from ainari_sdk import login
from ainari_sdk import project
from ainari_sdk import secret
from ainari_sdk import user

from ainari_test.checks import check_equal, check_in, check_not_in, exists, expect_error
from ainari_test.framework import Suite

suite = Suite("projects", "default-projects, project-roles and project-assignments")


def project_user_id(ctx) -> str:
    return f"lst-prj-{ctx.test_id}"


def project_id(ctx) -> str:
    # differs from the id of the user, because the cleanup identifies its entries by their id
    return f"lst-project-{ctx.test_id}"


def login_project_user(ctx, project: str = None):
    """
    Requests a token for the user of this suite. Without a project, the default-project is used.
    """
    return login.request_context(ctx.config.miko_address, project_user_id(ctx),
                                 ctx.state["project_user_passphrase"], verify_connection=False,
                                 project_id=project)


def token_context(context) -> dict:
    return login.validate_token(context)["context"]


@suite.test("create user and project", provides=("project_user", "project"))
def create_user_and_project(ctx):
    user_id = project_user_id(ctx)
    passphrase = str(uuid.uuid4())
    user.create_user(ctx.api, user_id, f"local-stack-test {ctx.test_id}", passphrase, False)
    ctx.cleanup.add("user", user_id, user_id,
                    lambda: user.delete_user(ctx.api, user_id),
                    lambda: exists(user.get_user, ctx.api, user_id))
    ctx.state["project_user"] = user_id
    ctx.state["project_user_passphrase"] = passphrase

    # the default-project is created together with the user, but not deleted together with it
    default_project_id = f"default-{user_id}"
    ctx.cleanup.add("project", default_project_id, default_project_id,
                    lambda: project.delete_project(ctx.api, default_project_id),
                    lambda: exists(project.get_project, ctx.api, default_project_id))

    new_project_id = project_id(ctx)
    result = project.create_project(ctx.api, new_project_id, f"local-stack-test {ctx.test_id}")
    ctx.cleanup.add("project", new_project_id, new_project_id,
                    lambda: project.delete_project(ctx.api, new_project_id),
                    lambda: exists(project.get_project, ctx.api, new_project_id))
    check_equal(result["id"], new_project_id, "id of the new project")
    ctx.state["project"] = new_project_id


@suite.test("new user is admin of its default-project", requires=("project_user",))
def default_project(ctx):
    user_id = ctx.state["project_user"]
    default_project_id = f"default-{user_id}"
    check_in(default_project_id,
             [entry["id"] for entry in project.list_projects(ctx.api)["projects"]],
             "default-project of the new user in list")

    # without a project, the token is created for the default-project
    context = token_context(login_project_user(ctx))
    check_equal(context["project_id"], default_project_id, "project of the default token")
    check_equal(context["project_role"], "admin", "role in the default-project")

    # the default-project can also be requested explicitly
    context = token_context(login_project_user(ctx, default_project_id))
    check_equal(context["project_id"], default_project_id, "project of the explicit token")


@suite.test("project-ids with the prefix 'default-' are reserved")
def reserved_prefix(ctx):
    expect_error(ainari_exceptions.BadRequestException, project.create_project, ctx.api,
                 f"default-{ctx.test_id}", "reserved")


@suite.test("token for a not assigned project is rejected", requires=("project_user", "project"))
def token_not_assigned(ctx):
    expect_error(ainari_exceptions.UnauthorizedException, login_project_user, ctx,
                 ctx.state["project"])
    expect_error(ainari_exceptions.UnauthorizedException, login_project_user, ctx,
                 f"lst-unknown-{ctx.test_id}")


@suite.test("assign project", requires=("project_user", "project"), provides=("assigned",))
def assign_project(ctx):
    user_id = ctx.state["project_user"]
    assigned_project = ctx.state["project"]
    result = user.assign_project(ctx.api, user_id, assigned_project, "member")
    check_equal(result["user_id"], user_id, "user of the assignment")
    check_equal(result["project_id"], assigned_project, "project of the assignment")
    check_equal(result["project_role"], "member", "role of the assignment")

    context = token_context(login_project_user(ctx, assigned_project))
    check_equal(context["project_id"], assigned_project, "project of the token")
    check_equal(context["project_role"], "member", "role in the token")
    ctx.state["assigned"] = True


@suite.test("invalid assignments are rejected", requires=("assigned",))
def invalid_assignments(ctx):
    user_id = ctx.state["project_user"]
    assigned_project = ctx.state["project"]
    # a user can be assigned to the same project only once at the same time
    expect_error(ainari_exceptions.ConflictException, user.assign_project, ctx.api, user_id,
                 assigned_project, "admin")
    expect_error(ainari_exceptions.BadRequestException, user.assign_project, ctx.api, user_id,
                 assigned_project, "superuser")
    expect_error(ainari_exceptions.NotFoundException, user.assign_project, ctx.api, user_id,
                 f"lst-unknown-{ctx.test_id}", "member")
    expect_error(ainari_exceptions.NotFoundException, user.assign_project, ctx.api,
                 f"lst-unknown-{ctx.test_id}", assigned_project, "member")
    expect_error(ainari_exceptions.BadRequestException, user.set_project_role, ctx.api,
                 user_id, assigned_project, "superuser")


@suite.test("only admins can manage project-assignments", requires=("assigned",))
def assignments_admin_only(ctx):
    user_id = ctx.state["project_user"]
    assigned_project = ctx.state["project"]
    # even as admin of the project, the user is no admin of the whole system
    own = login_project_user(ctx)
    expect_error(ainari_exceptions.UnauthorizedException, user.assign_project, own, user_id,
                 assigned_project, "admin")
    expect_error(ainari_exceptions.UnauthorizedException, user.set_project_role, own, user_id,
                 assigned_project, "admin")
    expect_error(ainari_exceptions.UnauthorizedException, user.unassign_project, own, user_id,
                 assigned_project)


@suite.test("member can create resources in the project", requires=("assigned",),
            provides=("project_secret",))
def member_creates(ctx):
    member = login_project_user(ctx, ctx.state["project"])
    name = ctx.name("prj-secret")
    result = secret.create_secret(member, name, "test-payload")
    secret_uuid = result["uuid"]
    # deleted by the admin, because the user and its assignment are gone at the cleanup
    ctx.cleanup.add("secret", secret_uuid, name,
                    lambda: secret.delete_secret(ctx.api, secret_uuid),
                    lambda: exists(secret.get_secret, ctx.api, secret_uuid))
    check_equal(result["name"], name, "name of the secret")
    ctx.state["project_secret"] = secret_uuid


@suite.test("set project-role", requires=("assigned",), provides=("observer",))
def set_project_role(ctx):
    user_id = ctx.state["project_user"]
    assigned_project = ctx.state["project"]
    result = user.set_project_role(ctx.api, user_id, assigned_project, "observer")
    check_equal(result["user_id"], user_id, "user of the assignment")
    check_equal(result["project_id"], assigned_project, "project of the assignment")
    check_equal(result["project_role"], "observer", "new role of the assignment")

    context = token_context(login_project_user(ctx, assigned_project))
    check_equal(context["project_role"], "observer", "role in the token")
    ctx.state["observer"] = True


@suite.test("observer can only read", requires=("observer", "project_secret"))
def observer_read_only(ctx):
    observer = login_project_user(ctx, ctx.state["project"])
    secret_uuid = ctx.state["project_secret"]

    # reading is still allowed
    check_equal(secret.get_secret(observer, secret_uuid)["uuid"], secret_uuid,
                "secret read by the observer")
    listed = [entry["uuid"] for entry in secret.list_secrets(observer)["secrets"]]
    check_in(secret_uuid, listed, "secret in the list of the observer")

    # creating is blocked
    name = ctx.name("prj-observer-secret")
    expect_error(ainari_exceptions.ForbiddenException, secret.create_secret, observer, name,
                 "test-payload")
    check_not_in(name, [entry["name"] for entry in secret.list_secrets(ctx.api)["secrets"]],
                 "secret of the observer in list")

    # deleting is blocked as well
    expect_error(ainari_exceptions.ForbiddenException, secret.delete_secret, observer,
                 secret_uuid)
    check_equal(secret.get_secret(ctx.api, secret_uuid)["uuid"], secret_uuid,
                "secret still exists after the delete of the observer")


@suite.test("unassign project", requires=("assigned",))
def unassign_project(ctx):
    user_id = ctx.state["project_user"]
    assigned_project = ctx.state["project"]
    user.unassign_project(ctx.api, user_id, assigned_project)
    del ctx.state["assigned"]

    expect_error(ainari_exceptions.NotFoundException, user.unassign_project, ctx.api, user_id,
                 assigned_project)
    expect_error(ainari_exceptions.NotFoundException, user.set_project_role, ctx.api, user_id,
                 assigned_project, "admin")
    expect_error(ainari_exceptions.UnauthorizedException, login_project_user, ctx,
                 assigned_project)

    # after the unassignment, the user can be assigned again
    result = user.assign_project(ctx.api, user_id, assigned_project, "admin")
    check_equal(result["project_role"], "admin", "role of the new assignment")
    user.unassign_project(ctx.api, user_id, assigned_project)
