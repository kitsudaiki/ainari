/**
 * @file        project_table_test.cpp
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

#include "projects_table_test.h"

#include <hanami_common/functions/file_functions.h>
#include <hanami_common/logger.h>
#include <hanami_common/uuid.h>
#include <hanami_database/sql_database.h>
#include <src/database/projects_table.h>

ProjectTable_Test::ProjectTable_Test() : Hanami::CompareTestHelper("ProjectTable_Test")
{
    m_testId = "test-id";
    m_testName = "test-project";

    addProject_test();
    getProject_test();
    getAllProject_test();
    deleteProject_test();
}

/**
 * @brief initTest
 */
void
ProjectTable_Test::initTest()
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
 * @brief createTestDb
 */
void
ProjectTable_Test::createTestDb()
{
    ProjectTable::ProjectDbEntry projectData;
    Hanami::ErrorContainer error;

    Hanami::UserContext context;
    context.userId = "test_user";

    projectData.id = m_testId;
    projectData.name = m_testName;

    ProjectTable* projectTable = ProjectTable::getInstance();
    projectTable->initTable(error);
    projectTable->addProject(projectData, context, error);

    projectData.id = "test-id2";
    projectData.name = "test-project2";
    projectTable->addProject(projectData, context, error);
}

/**
 * @brief cleanupTest
 */
void
ProjectTable_Test::cleanupTest()
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
 * @brief addProject_test
 */
void
ProjectTable_Test::addProject_test()
{
    initTest();

    ProjectTable::ProjectDbEntry projectData;
    Hanami::ErrorContainer error;

    Hanami::UserContext context;
    context.userId = "test_user";

    projectData.id = m_testId;
    projectData.name = m_testName;

    ProjectTable* projectTable = ProjectTable::getInstance();
    TEST_EQUAL(projectTable->initTable(error), true);
    TEST_EQUAL(projectTable->addProject(projectData, context, error), OK);
    TEST_EQUAL(projectTable->addProject(projectData, context, error), INVALID_INPUT);

    cleanupTest();
}

/**
 * @brief getProject_test
 */
void
ProjectTable_Test::getProject_test()
{
    initTest();

    Hanami::ErrorContainer error;
    ProjectTable* projectTable = ProjectTable::getInstance();

    createTestDb();

    // positive test
    ProjectTable::ProjectDbEntry result;
    TEST_EQUAL(projectTable->getProject(result, m_testId, error), OK);
    TEST_EQUAL(result.id, m_testId);
    TEST_EQUAL(result.name, m_testName);

    // negative test
    TEST_EQUAL(projectTable->getProject(result, "fail", error), INVALID_INPUT);

    cleanupTest();
}

/**
 * @brief getAllProject_test
 */
void
ProjectTable_Test::getAllProject_test()
{
    initTest();

    Hanami::ErrorContainer error;
    ProjectTable* projectTable = ProjectTable::getInstance();

    createTestDb();

    Hanami::TableItem result;
    TEST_EQUAL(projectTable->getAllProjects(result, error), true);
    TEST_EQUAL(result.getNumberOfRows(), 2);
    TEST_EQUAL(result.getNumberOfColums(), projectTable->getNumberOfColumns());

    cleanupTest();
}

/**
 * @brief deleteProject_test
 */
void
ProjectTable_Test::deleteProject_test()
{
    initTest();

    Hanami::UserContext context;
    context.userId = "test_user";

    Hanami::ErrorContainer error;
    ProjectTable* projectTable = ProjectTable::getInstance();

    createTestDb();

    ProjectTable::ProjectDbEntry result;
    TEST_EQUAL(projectTable->deleteProject(m_testId, context, error), OK);
    TEST_EQUAL(projectTable->deleteProject(m_testId, context, error), INVALID_INPUT);
    TEST_EQUAL(projectTable->getProject(result, m_testId, error), INVALID_INPUT);

    Hanami::TableItem result2;
    TEST_EQUAL(projectTable->getAllProjects(result2, error), true);
    TEST_EQUAL(result2.getNumberOfRows(), 1);
    TEST_EQUAL(result2.getNumberOfColums(), projectTable->getNumberOfColumns());

    cleanupTest();
}
