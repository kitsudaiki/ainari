/**
 * @file        audit_log_table.cpp
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

#include <database/audit_log_table.h>
#include <hanami_common/functions/string_functions.h>
#include <hanami_common/items/table_item.h>
#include <hanami_database/sql_database.h>

AuditLogTable* AuditLogTable::instance = nullptr;

/**
 * @brief constructor
 *
 * @param db pointer to database
 */
AuditLogTable::AuditLogTable() : HanamiSqlLogTable(Hanami::SqlDatabase::getInstance())
{
    m_tableName = "audit_log";

    registerColumn("user_id", STRING_TYPE).setMaxLength(256);

    registerColumn("endpoint", STRING_TYPE).setMaxLength(1024);

    registerColumn("request_type", STRING_TYPE).setMaxLength(16);
}

/**
 * @brief destructor
 */
AuditLogTable::~AuditLogTable() {}

/**
 * @brief add new audit-log-entry into the database
 *
 * @param timestamp UTC-timestamp of the request as string
 * @param userId id of the user, who made the request
 * @param endpoint requested endpoint
 * @param requestType HTTP-type of the request
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
AuditLogTable::addAuditLogEntry(const std::string& timestamp,
                                const std::string& userId,
                                const std::string& endpoint,
                                const std::string& requestType,
                                Hanami::ErrorContainer& error)
{
    json data;
    data["user_id"] = userId;
    data["endpoint"] = endpoint;
    data["request_type"] = requestType;

    if (insertToDb(data, userId, error) == false) {
        error.addMessage("Failed to add audit-log-entry to database");
        return false;
    }

    return true;
}

/**
 * @brief get all audit-log-entries from the database
 *
 * @param result reference for the result-output
 * @param userId id of the user, whos logs are requested
 * @param page a page has 100 entries so (page * 100)
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
AuditLogTable::getAllAuditLogEntries(Hanami::TableItem& result,
                                     const std::string& userId,
                                     const uint64_t page,
                                     Hanami::ErrorContainer& error)
{
    if (getPageFromDb(result, userId, page, error) != OK) {
        error.addMessage("Failed to get all audit-log-entries from database");
        return false;
    }

    return true;
}
