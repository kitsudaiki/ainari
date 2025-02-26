/**
 * @file        sql_table_test.cpp
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

#include "sql_table_test.h"

#include <hanami_database/sql_database.h>
#include <hanami_database/sql_table.h>
#include <test_table.h>

namespace Hanami
{

SqlTable_Test::SqlTable_Test() : Hanami::CompareTestHelper("SqlTable_Test")
{
    initTest();

    initDatabase_test();
    initTable_test();

    create_test();
    get_test();
    getAll_test();
    update_test();
    delete_test();
    getNumberOfRows_test();

    delete m_table;
}

/**
 * @brief initTest
 */
void
SqlTable_Test::initTest()
{
    m_filePath = "/tmp/testdb.db";
    deleteFile();
}

/**
 * @brief initDB_test
 */
void
SqlTable_Test::initDatabase_test()
{
    Hanami::ErrorContainer error;
    m_db = SqlDatabase::getInstance();
    TEST_EQUAL(m_db->initDatabase(m_filePath, error), true);
}

/**
 * @brief initTable_test
 */
void
SqlTable_Test::initTable_test()
{
    m_table = new TestTable(m_db);
    ErrorContainer error;
    TEST_EQUAL(m_table->initTable(error), true);
}

/**
 * @brief create_test
 */
void
SqlTable_Test::create_test()
{
    ErrorContainer error;

    json testData;
    testData["name"] = m_name1;
    testData["pw_hash"] = "secret";
    testData["is_admin"] = true;

    TEST_EQUAL(m_table->addUser(testData, error), true);

    json testData2;
    testData2["name"] = m_name2;
    testData2["pw_hash"] = "secret2";
    testData2["is_admin"] = false;

    m_table->addUser(testData2, error);
}

/**
 * @brief get_test
 */
void
SqlTable_Test::get_test()
{
    json resultItem;
    TableItem resultTable;
    ErrorContainer error;

    TEST_EQUAL(m_table->getUser(resultItem, m_name1, false, error), OK);
    resultItem.erase("created_at");
    TEST_EQUAL(resultItem.dump(), std::string("{\"is_admin\":true,\"name\":\"user0815\"}"));

    TEST_EQUAL(m_table->getUser(resultTable, m_name1, error), OK);
    resultTable.deleteColumn("created_at");
    std::string compare
        = "+----------+----------+\n"
          "| name     | is_admin |\n"
          "+==========+==========+\n"
          "| user0815 | true     |\n"
          "+----------+----------+\n";
    TEST_EQUAL(resultTable.toString(), compare);
}

/**
 * @brief getAll_test
 */
void
SqlTable_Test::getAll_test()
{
    TableItem result;
    ErrorContainer error;

    TEST_EQUAL(m_table->getAllUser(result, error, false), true);
    result.deleteColumn("created_at");

    TEST_EQUAL(result.getNumberOfRows(), 2);
    TEST_EQUAL(result.getNumberOfColums(), 2);

    result.clearTable();
    TEST_EQUAL(m_table->getAllUser(result, error, true), true);
    result.deleteColumn("created_at");

    TEST_EQUAL(result.getNumberOfRows(), 2);
    TEST_EQUAL(result.getNumberOfColums(), 3);

    // test with limitation
    result.clearTable();
    TEST_EQUAL(m_table->getAllUser(result, error, true, 1, 10), true);
    result.deleteColumn("created_at");

    TEST_EQUAL(result.getNumberOfRows(), 1);
    TEST_EQUAL(result.getNumberOfColums(), 3);
    TEST_EQUAL(result.getCell(0, 0), m_name2);
}

/**
 * @brief update_test
 */
void
SqlTable_Test::update_test()
{
    ErrorContainer error;

    json updateDate;
    updateDate["pw_hash"] = "secret2";
    updateDate["is_admin"] = false;
    TEST_EQUAL(m_table->updateUser(m_name1, updateDate, error), OK);

    json resultItem;
    TableItem resultTable;

    TEST_EQUAL(m_table->getUser(resultItem, m_name1, true, error), OK);
    resultItem.erase("created_at");
    TEST_EQUAL(resultItem.dump(),
               std::string("{\"is_admin\":false,\"name\":\"user0815\",\"pw_hash\":\"secret2\"}"));
}

/**
 * @brief delete_test
 */
void
SqlTable_Test::delete_test()
{
    ErrorContainer error;

    TEST_EQUAL(m_table->deleteUser(m_name1, error), OK);
    TableItem result1;
    m_table->getAllUser(result1, error, false);
    result1.deleteColumn("created_at");

    TEST_EQUAL(result1.getNumberOfRows(), 1);
    TEST_EQUAL(result1.getNumberOfColums(), 2);

    TEST_EQUAL(m_table->deleteUser(m_name2, error), OK);
    TableItem result2;
    m_table->getAllUser(result2, error, false);
    result2.deleteColumn("created_at");

    TEST_EQUAL(result2.getNumberOfRows(), 0);
    TEST_EQUAL(result2.getNumberOfColums(), 2);

    TableItem result3;
    m_table->getAllUser(result3, error, OK);
    result3.deleteColumn("created_at");

    TEST_EQUAL(result3.getNumberOfRows(), 0);
    TEST_EQUAL(result3.getNumberOfColums(), 2);
}

/**
 * @brief getNumberOfRows_test
 */
void
SqlTable_Test::getNumberOfRows_test()
{
    ErrorContainer error;

    TEST_EQUAL(m_table->getNumberOfUsers(error), 0);

    json testData;
    testData["name"] = m_name1;
    testData["pw_hash"] = "secret";
    testData["is_admin"] = true;
    m_table->addUser(testData, error);

    TEST_EQUAL(m_table->getNumberOfUsers(error), 1);

    json testData2;
    testData2["name"] = m_name2;
    testData2["pw_hash"] = "secret2";
    testData2["is_admin"] = false;
    m_table->addUser(testData2, error);

    TEST_EQUAL(m_table->getNumberOfUsers(error), 2);
}

/**
 * common usage to delete test-file
 */
void
SqlTable_Test::deleteFile()
{
    std::filesystem::path rootPathObj(m_filePath);
    if (std::filesystem::exists(rootPathObj)) {
        std::filesystem::remove(rootPathObj);
    }
}

}  // namespace Hanami
