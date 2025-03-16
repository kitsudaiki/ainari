/**
 * @file        get_checkpoint_v1_0.cpp
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

#include "get_checkpoint_v1_0.h"

#include <database/checkpoint_table.h>
#include <hanami_root.h>

GetCheckpointV1M0::GetCheckpointV1M0() : Blossom("Get checkpoint of a cluster.")
{
    errorCodes.push_back(NOT_FOUND_RTYPE);

    //----------------------------------------------------------------------------------------------
    // input
    //----------------------------------------------------------------------------------------------

    registerInputField("uuid", SAKURA_STRING_TYPE)
        .setComment("UUID of the original request-task, which placed the result in shiori.")
        .setRegex(UUID_REGEX);

    //----------------------------------------------------------------------------------------------
    // output
    //----------------------------------------------------------------------------------------------

    registerOutputField("uuid", SAKURA_STRING_TYPE).setComment("UUID of the dataset.");

    registerOutputField("name", SAKURA_STRING_TYPE).setComment("Name of the dataset.");

    registerOutputField("location", SAKURA_STRING_TYPE).setComment("File path on local storage.");

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
GetCheckpointV1M0::runTask(BlossomIO& blossomIO,
                           const Hanami::UserContext& userContext,
                           BlossomStatus& status,
                           Hanami::ErrorContainer& error)
{
    const std::string checkpointUuid = blossomIO.input["uuid"];

    // get checkpoint-info from database
    const ReturnStatus ret = CheckpointTable::getInstance()->getCheckpoint(
        blossomIO.output, checkpointUuid, userContext, false, error);
    if (ret == ERROR) {
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        return false;
    }
    if (ret == INVALID_INPUT) {
        status.errorMessage = "Checkpoint with uuid '" + checkpointUuid + "' not found";
        status.statusCode = NOT_FOUND_RTYPE;
        LOG_DEBUG(status.errorMessage);
        return false;
    }

    // remove irrelevant fields
    blossomIO.output.erase("owner_id");
    blossomIO.output.erase("project_id");
    blossomIO.output.erase("visibility");

    return true;
}
