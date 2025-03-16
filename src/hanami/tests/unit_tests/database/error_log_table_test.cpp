/**
 * @file        error_log_table_test.cpp
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

#include "error_log_table_test.h"

#include <hanami_common/functions/file_functions.h>
#include <hanami_common/logger.h>
#include <hanami_common/uuid.h>
#include <hanami_database/sql_database.h>
#include <src/database/error_log_table.h>

ErrorLogTable_Test::ErrorLogTable_Test() : Hanami::CompareTestHelper("ErrorLogTable_Test")
{
    addErrorLog_test();
    getAllErrorLog_test();
}

/**
 * @brief initTest
 */
void
ErrorLogTable_Test::initTest()
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
ErrorLogTable_Test::cleanupTest()
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
 * @brief addErrorLog_test
 */
void
ErrorLogTable_Test::addErrorLog_test()
{
    initTest();

    Hanami::ErrorContainer error;

    ErrorLogTable* errorLogTable = ErrorLogTable::getInstance();
    TEST_EQUAL(errorLogTable->initTable(error), true);
    TEST_EQUAL(
        errorLogTable->addErrorLogEntry(
            "test-user", "hanami", "example-context", "values", "this is a test-message", error),
        true);
    TEST_EQUAL(
        errorLogTable->addErrorLogEntry(
            "test-user2", "hanami", "example-context", "values", "this is a test-message", error),
        true);

    cleanupTest();
}

/**
 * @brief getAllErrorLog_test
 */
void
ErrorLogTable_Test::getAllErrorLog_test()
{
    initTest();

    Hanami::ErrorContainer error;
    ErrorLogTable* errorLogTable = ErrorLogTable::getInstance();

    errorLogTable->initTable(error);
    errorLogTable->addErrorLogEntry(
        "test-user", "hanami", "example-context", "values", "this is a test-message", error);
    errorLogTable->addErrorLogEntry(
        "test-user2", "hanami", "example-context", "values", "this is a test-message", error);

    Hanami::TableItem result;
    TEST_EQUAL(errorLogTable->getAllErrorLogEntries(result, "test-user", 0, error), true);
    TEST_EQUAL(result.getNumberOfRows(), 1);
    TEST_EQUAL(result.getNumberOfColums(), errorLogTable->getNumberOfColumns());

    TEST_EQUAL(result.getCell(4, 0), "test-user");
    TEST_EQUAL(result.getCell(5, 0), "hanami");
    TEST_EQUAL(result.getCell(6, 0), "example-context");
    TEST_EQUAL(result.getCell(7, 0), "values");
    TEST_EQUAL(result.getCell(8, 0), "this is a test-message");

    cleanupTest();
}
