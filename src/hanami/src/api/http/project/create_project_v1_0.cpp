/**
 * @file        create_project_v1_0.cpp
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

#include "create_project_v1_0.h"

#include <database/projects_table.h>
#include <hanami_common/functions/string_functions.h>
#include <hanami_crypto/hashes.h>
#include <hanami_root.h>

/**
 * @brief constructor
 */
CreateProjectV1M0::CreateProjectV1M0() : Blossom("Register a new project within Misaki.")
{
    errorCodes.push_back(UNAUTHORIZED_RTYPE);
    errorCodes.push_back(CONFLICT_RTYPE);

    //----------------------------------------------------------------------------------------------
    // input
    //----------------------------------------------------------------------------------------------

    registerInputField("id", SAKURA_STRING_TYPE)
        .setComment("ID of the new project.")
        .setLimit(4, 254)
        .setRegex(ID_REGEX);

    registerInputField("name", SAKURA_STRING_TYPE)
        .setComment("Name of the new project.")
        .setLimit(4, 254)
        .setRegex(NAME_REGEX);

    //----------------------------------------------------------------------------------------------
    // output
    //----------------------------------------------------------------------------------------------

    registerOutputField("id", SAKURA_STRING_TYPE).setComment("ID of the new project.");

    registerOutputField("name", SAKURA_STRING_TYPE).setComment("Name of the new project.");

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
CreateProjectV1M0::runTask(BlossomIO& blossomIO,
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
    const std::string projectName = blossomIO.input["name"];
    const std::string creatorId = userContext.userId;

    // convert values
    ProjectTable::ProjectDbEntry dbEntry;
    dbEntry.id = projectId;
    dbEntry.name = projectName;

    // add new project to table
    const ReturnStatus ret = ProjectTable::getInstance()->addProject(dbEntry, userContext, error);
    if (ret == INVALID_INPUT) {
        status.errorMessage = "Project with id '" + projectId + "' already exist.";
        status.statusCode = CONFLICT_RTYPE;
        LOG_DEBUG(status.errorMessage);
        return false;
    }
    if (ret == ERROR) {
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        return false;
    }

    // get new created user from database
    if (ProjectTable::getInstance()->getProject(blossomIO.output, projectId, false, error) != OK) {
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        return false;
    }

    return true;
}
