/**
 * @file        tempfile_table.cpp
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

#include <database/tempfile_table.h>
#include <hanami_common/functions/string_functions.h>
#include <hanami_common/items/table_item.h>
#include <hanami_database/sql_database.h>

TempfileTable* TempfileTable::instance = nullptr;

/**
 * @brief constructor
 *
 * @param db pointer to database
 */
TempfileTable::TempfileTable() : HanamiSqlTable(Hanami::SqlDatabase::getInstance())
{
    m_tableName = "tempfile";

    registerColumn("related_resource_type", STRING_TYPE).setMaxLength(16);

    registerColumn("related_resource_uuid", STRING_TYPE).setMaxLength(36);

    registerColumn("file_size", INT_TYPE);

    registerColumn("location", STRING_TYPE).hideValue();
}

/**
 * @brief destructor
 */
TempfileTable::~TempfileTable() {}

/**
 * @brief add metadata of a new tempfile to the database
 *
 * @param tempfileData tempfile-entry to add to database
 * @param userContext context-object with all user specific information
 * @param error reference for error-output
 *
 * @return OK if found, INVALID_INPUT if conflict, ERROR in case of internal error
 */
ReturnStatus
TempfileTable::addTempfile(const TempfileDbEntry& tempfileData,
                           const Hanami::UserContext& userContext,
                           Hanami::ErrorContainer& error)
{
    json tempfileDataJson;
    tempfileDataJson["uuid"] = tempfileData.uuid;
    tempfileDataJson["related_resource_type"] = tempfileData.relatedResourceType;
    tempfileDataJson["related_resource_uuid"] = tempfileData.relatedResourceUuid;
    tempfileDataJson["name"] = tempfileData.name;
    tempfileDataJson["file_size"] = tempfileData.fileSize;
    tempfileDataJson["location"] = tempfileData.location;
    tempfileDataJson["visibility"] = tempfileData.visibility;

    const ReturnStatus ret = addWithContext(tempfileDataJson, userContext, error);
    if (ret != OK) {
        error.addMessage("Failed to add tempfile to database");
        return ret;
    }

    return OK;
}

/**
 * @brief get a metadata-entry for a specific tempfile from the database
 *
 * @param result reference for the result-output
 * @param tempfileUuid uuid of the data
 * @param userContext context-object with all user specific information
 * @param error reference for error-output
 *
 * @return OK if found, INVALID_INPUT if not found, ERROR in case of internal error
 */
ReturnStatus
TempfileTable::getTempfile(TempfileDbEntry& result,
                           const std::string& tempfileUuid,
                           const Hanami::UserContext& userContext,
                           Hanami::ErrorContainer& error)
{
    json jsonRet;
    const ReturnStatus ret = getTempfile(jsonRet, tempfileUuid, userContext, true, error);
    if (ret != OK) {
        return ret;
    }

    result.name = jsonRet["name"];
    result.ownerId = jsonRet["owner_id"];
    result.projectId = jsonRet["project_id"];
    result.uuid = jsonRet["uuid"];
    result.visibility = jsonRet["visibility"];
    result.relatedResourceType = jsonRet["related_resource_type"];
    result.relatedResourceUuid = jsonRet["related_resource_uuid"];
    result.fileSize = jsonRet["file_size"];
    result.location = jsonRet["location"];

    return OK;
}

/**
 * @brief get a metadata-entry for a specific tempfile from the database
 *
 * @param result reference for the result-output
 * @param tempfileUuid uuid of the data
 * @param userContext context-object with all user specific information
 * @param showHiddenValues set to true to also show as hidden marked fields
 * @param error reference for error-output
 *
 * @return OK if found, INVALID_INPUT if not found, ERROR in case of internal error
 */
ReturnStatus
TempfileTable::getTempfile(json& result,
                           const std::string& tempfileUuid,
                           const Hanami::UserContext& userContext,
                           const bool showHiddenValues,
                           Hanami::ErrorContainer& error)
{
    // get user from db
    std::vector<RequestCondition> conditions;
    conditions.emplace_back("uuid", tempfileUuid);

    // get dataset from db
    const ReturnStatus ret
        = getWithContext(result, userContext, conditions, showHiddenValues, error);
    if (ret != OK) {
        error.addMessage("Failed to get tempfile with UUID '" + tempfileUuid + "' from database");
        return ret;
    }

    return OK;
}

/**
 * @brief get metadata of all tempfiles from the database
 *
 * @param result reference for the result-output
 * @param userContext context-object with all user specific information
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
TempfileTable::getAllTempfile(Hanami::TableItem& result,
                              const Hanami::UserContext& userContext,
                              Hanami::ErrorContainer& error)
{
    std::vector<RequestCondition> conditions;
    if (getAllWithContext(result, userContext, conditions, error, false) != OK) {
        error.addMessage("Failed to get all tempfiles from database");
        return false;
    }

    return true;
}

/**
 * @brief delete metadata of a tempfile from the database
 *
 * @param tempfileUuid uuid of the data
 * @param userContext context-object with all user specific information
 * @param error reference for error-output
 *
 * @return OK if found, INVALID_INPUT if not found, ERROR in case of internal error
 */
ReturnStatus
TempfileTable::deleteTempfile(const std::string& tempfileUuid,
                              const Hanami::UserContext& userContext,
                              Hanami::ErrorContainer& error)
{
    std::vector<RequestCondition> conditions;
    conditions.emplace_back("uuid", tempfileUuid);

    const ReturnStatus ret = deleteFromDbWithContext(conditions, userContext, error);
    if (ret != OK) {
        error.addMessage("Failed to delete tempfile with UUID '" + tempfileUuid
                         + "' from database");
        return ret;
    }

    return OK;
}

/**
 * @brief get list of all temp-files, which are related to a specific resource
 *
 * @param relatedUuids list of templist-uuids, which are related to the requested resource
 * @param resourceType type of the related resource
 * @param resourceUuid uuid of the related resource
 * @param userContext context-object with all user specific information
 * @param error reference for error-output
 *
 * @return OK if found, INVALID_INPUT if not found, ERROR in case of internal error
 */
ReturnStatus
TempfileTable::getRelatedResourceUuids(std::vector<std::string>& relatedUuids,
                                       const std::string& resourceType,
                                       const std::string& resourceUuid,
                                       const Hanami::UserContext& userContext,
                                       Hanami::ErrorContainer& error)
{
    std::vector<RequestCondition> conditions;
    conditions.emplace_back("related_resource_type", resourceType);
    conditions.emplace_back("related_resource_uuid", resourceUuid);

    // get tempfile from db
    Hanami::TableItem result;
    const ReturnStatus ret = getAllWithContext(result, userContext, conditions, error, true);
    if (ret != OK) {
        error.addMessage("Failed to get related recources for UUID '" + resourceUuid
                         + "' and type '" + resourceType + "' from database");
        return ret;
    }

    for (uint64_t i = 0; i < result.getNumberOfRows(); i++) {
        relatedUuids.push_back(result.getCell(4, i));
    }

    return OK;
}
