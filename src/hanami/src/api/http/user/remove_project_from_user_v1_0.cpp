/**
 * @file        remove_project_from_user_v1_0.cpp
 *
 * @author      Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 * @copyright   Apache License Version 2.0
 *
 *      Copyright 2022 Tobias Anker
 *
 *      Licensed under the Apache License, Version 2.0 (the "License");
 *      you may not use this file except in compliance with the License.
 *      You may obtain a copy of the License at
 *
 *          http://www.apache.org/licenses/LICENSE-2.0
 *
 *      Unless required by applicable law or agreed to in writing, software
 *      distributed under the License is distributed on an "AS IS" BASIS,
 *      WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *      See the License for the specific language governing permissions and
 *      limitations under the License.
 */

#include "remove_project_from_user_v1_0.h"

#include <database/users_table.h>
#include <hanami_common/functions/string_functions.h>
#include <hanami_crypto/hashes.h>
#include <hanami_root.h>

/**
 * @brief constructor
 */
RemoveProjectFromUserV1M0::RemoveProjectFromUserV1M0()
    : Blossom("Remove a project from a specific user")
{
    errorCodes.push_back(UNAUTHORIZED_RTYPE);
    errorCodes.push_back(NOT_FOUND_RTYPE);

    //----------------------------------------------------------------------------------------------
    // input
    //----------------------------------------------------------------------------------------------

    registerInputField("id", SAKURA_STRING_TYPE)
        .setComment("ID of the user.")
        .setLimit(4, 254)
        .setRegex(ID_EXT_REGEX);

    registerInputField("project_id", SAKURA_STRING_TYPE)
        .setComment("ID of the project, which has to be removed from the user.")
        .setLimit(4, 254)
        .setRegex(ID_REGEX);

    //----------------------------------------------------------------------------------------------
    // output
    //----------------------------------------------------------------------------------------------

    registerOutputField("id", SAKURA_STRING_TYPE).setComment("ID of the user.");

    registerOutputField("name", SAKURA_STRING_TYPE).setComment("Name of the user.");

    registerOutputField("is_admin", SAKURA_BOOL_TYPE).setComment("True, if user is an admin.");

    registerOutputField("projects", SAKURA_ARRAY_TYPE)
        .setComment(
            "Json-array with all assigned projects "
            "together with role and project-admin-status.");

    registerOutputField("created_at", SAKURA_STRING_TYPE)
        .setComment("Timestamp, when resource was created.");

    registerOutputField("created_by", SAKURA_STRING_TYPE)
        .setComment("ID of the user, who created the resource.");

    registerOutputField("updated_at", SAKURA_STRING_TYPE)
        .setComment("Timestamp, when resource was updated.");

    registerOutputField("updated_by", SAKURA_STRING_TYPE)
        .setComment("ID of the user, who updated the resource.");

    //----------------------------------------------------------------------------------------------
    //
    //----------------------------------------------------------------------------------------------
}

/**
 * @brief runTask
 */
bool
RemoveProjectFromUserV1M0::runTask(BlossomIO& blossomIO,
                                   const Hanami::UserContext& userContext,
                                   BlossomStatus& status,
                                   Hanami::ErrorContainer& error)
{
    // check if admin
    if (userContext.isAdmin == false) {
        status.statusCode = UNAUTHORIZED_RTYPE;
        return false;
    }

    const std::string userId = blossomIO.input["id"];
    const std::string projectId = blossomIO.input["project_id"];
    const std::string creatorId = userContext.userId;

    // check if user already exist within the table
    UserTable::UserDbEntry getResult;
    ReturnStatus ret = UserTable::getInstance()->getUser(getResult, userId, error);
    if (ret == ERROR) {
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        return false;
    }
    if (ret == INVALID_INPUT) {
        status.errorMessage = "User with id '" + userId + "' not found";
        status.statusCode = NOT_FOUND_RTYPE;
        LOG_DEBUG(status.errorMessage);
        return false;
    }

    // check if project is assigned to user and remove it if found
    bool found = false;
    for (uint64_t i = 0; i < getResult.projects.size(); i++) {
        if (getResult.projects[i].projectId == projectId) {
            getResult.projects.erase(getResult.projects.begin() + i);
            found = true;
            break;
        }
    }

    // handle error that project is not assigned to user
    if (found == false) {
        status.errorMessage = "Project with ID '" + projectId
                              + "' is not assigned to user with id '" + userId
                              + "' and so it can not be removed from the user.";
        status.statusCode = NOT_FOUND_RTYPE;
        LOG_DEBUG(status.errorMessage);
        return false;
    }

    // updated projects of user in database
    if (UserTable::getInstance()->updateProjectsOfUser(
            userId, getResult.projects, userContext, error)
        == false)
    {
        error.addMessage("Failed to update projects of user with id '" + userId + "'.");
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        return false;
    }

    // get new created user from database
    if (UserTable::getInstance()->getUser(blossomIO.output, userId, false, error) == false) {
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        return false;
    }

    return true;
}
