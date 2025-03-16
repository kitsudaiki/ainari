/**
 * @file       hanami_sql_table.cpp
 *
 * @author     Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 * @copyright  Apache License Version 2.0
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

#include "hanami_sql_table.h"

#include <hanami_common/functions/string_functions.h>
#include <hanami_database/sql_database.h>
#include <uuid/uuid.h>

/**
 * @brief constructor, which add basic columns to the table
 *
 * @param db pointer to database
 */
HanamiSqlTable::HanamiSqlTable(Hanami::SqlDatabase* db) : SqlTable(db)
{
    registerColumn("uuid", STRING_TYPE).setMaxLength(36).setIsPrimary();

    registerColumn("project_id", STRING_TYPE).setMaxLength(256);

    registerColumn("owner_id", STRING_TYPE).setMaxLength(256);

    registerColumn("visibility", STRING_TYPE).setMaxLength(10);

    registerColumn("name", STRING_TYPE).setMaxLength(256);
}

/**
 * @brief destructor
 */
HanamiSqlTable::~HanamiSqlTable() {}

/**
 * @brief add a new row to the table
 *
 * @param values json-item with key-value-pairs, which should be added as new row to the table
 * @param userContext context-object with all user specific information
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
ReturnStatus
HanamiSqlTable::addWithContext(json& values,
                               const Hanami::UserContext& userContext,
                               Hanami::ErrorContainer& error)
{
    // generate new uuid if the is no predefined
    if (values.contains("uuid") == false) {
        values["uuid"] = generateUuid().toString();
    }
    else {
        const std::string uuid = values["uuid"];
        if (uuid == "") {
            values["uuid"] = generateUuid().toString();
        }
    }

    // add user-ids
    values["owner_id"] = userContext.userId;
    values["project_id"] = userContext.projectId;

    const ReturnStatus ret = doesUuidAlreadyExist(values["uuid"], userContext, error);
    if (ret == OK) {
        return INVALID_INPUT;
    }
    if (ret == ERROR) {
        return ERROR;
    }

    if (insertToDb(values, userContext.userId, error) == false) {
        return ERROR;
    }

    return OK;
}

/**
 * @brief get specific values for the table
 *
 * @param result reference for result-output
 * @param userContext context-object with all user specific information
 * @param conditions list of conditions to filter result
 * @param error reference for error-output
 * @param showHiddenValues true to also return as hidden marked values
 *
 * @return true, if successful, else false
 */
ReturnStatus
HanamiSqlTable::getWithContext(json& result,
                               const Hanami::UserContext& userContext,
                               std::vector<RequestCondition>& conditions,
                               const bool showHiddenValues,
                               Hanami::ErrorContainer& error)
{
    fillCondition(conditions, userContext);
    return getFromDb(result, conditions, showHiddenValues, true, error);
}

/**
 * @brief update specific values for the table
 *
 * @param values key-values-pairs to update
 * @param userContext context-object with all user specific information
 * @param conditions list of conditions to filter result
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
ReturnStatus
HanamiSqlTable::updateWithContext(json& values,
                                  const Hanami::UserContext& userContext,
                                  std::vector<RequestCondition>& conditions,
                                  Hanami::ErrorContainer& error)
{
    fillCondition(conditions, userContext);
    return updateInDb(conditions, values, userContext.userId, error);
}

/**
 * @brief get all entries of the table
 *
 * @param result reference for result-output
 * @param userContext context-object with all user specific information
 * @param conditions predefined list of conditions for filtering
 * @param error reference for error-output
 * @param showHiddenValues true to also return as hidden marked values
 *
 * @return true, if successful, else false
 */
ReturnStatus
HanamiSqlTable::getAllWithContext(Hanami::TableItem& result,
                                  const Hanami::UserContext& userContext,
                                  std::vector<RequestCondition>& conditions,
                                  Hanami::ErrorContainer& error,
                                  const bool showHiddenValues)
{
    fillCondition(conditions, userContext);
    return getFromDb(result, conditions, showHiddenValues, false, error);
}

/**
 * @brief HanamiSqlTable::del
 *
 * @param conditions list of conditions to filter result
 * @param userContext context-object with all user specific information
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
ReturnStatus
HanamiSqlTable::deleteFromDbWithContext(std::vector<RequestCondition>& conditions,
                                        const Hanami::UserContext& userContext,
                                        Hanami::ErrorContainer& error)
{
    fillCondition(conditions, userContext);
    return deleteFromDb(conditions, userContext.userId, error);
}

/**
 * @brief check if a specific name already exist within the table
 *
 * @param name name to check
 * @param userContext context-object with all user specific information
 * @param error reference for error-output
 *
 * @return true, if name is already in use, else false
 */
ReturnStatus
HanamiSqlTable::doesNameAlreadyExist(const std::string& name,
                                     const Hanami::UserContext& userContext,
                                     Hanami::ErrorContainer& error)
{
    json result;
    std::vector<RequestCondition> conditions;
    conditions.emplace_back("name", name);

    // get user from db
    const ReturnStatus ret = getWithContext(result, userContext, conditions, false, error);
    if (ret != OK) {
        return ret;
    }

    if (result.size() != 0) {
        return OK;
    }

    return INVALID_INPUT;
}

/**
 * @brief HanamiSqlTable::doesIdAlreadyExist
 * @param uuid
 * @param userContext
 * @param error
 * @return
 */
ReturnStatus
HanamiSqlTable::doesUuidAlreadyExist(const std::string& uuid,
                                     const Hanami::UserContext& userContext,
                                     Hanami::ErrorContainer& error)
{
    json result;
    std::vector<RequestCondition> conditions;
    conditions.emplace_back("uuid", uuid);

    // get user from db
    const ReturnStatus ret = getWithContext(result, userContext, conditions, false, error);
    if (ret != OK) {
        return ret;
    }

    if (result.size() != 0) {
        return OK;
    }

    return INVALID_INPUT;
}

/**
 * @brief update list of conditions based on admin-status
 *
 * @param conditions list of conditions to filter result
 * @param userContext context-object with all user specific information
 */
void
HanamiSqlTable::fillCondition(std::vector<RequestCondition>& conditions,
                              const Hanami::UserContext& userContext)
{
    if (userContext.isAdmin) {
        return;
    }

    if (userContext.isProjectAdmin) {
        conditions.emplace_back("project_id", userContext.projectId);
        return;
    }

    conditions.emplace_back("owner_id", userContext.userId);
    conditions.emplace_back("project_id", userContext.projectId);

    return;
}
