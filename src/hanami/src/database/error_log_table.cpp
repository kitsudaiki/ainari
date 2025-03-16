/**
 * @file        error_log_table.cpp
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

#include <database/error_log_table.h>
#include <hanami_common/functions/string_functions.h>
#include <hanami_common/functions/time_functions.h>
#include <hanami_common/items/table_item.h>
#include <hanami_database/sql_database.h>

ErrorLogTable* ErrorLogTable::instance = nullptr;

/**
 * @brief constructor
 *
 * @param db pointer to database
 */
ErrorLogTable::ErrorLogTable() : HanamiSqlLogTable(Hanami::SqlDatabase::getInstance())
{
    m_tableName = "error_log";

    registerColumn("user_id", STRING_TYPE).setMaxLength(256);

    registerColumn("component", STRING_TYPE).setMaxLength(128);

    registerColumn("context", STRING_TYPE);

    registerColumn("input_values", STRING_TYPE);

    registerColumn("message", BASE64_TYPE);
}

/**
 * @brief destructor
 */
ErrorLogTable::~ErrorLogTable() {}

/**
 * @brief add new error-log-entry into the database
 *
 * @param userid id of the user, who had the error
 * @param component component, where the error appeared
 * @param context
 * @param values
 * @param message
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
ErrorLogTable::addErrorLogEntry(const std::string& userid,
                                const std::string& component,
                                const std::string& context,
                                const std::string& values,
                                const std::string& message,
                                Hanami::ErrorContainer& error)
{
    json data;
    data["user_id"] = userid;
    data["component"] = component;
    data["context"] = context;
    data["input_values"] = values;
    data["message"] = message;

    if (insertToDb(data, userid, error) == false) {
        error.addMessage("Failed to add error-log-entry to database");
        return false;
    }

    return true;
}

/**
 * @brief get all error-log-entries from the database
 *
 * @param result reference for the result-output
 * @param userId id of the user, whos logs are requested
 * @param page a page has 100 entries so (page * 100)
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
ErrorLogTable::getAllErrorLogEntries(Hanami::TableItem& result,
                                     const std::string& userId,
                                     const uint64_t page,
                                     Hanami::ErrorContainer& error)
{
    if (getPageFromDb(result, userId, page, error) != OK) {
        error.addMessage("Failed to get all error-log-entries from database");
        return false;
    }

    return true;
}
