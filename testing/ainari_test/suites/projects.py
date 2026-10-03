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

import dataclasses
import uuid

from ainari_sdk import ainari_exceptions
from ainari_sdk import login
from ainari_sdk import project
from ainari_sdk import quota
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


def user_projects(context) -> dict:
    """
    Returns the projects of the user of the context as mapping from the project-id to the role.
    """
    return {entry["project_id"]: entry["project_role"]
            for entry in user.list_user_projects(context)["projects"]}


def admin_project_members(ctx, project_id: str) -> dict:
    """
    Returns the members of any project, listed by the admin of the test, as mapping from the
    user-id to the role.
    """
    return {entry["user_id"]: entry["project_role"]
            for entry in project.list_users_in_project_admin(ctx.api, project_id)["members"]}


def project_members(context) -> dict:
    """
    Returns the members of the project of the context as mapping from the user-id to the role.
    """
    return {entry["user_id"]: entry["project_role"]
            for entry in project.list_users_in_project(context)["members"]}


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

    # the default-project is created and deleted together with the user, so it is not registered
    # for the cleanup. It can't be deleted directly anyway.

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


@suite.test("each project has its own quota", requires=("project_user", "project"))
def project_quota(ctx):
    project_quota_id = ctx.state["project"]
    result = quota.get_quota(ctx.api, project_quota_id)
    check_equal(result["project_id"], project_quota_id, "project of the quota")
    check_in(project_quota_id,
             [entry["project_id"] for entry in quota.list_quotas(ctx.api)["quotas"]],
             "quota of the new project in list")

    # the default-project of a new user gets a quota as well
    default_project_id = f"default-{ctx.state['project_user']}"
    check_equal(quota.get_quota(ctx.api, default_project_id)["project_id"], default_project_id,
                "project of the quota of the default-project")

    # the quota of the token is the one of the project, for which the token was created
    own = quota.get_own_quota(login_project_user(ctx))
    check_equal(own["project_id"], default_project_id, "project of the own quota")

    # changing the quota of one project doesn't touch the others
    changed = quota.set_quota(ctx.api, project_quota_id, 3, 3, 3, 3, 3)
    check_equal(changed["max_secret"], 3, "changed limit of the quota")
    check_equal(quota.get_quota(ctx.api, default_project_id)["max_secret"], own["max_secret"],
                "limit of the quota of the default-project after the change of another project")


@suite.test("default-project is protected", requires=("project_user",))
def default_project_protected(ctx):
    user_id = ctx.state["project_user"]
    default_project_id = f"default-{user_id}"
    # the default-project is only deleted together with its user
    expect_error(ainari_exceptions.ConflictException, project.delete_project, ctx.api,
                 default_project_id)
    # the user can't lose its own default-project or the possibility to create resources in it
    expect_error(ainari_exceptions.ConflictException, project.remove_user_from_project, ctx.api,
                 default_project_id, user_id)
    expect_error(ainari_exceptions.ConflictException, user.set_project_role, ctx.api, user_id,
                 default_project_id, "observer")
    check_equal(token_context(login_project_user(ctx))["project_role"], "admin",
                "role in the default-project after the rejected changes")


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


@suite.test("add user to project", requires=("project_user", "project"), provides=("assigned",))
def add_user_to_project(ctx):
    user_id = ctx.state["project_user"]
    assigned_project = ctx.state["project"]
    result = project.add_user_to_project(ctx.api, assigned_project, user_id, "member")
    check_equal(result["user_id"], user_id, "user of the assignment")
    check_equal(result["project_id"], assigned_project, "project of the assignment")
    check_equal(result["project_role"], "member", "role of the assignment")

    context = token_context(login_project_user(ctx, assigned_project))
    check_equal(context["project_id"], assigned_project, "project of the token")
    check_equal(context["project_role"], "member", "role in the token")
    ctx.state["assigned"] = True


@suite.test("list invited projects", requires=("assigned",))
def list_user_projects(ctx):
    # the list is the same for every token of the user, no matter for which project it was created
    expected = {f"default-{ctx.state['project_user']}": "admin", ctx.state["project"]: "member"}
    check_equal(user_projects(login_project_user(ctx)), expected,
                "invited projects with the token of the default-project")
    check_equal(user_projects(login_project_user(ctx, ctx.state["project"])), expected,
                "invited projects with the token of the assigned project")

    # the admin of the test is not assigned to the project of this suite
    check_not_in(ctx.state["project"], user_projects(ctx.api),
                 "project of the suite in the invited projects of the admin")


@suite.test("list users in project", requires=("assigned",))
def list_users_in_project(ctx):
    user_id = ctx.state["project_user"]
    # the members are taken from the project of the token
    check_equal(project_members(login_project_user(ctx, ctx.state["project"])),
                {user_id: "member"}, "members of the assigned project")
    check_equal(project_members(login_project_user(ctx)), {user_id: "admin"},
                "members of the default-project")

    # an admin can list the members of any project, also without being assigned to it
    members = {entry["user_id"]: entry["project_role"]
               for entry in project.list_users_in_project_admin(
                   ctx.api, ctx.state["project"])["members"]}
    check_equal(members, {user_id: "member"}, "members of the project listed by the admin")
    expect_error(ainari_exceptions.UnauthorizedException, project.list_users_in_project_admin,
                 login_project_user(ctx), ctx.state["project"])


@suite.test("invalid assignments are rejected", requires=("assigned",))
def invalid_assignments(ctx):
    user_id = ctx.state["project_user"]
    assigned_project = ctx.state["project"]
    # a user can be assigned to the same project only once at the same time
    expect_error(ainari_exceptions.ConflictException, project.add_user_to_project, ctx.api,
                 assigned_project, user_id, "admin")
    expect_error(ainari_exceptions.BadRequestException, project.add_user_to_project, ctx.api,
                 assigned_project, user_id, "superuser")
    expect_error(ainari_exceptions.NotFoundException, project.add_user_to_project, ctx.api,
                 f"lst-unknown-{ctx.test_id}", user_id, "member")
    expect_error(ainari_exceptions.NotFoundException, project.add_user_to_project, ctx.api,
                 assigned_project, f"lst-unknown-{ctx.test_id}", "member")
    expect_error(ainari_exceptions.BadRequestException, user.set_project_role, ctx.api,
                 user_id, assigned_project, "superuser")


@suite.test("only admins can manage project-assignments", requires=("assigned",))
def assignments_admin_only(ctx):
    user_id = ctx.state["project_user"]
    assigned_project = ctx.state["project"]
    # even as admin of the project, the user is no admin of the whole system
    own = login_project_user(ctx)
    expect_error(ainari_exceptions.UnauthorizedException, project.add_user_to_project, own,
                 assigned_project, user_id, "admin")
    expect_error(ainari_exceptions.UnauthorizedException, user.set_project_role, own, user_id,
                 assigned_project, "admin")
    expect_error(ainari_exceptions.UnauthorizedException, project.remove_user_from_project, own,
                 assigned_project, user_id)


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


@suite.test("quota of the project is enforced", requires=("project_secret",))
def project_quota_enforced(ctx):
    # the project contains one secret, so a limit of one secret allows no further secret
    quota.set_quota(ctx.api, ctx.state["project"], 3, 3, 1, 3, 3)
    member = login_project_user(ctx, ctx.state["project"])
    expect_error(ainari_exceptions.ConflictException, secret.create_secret, member,
                 ctx.name("prj-secret-over-quota"), "test-payload")

    # the quota of the default-project of the same user is not affected
    own = login_project_user(ctx)
    name = ctx.name("prj-default-secret")
    result = secret.create_secret(own, name, "test-payload")
    secret_uuid = result["uuid"]
    ctx.cleanup.add("secret", secret_uuid, name,
                    lambda: secret.delete_secret(ctx.api, secret_uuid),
                    lambda: exists(secret.get_secret, ctx.api, secret_uuid))

    # the following tests create resources in the project again
    quota.set_quota(ctx.api, ctx.state["project"], 3, 3, 3, 3, 3)


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
    check_equal(user_projects(login_project_user(ctx)).get(assigned_project), "observer",
                "role in the list of invited projects")
    check_equal(project_members(login_project_user(ctx, assigned_project)).get(user_id),
                "observer", "role in the list of members of the project")
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


@suite.test("remove user from project", requires=("assigned",))
def remove_user_from_project(ctx):
    user_id = ctx.state["project_user"]
    assigned_project = ctx.state["project"]
    project.remove_user_from_project(ctx.api, assigned_project, user_id)
    del ctx.state["assigned"]

    expect_error(ainari_exceptions.NotFoundException, project.remove_user_from_project, ctx.api,
                 assigned_project, user_id)
    expect_error(ainari_exceptions.NotFoundException, user.set_project_role, ctx.api, user_id,
                 assigned_project, "admin")
    expect_error(ainari_exceptions.UnauthorizedException, login_project_user, ctx,
                 assigned_project)
    check_equal(list(user_projects(login_project_user(ctx))),
                [f"default-{user_id}"], "invited projects after the unassignment")

    # after the unassignment, the user can be assigned again
    result = project.add_user_to_project(ctx.api, assigned_project, user_id, "admin")
    check_equal(result["project_role"], "admin", "role of the new assignment")
    project.remove_user_from_project(ctx.api, assigned_project, user_id)


@suite.test("project with resources can not be deleted", requires=("project_secret",))
def delete_project_with_resources(ctx):
    project_with_secret = ctx.state["project"]
    expect_error(ainari_exceptions.ConflictException, project.delete_project, ctx.api,
                 project_with_secret)
    check_equal(project.get_project(ctx.api, project_with_secret)["id"], project_with_secret,
                "project still exists after the rejected delete")


@suite.test("project with admin or member can not be deleted", requires=("project_user",))
def delete_project_with_users(ctx):
    user_id = ctx.state["project_user"]
    new_project_id = f"lst-project-usr-{ctx.test_id}"
    project.create_project(ctx.api, new_project_id, f"local-stack-test {ctx.test_id}")
    ctx.cleanup.add("project", new_project_id, new_project_id,
                    lambda: project.delete_project(ctx.api, new_project_id),
                    lambda: exists(project.get_project, ctx.api, new_project_id))

    # a member could create a resource at the same time, so the empty project is still blocked
    project.add_user_to_project(ctx.api, new_project_id, user_id, "member")
    expect_error(ainari_exceptions.ConflictException, project.delete_project, ctx.api,
                 new_project_id)
    check_equal(project.get_project(ctx.api, new_project_id)["id"], new_project_id,
                "project still exists after the rejected delete")

    # an observer can't create resources, so it doesn't block the delete and is removed with it
    user.set_project_role(ctx.api, user_id, new_project_id, "observer")
    project.delete_project(ctx.api, new_project_id)
    expect_error(ainari_exceptions.NotFoundException, project.get_project, ctx.api,
                 new_project_id)
    check_not_in(new_project_id, user_projects(login_project_user(ctx)),
                 "deleted project in the invited projects of the observer")


@suite.test("user-delete only checks and deletes the default-project", requires=("project_user",))
def delete_user_with_projects(ctx):
    user_id = f"lst-prj-del-{ctx.test_id}"
    passphrase = str(uuid.uuid4())
    user.create_user(ctx.api, user_id, f"local-stack-test {ctx.test_id}", passphrase, False)
    ctx.cleanup.add("user", user_id, user_id,
                    lambda: user.delete_user(ctx.api, user_id),
                    lambda: exists(user.get_user, ctx.api, user_id))
    default_project_id = f"default-{user_id}"

    # another member doesn't keep the default-project, it is always deleted together with its user
    project.add_user_to_project(ctx.api, default_project_id, ctx.state["project_user"], "member")

    # another project, in which the user is the only admin. Its resources don't block the delete
    # and the project is not deleted together with the user.
    other_project_id = f"lst-project-del-{ctx.test_id}"
    project.create_project(ctx.api, other_project_id, f"local-stack-test {ctx.test_id}")
    ctx.cleanup.add("project", other_project_id, other_project_id,
                    lambda: project.delete_project(ctx.api, other_project_id),
                    lambda: exists(project.get_project, ctx.api, other_project_id))
    project.add_user_to_project(ctx.api, other_project_id, user_id, "admin")
    other = login.request_context(ctx.config.miko_address, user_id, passphrase,
                                  verify_connection=False, project_id=other_project_id)
    other_name = ctx.name("prj-del-other-secret")
    other_secret_uuid = secret.create_secret(other, other_name, "test-payload")["uuid"]
    ctx.cleanup.add("secret", other_secret_uuid, other_name,
                    lambda: secret.delete_secret(ctx.api, other_secret_uuid),
                    lambda: exists(secret.get_secret, ctx.api, other_secret_uuid))

    # a resource in the default-project blocks the delete
    own = login.request_context(ctx.config.miko_address, user_id, passphrase,
                                verify_connection=False)
    name = ctx.name("prj-del-secret")
    secret_uuid = secret.create_secret(own, name, "test-payload")["uuid"]
    ctx.cleanup.add("secret", secret_uuid, name,
                    lambda: secret.delete_secret(ctx.api, secret_uuid),
                    lambda: exists(secret.get_secret, ctx.api, secret_uuid))
    expect_error(ainari_exceptions.ConflictException, user.delete_user, ctx.api, user_id)
    check_equal(user.get_user(ctx.api, user_id)["id"], user_id,
                "user still exists after the rejected delete")
    check_equal(project.get_project(ctx.api, default_project_id)["id"], default_project_id,
                "default-project still exists after the rejected delete")
    check_equal(admin_project_members(ctx, default_project_id),
                {user_id: "admin", ctx.state["project_user"]: "member"},
                "roles in the default-project are restored after the rejected delete")

    # without resources in the default-project, the user is deleted together with it
    secret.delete_secret(ctx.api, secret_uuid)
    user.delete_user(ctx.api, user_id)
    expect_error(ainari_exceptions.NotFoundException, user.get_user, ctx.api, user_id)
    expect_error(ainari_exceptions.NotFoundException, project.get_project, ctx.api,
                 default_project_id)
    expect_error(ainari_exceptions.NotFoundException, quota.get_quota, ctx.api,
                 default_project_id)

    # the user is removed from the other project, but the project and its resources stay
    check_equal(project.get_project(ctx.api, other_project_id)["id"], other_project_id,
                "other project still exists after the user-delete")
    check_equal(admin_project_members(ctx, other_project_id), {},
                "members of the other project after the user-delete")
    check_equal(secret.get_secret(ctx.api, other_secret_uuid)["uuid"], other_secret_uuid,
                "resource of the other project still exists after the user-delete")

    # the tokens of the deleted user are not valid anymore
    expect_error(ainari_exceptions.UnauthorizedException, secret.list_secrets, other)


@suite.test("tokens are invalidated by changes of the user or project", requires=("project_user",))
def token_invalidation(ctx):
    user_id = ctx.state["project_user"]
    token_project_id = f"lst-project-tkn-{ctx.test_id}"
    project.create_project(ctx.api, token_project_id, f"local-stack-test {ctx.test_id}")
    ctx.cleanup.add("project", token_project_id, token_project_id,
                    lambda: project.delete_project(ctx.api, token_project_id),
                    lambda: exists(project.get_project, ctx.api, token_project_id))
    project.add_user_to_project(ctx.api, token_project_id, user_id, "member")
    member = login_project_user(ctx, token_project_id)
    secret.list_secrets(member)

    # a changed role invalidates the token, but it can still be renewed to get the new role
    user.set_project_role(ctx.api, user_id, token_project_id, "observer")
    expect_error(ainari_exceptions.UnauthorizedException, secret.list_secrets, member)
    observer = dataclasses.replace(member, token=login.renew_token(member)["access_token"])
    check_equal(token_context(observer)["project_role"], "observer", "role in the renewed token")
    secret.list_secrets(observer)

    # removing the user from the project invalidates the token as well, also for a renewal
    project.remove_user_from_project(ctx.api, token_project_id, user_id)
    expect_error(ainari_exceptions.UnauthorizedException, secret.list_secrets, observer)
    expect_error(ainari_exceptions.UnauthorizedException, login.renew_token, observer)

    # the token of the default-project is not affected by the changes of the other project
    secret.list_secrets(login_project_user(ctx))
