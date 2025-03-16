/**
 * @file        delete_project_v1_0.cpp
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

#include "delete_project_v1_0.h"

#include <database/projects_table.h>
#include <hanami_root.h>

/**
 * @brief constructor
 */
DeleteProjectV1M0::DeleteProjectV1M0() : Blossom("Delete a specific user from the database.")
{
    errorCodes.push_back(UNAUTHORIZED_RTYPE);
    errorCodes.push_back(NOT_FOUND_RTYPE);

    //----------------------------------------------------------------------------------------------
    // input
    //----------------------------------------------------------------------------------------------

    registerInputField("id", SAKURA_STRING_TYPE)
        .setComment("ID of the project.")
        .setLimit(4, 254)
        .setRegex(ID_REGEX);

    //----------------------------------------------------------------------------------------------
    //
    //----------------------------------------------------------------------------------------------
}

/**
 * @brief runTask
 */
bool
DeleteProjectV1M0::runTask(BlossomIO& blossomIO,
                           const Hanami::UserContext& userContext,
                           BlossomStatus& status,
                           Hanami::ErrorContainer& error)
{
    // check if admin
    if (userContext.isAdmin == false) {
        status.statusCode = UNAUTHORIZED_RTYPE;
        return false;
    }

    // get information from request
    const std::string projectId = blossomIO.input["id"];

    // delete data from table
    const ReturnStatus ret
        = ProjectTable::getInstance()->deleteProject(projectId, userContext, error);
    if (ret == INVALID_INPUT) {
        status.errorMessage = "Project with id '" + projectId + "' not found";
        status.statusCode = NOT_FOUND_RTYPE;
        LOG_DEBUG(status.errorMessage);
        return false;
    }
    if (ret == ERROR) {
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        return false;
    }

    return true;
}
