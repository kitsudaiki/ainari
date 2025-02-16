/**
 * @file        test_table.cpp
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

#include "test_table.h"

#include <hanami_database/sql_database.h>
#include <hanami_database/sql_table.h>

namespace Hanami
{

TestTable::TestTable(Hanami::SqlDatabase* db) : SqlTable(db)
{
    m_tableName = "users";

    registerColumn("name", STRING_TYPE).setMaxLength(256);
    registerColumn("pw_hash", STRING_TYPE).setMaxLength(64).hideValue();
    registerColumn("is_admin", BOOL_TYPE);
}

TestTable::~TestTable() {}

/**
 * @brief addUser
 */
bool
TestTable::addUser(json& data, ErrorContainer& error)
{
    return insertToDb(data, error);
}

/**
 * @brief getUser
 */
ReturnStatus
TestTable::getUser(TableItem& resultTable,
                   const std::string& userID,
                   ErrorContainer& error,
                   const bool withHideValues)
{
    std::vector<RequestCondition> conditions;
    conditions.emplace_back("name", userID);
    return getFromDb(resultTable, conditions, withHideValues, true, error);
}

/**
 * @brief getUser
 */
ReturnStatus
TestTable::getUser(json& resultItem,
                   const std::string& userID,
                   const bool withHideValues,
                   ErrorContainer& error)
{
    std::vector<RequestCondition> conditions;
    conditions.emplace_back("name", userID);
    return getFromDb(resultItem, conditions, withHideValues, true, error);
}

/**
 * @brief updateUser
 */
ReturnStatus
TestTable::updateUser(const std::string& userID, const json& values, ErrorContainer& error)
{
    std::vector<RequestCondition> conditions;
    conditions.emplace_back("name", userID);
    return updateInDb(conditions, values, error);
}

/**
 * @brief getNumberOfUsers
 */
long
TestTable::getNumberOfUsers(ErrorContainer& error)
{
    return getNumberOfRows(error);
}

/**
 * @brief getAllUser
 */
bool
TestTable::getAllUser(TableItem& resultItem,
                      ErrorContainer& error,
                      const bool showHiddenValues,
                      const uint64_t positionOffset,
                      const uint64_t numberOfRows)
{
    return getAllFromDb(resultItem, error, showHiddenValues, positionOffset, numberOfRows);
}

/**
 * @brief deleteUser
 */
ReturnStatus
TestTable::deleteUser(const std::string& userID, ErrorContainer& error)
{
    std::vector<RequestCondition> conditions;
    conditions.emplace_back("name", userID);
    return deleteFromDb(conditions, error);
}

}  // namespace Hanami
