/**
 * @file        list_checkpoint_v1_0.cpp
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

#include "list_checkpoint_v1_0.h"

#include <database/checkpoint_table.h>
#include <hanami_root.h>

ListCheckpointV1M0::ListCheckpointV1M0() : Blossom("List checkpoints of all visible cluster.")
{
    //----------------------------------------------------------------------------------------------
    // output
    //----------------------------------------------------------------------------------------------

    json headerMatch = json::array();
    headerMatch.push_back("created_at");
    headerMatch.push_back("created_by");
    headerMatch.push_back("updated_at");
    headerMatch.push_back("updated_by");
    headerMatch.push_back("uuid");
    headerMatch.push_back("project_id");
    headerMatch.push_back("owner_id");
    headerMatch.push_back("visibility");
    headerMatch.push_back("name");

    registerOutputField("header", SAKURA_ARRAY_TYPE)
        .setComment("Array with the namings all columns of the table.")
        .setMatch(headerMatch);

    registerOutputField("body", SAKURA_ARRAY_TYPE)
        .setComment("Array with all rows of the table, which array arrays too.");

    //----------------------------------------------------------------------------------------------
    //
    //----------------------------------------------------------------------------------------------
}

/**
 * @brief runTask
 */
bool
ListCheckpointV1M0::runTask(BlossomIO& blossomIO,
                            const Hanami::UserContext& userContext,
                            BlossomStatus& status,
                            Hanami::ErrorContainer& error)
{
    // get data from table
    Hanami::TableItem table;
    if (CheckpointTable::getInstance()->getAllCheckpoint(table, userContext, false, error) == false)
    {
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        return false;
    }

    // create output
    blossomIO.output["header"] = table.getInnerHeader();
    blossomIO.output["body"] = table.getBody();

    return true;
}
