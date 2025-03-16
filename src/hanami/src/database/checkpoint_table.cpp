/**
 * @file        checkpoint_table.cpp
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

#include <database/checkpoint_table.h>
#include <hanami_common/functions/string_functions.h>
#include <hanami_common/items/table_item.h>
#include <hanami_database/sql_database.h>

CheckpointTable* CheckpointTable::instance = nullptr;

/**
 * @brief constructor
 *
 * @param db pointer to database
 */
CheckpointTable::CheckpointTable() : HanamiSqlTable(Hanami::SqlDatabase::getInstance())
{
    m_tableName = "checkpoints";

    registerColumn("location", STRING_TYPE).hideValue();
}

/**
 * @brief destructor
 */
CheckpointTable::~CheckpointTable() {}

/**
 * @brief add new metadata of a checkpoint into the database
 *
 * @param checkpointData checkpoint-entry to add to database
 * @param userContext context-object with all user specific information
 * @param error reference for error-output
 *
 * @return OK if found, INVALID_INPUT if conflict, ERROR in case of internal error
 */
ReturnStatus
CheckpointTable::addCheckpoint(const CheckpointDbEntry& checkpointData,
                               const Hanami::UserContext& userContext,
                               Hanami::ErrorContainer& error)
{
    json checkpointDataJson;

    checkpointDataJson["name"] = checkpointData.name;
    checkpointDataJson["uuid"] = checkpointData.uuid;
    checkpointDataJson["visibility"] = checkpointData.visibility;
    checkpointDataJson["location"] = checkpointData.location;

    const ReturnStatus ret = addWithContext(checkpointDataJson, userContext, error);
    if (ret != OK) {
        error.addMessage("Failed to add checkpoint to database");
        return ret;
    }

    return OK;
}

/**
 * @brief get a metadata-entry for a specific checkpoint from the database
 *
 * @param result reference for the result-output
 * @param checkpointUuid uuid of the data
 * @param userContext context-object with all user specific information
 * @param error reference for error-output
 *
 * @return OK if found, INVALID_INPUT if not found, ERROR in case of internal error
 */
ReturnStatus
CheckpointTable::getCheckpoint(CheckpointDbEntry& result,
                               const std::string& checkpointUuid,
                               const Hanami::UserContext& userContext,
                               Hanami::ErrorContainer& error)
{
    json jsonRet;
    const ReturnStatus ret = getCheckpoint(jsonRet, checkpointUuid, userContext, true, error);
    if (ret != OK) {
        return ret;
    }

    result.name = jsonRet["name"];
    result.ownerId = jsonRet["owner_id"];
    result.projectId = jsonRet["project_id"];
    result.uuid = jsonRet["uuid"];
    result.visibility = jsonRet["visibility"];
    result.location = jsonRet["location"];
    result.createdAt = jsonRet["created_at"];
    result.createdBy = jsonRet["created_by"];
    result.updatedAt = jsonRet["updated_at"];
    result.updatedBy = jsonRet["updated_by"];

    return OK;
}

/**
 * @brief get a metadata-entry for a specific checkpoint from the database
 *
 * @param result reference for the result-output
 * @param checkpointUuid uuid of the data
 * @param userContext context-object with all user specific information
 * @param showHiddenValues set to true to also show as hidden marked fields
 * @param error reference for error-output
 *
 * @return OK if found, INVALID_INPUT if not found, ERROR in case of internal error
 */
ReturnStatus
CheckpointTable::getCheckpoint(json& result,
                               const std::string& checkpointUuid,
                               const Hanami::UserContext& userContext,
                               const bool showHiddenValues,
                               Hanami::ErrorContainer& error)
{
    // get user from db
    std::vector<RequestCondition> conditions;
    conditions.emplace_back("uuid", checkpointUuid);

    // get dataset from db
    const ReturnStatus ret
        = getWithContext(result, userContext, conditions, showHiddenValues, error);
    if (ret != OK) {
        error.addMessage("Failed to get checkpoint with UUID '" + checkpointUuid
                         + "' from database");
        return ret;
    }

    return OK;
}

/**
 * @brief get metadata of all checkpoints from the database
 *
 * @param result reference for the result-output
 * @param userContext context-object with all user specific information
 * @param showHiddenValues set to true to also show as hidden marked fields
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
CheckpointTable::getAllCheckpoint(Hanami::TableItem& result,
                                  const Hanami::UserContext& userContext,
                                  const bool showHiddenValues,
                                  Hanami::ErrorContainer& error)
{
    std::vector<RequestCondition> conditions;
    if (getAllWithContext(result, userContext, conditions, error, showHiddenValues) != OK) {
        error.addMessage("Failed to get all checkpoints from database");
        return false;
    }

    return true;
}

/**
 * @brief delete metadata of a checkpoint from the database
 *
 * @param checkpointUuid uuid of the data
 * @param userContext context-object with all user specific information
 * @param error reference for error-output
 *
 * @return OK if found, INVALID_INPUT if not found, ERROR in case of internal error
 */
ReturnStatus
CheckpointTable::deleteCheckpoint(const std::string& checkpointUuid,
                                  const Hanami::UserContext& userContext,
                                  Hanami::ErrorContainer& error)
{
    std::vector<RequestCondition> conditions;
    conditions.emplace_back("uuid", checkpointUuid);
    const ReturnStatus ret = deleteFromDbWithContext(conditions, userContext, error);
    if (ret != OK) {
        error.addMessage("Failed to delete checkpoint with UUID '" + checkpointUuid
                         + "' from database");
        return ret;
    }

    return OK;
}
