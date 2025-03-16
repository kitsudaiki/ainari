/**
 * @file        audit_log_table_test.cpp
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

#include "audit_log_table_test.h"

#include <hanami_common/functions/file_functions.h>
#include <hanami_common/logger.h>
#include <hanami_common/uuid.h>
#include <hanami_database/sql_database.h>
#include <src/database/audit_log_table.h>

AuditLogTable_Test::AuditLogTable_Test() : Hanami::CompareTestHelper("AuditLogTable_Test")
{
    addAuditLog_test();
    getAllAuditLog_test();
}

/**
 * @brief initTest
 */
void
AuditLogTable_Test::initTest()
{
    Hanami::ErrorContainer error;
    bool success = false;

    if (Hanami::deleteFileOrDir(m_databasePath, error) == false) {
        LOG_DEBUG("No test-database to delete");
    }

    // initalize database
    m_database = Hanami::SqlDatabase::getInstance();
    if (m_database->initDatabase(m_databasePath, error) == false) {
        error.addMessage("Failed to initialize sql-database.");
        LOG_ERROR(error);
        assert(false);
    }
}

/**
 * @brief cleanupTest
 */
void
AuditLogTable_Test::cleanupTest()
{
    Hanami::ErrorContainer error;

    if (m_database->closeDatabase() == false) {
        error.addMessage("Failed to close test-database.");
        LOG_ERROR(error);
        assert(false);
    }

    if (Hanami::deleteFileOrDir(m_databasePath, error) == false) {
        error.addMessage("Failed to delete database-test-file.");
        LOG_ERROR(error);
        assert(false);
    }
}

/**
 * @brief addAuditLog_test
 */
void
AuditLogTable_Test::addAuditLog_test()
{
    initTest();

    Hanami::ErrorContainer error;

    AuditLogTable* auditLogTable = AuditLogTable::getInstance();
    TEST_EQUAL(auditLogTable->initTable(error), true);
    TEST_EQUAL(auditLogTable->addAuditLogEntry("today", "test-user", "test-endpoint", "get", error),
               true);
    TEST_EQUAL(
        auditLogTable->addAuditLogEntry("yesterday", "test-user2", "test-endpoint2", "post", error),
        true);

    cleanupTest();
}

/**
 * @brief getAllAuditLog_test
 */
void
AuditLogTable_Test::getAllAuditLog_test()
{
    initTest();

    Hanami::ErrorContainer error;
    AuditLogTable* auditLogTable = AuditLogTable::getInstance();

    auditLogTable->initTable(error);
    auditLogTable->addAuditLogEntry("today", "test-user", "test-endpoint", "get", error);
    auditLogTable->addAuditLogEntry("yesterday", "test-user2", "test-endpoint2", "post", error);

    Hanami::TableItem result;
    TEST_EQUAL(auditLogTable->getAllAuditLogEntries(result, "test-user", 0, error), true);
    TEST_EQUAL(result.getNumberOfRows(), 1);
    TEST_EQUAL(result.getNumberOfColums(), auditLogTable->getNumberOfColumns());

    TEST_EQUAL(result.getCell(4, 0), "test-user");
    TEST_EQUAL(result.getCell(5, 0), "test-endpoint");
    TEST_EQUAL(result.getCell(6, 0), "get");

    cleanupTest();
}
