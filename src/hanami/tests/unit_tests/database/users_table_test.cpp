/**
 * @file        user_table_test.cpp
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

#include "users_table_test.h"

#include <hanami_common/functions/file_functions.h>
#include <hanami_common/logger.h>
#include <hanami_common/uuid.h>
#include <hanami_database/sql_database.h>
#include <src/database/users_table.h>

UserTable_Test::UserTable_Test() : Hanami::CompareTestHelper("UserTable_Test")
{
    m_testId = "test-id";
    m_testName = "test-user";

    addUser_test();
    getUser_test();
    getAllUser_test();
    deleteUser_test();
    updateProjectsOfUser_test();
}

/**
 * @brief initTest
 */
void
UserTable_Test::initTest()
{
    Hanami::ErrorContainer error;
    bool success = false;

    m_userContext.projectId = "test-project";
    m_userContext.userId = "test-user";

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
UserTable_Test::createTestDb()
{
    UserTable::UserDbEntry userData;
    UserTable::UserProjectDbEntry userProjectData;
    Hanami::ErrorContainer error;

    Hanami::UserContext context;
    context.userId = "test_user";

    userProjectData.projectId = "test-project";
    userProjectData.role = "admin";
    userProjectData.isProjectAdmin = true;

    userData.id = m_testId;
    userData.name = m_testName;
    userData.salt = "asdf";
    userData.pwHash = "test-hash";
    userData.projects.push_back(userProjectData);
    userData.isAdmin = true;

    UserTable* userTable = UserTable::getInstance();
    userTable->initTable(error);
    userTable->addUser(userData, context, error);

    userData.id = "test-id2";
    userData.name = "test-user2";
    userTable->addUser(userData, context, error);
}

/**
 * @brief cleanupTest
 */
void
UserTable_Test::cleanupTest()
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
 * @brief addUser_test
 */
void
UserTable_Test::addUser_test()
{
    initTest();

    UserTable::UserDbEntry userData;
    UserTable::UserProjectDbEntry userProjectData;
    Hanami::ErrorContainer error;

    Hanami::UserContext context;
    context.userId = "test_user";

    userProjectData.projectId = "test-project";
    userProjectData.role = "admin";
    userProjectData.isProjectAdmin = true;

    userData.id = m_testId;
    userData.name = m_testName;
    userData.salt = "asdf";
    userData.pwHash = "test-hash";
    userData.projects.push_back(userProjectData);
    userData.isAdmin = true;

    UserTable* userTable = UserTable::getInstance();
    TEST_EQUAL(userTable->initTable(error), true);
    TEST_EQUAL(userTable->addUser(userData, context, error), OK);
    TEST_EQUAL(userTable->addUser(userData, context, error), INVALID_INPUT);

    cleanupTest();
}

/**
 * @brief getUser_test
 */
void
UserTable_Test::getUser_test()
{
    initTest();

    Hanami::ErrorContainer error;
    UserTable* userTable = UserTable::getInstance();

    Hanami::UserContext context;
    context.userId = "test_user";

    createTestDb();

    // positive test
    UserTable::UserDbEntry result;
    TEST_EQUAL(userTable->getUser(result, m_testId, error), OK);
    TEST_EQUAL(result.id, m_testId);
    TEST_EQUAL(result.name, m_testName);
    TEST_EQUAL(result.salt, "asdf");
    TEST_EQUAL(result.pwHash, "test-hash");
    TEST_EQUAL(result.projects.size(), 1);
    TEST_EQUAL(result.projects.at(0).projectId, "test-project");
    TEST_EQUAL(result.projects.at(0).role, "admin");
    TEST_EQUAL(result.projects.at(0).isProjectAdmin, true);
    TEST_EQUAL(result.isAdmin, true);

    // negative test
    TEST_EQUAL(userTable->getUser(result, "fail", error), INVALID_INPUT);

    cleanupTest();
}

/**
 * @brief getAllUser_test
 */
void
UserTable_Test::getAllUser_test()
{
    initTest();

    Hanami::ErrorContainer error;
    UserTable* userTable = UserTable::getInstance();

    createTestDb();

    Hanami::TableItem result;
    TEST_EQUAL(userTable->getAllUser(result, error), true);
    TEST_EQUAL(result.getNumberOfRows(), 2);
    TEST_EQUAL(result.getNumberOfColums(), userTable->getNumberOfColumns() - 2);

    cleanupTest();
}

/**
 * @brief deleteUser_test
 */
void
UserTable_Test::deleteUser_test()
{
    initTest();

    Hanami::UserContext context;
    context.userId = "test_user";

    Hanami::ErrorContainer error;
    UserTable* userTable = UserTable::getInstance();

    createTestDb();

    UserTable::UserDbEntry result;
    TEST_EQUAL(userTable->deleteUser(m_testId, context, error), OK);
    TEST_EQUAL(userTable->deleteUser(m_testId, context, error), INVALID_INPUT);
    TEST_EQUAL(userTable->getUser(result, m_testId, error), INVALID_INPUT);

    Hanami::TableItem result2;
    TEST_EQUAL(userTable->getAllUser(result2, error), true);
    TEST_EQUAL(result2.getNumberOfRows(), 1);
    TEST_EQUAL(result2.getNumberOfColums(), userTable->getNumberOfColumns() - 2);

    cleanupTest();
}

/**
 * @brief updateProjectsOfUser_test
 */
void
UserTable_Test::updateProjectsOfUser_test()
{
    initTest();

    Hanami::ErrorContainer error;
    UserTable* userTable = UserTable::getInstance();

    createTestDb();

    Hanami::UserContext context;
    context.userId = "test_user";

    UserTable::UserDbEntry result;
    userTable->getUser(result, m_testId, error);

    UserTable::UserProjectDbEntry newProject;
    newProject.projectId = "test-project2";
    newProject.role = "user";
    newProject.isProjectAdmin = true;
    result.projects.push_back(newProject);

    TEST_EQUAL(userTable->updateProjectsOfUser(m_testId, result.projects, context, error), OK);

    result = UserTable::UserDbEntry();
    userTable->getUser(result, m_testId, error);
    TEST_EQUAL(result.id, m_testId);
    TEST_EQUAL(result.name, m_testName);
    TEST_EQUAL(result.salt, "asdf");
    TEST_EQUAL(result.pwHash, "test-hash");
    TEST_EQUAL(result.projects.size(), 2);
    TEST_EQUAL(result.projects.at(0).projectId, "test-project");
    TEST_EQUAL(result.projects.at(0).role, "admin");
    TEST_EQUAL(result.projects.at(0).isProjectAdmin, true);
    TEST_EQUAL(result.projects.at(1).projectId, "test-project2");
    TEST_EQUAL(result.projects.at(1).role, "user");
    TEST_EQUAL(result.projects.at(1).isProjectAdmin, true);
    TEST_EQUAL(result.isAdmin, true);

    cleanupTest();
}
